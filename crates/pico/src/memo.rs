use pico_core::{
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, NodeKind},
    params::ParamId,
};

pub enum DidRecalculate {
    ReusedMemoizedValue,
    Recalculated,
}

/// Memo is the workhorse function of pico. Given a [`DerivedNodeId`], which
/// uniquely identifies the function being called along with the parameters
/// passed, it:
///
/// - checks if we have an available [`DerivedNode`] (i.e. if this function
///   has previously been invoked, and the [`DerivedNode`] has not been
///   garbage collected).
///   - if so, check if the value from the previous invocation can be reused.
///     - if so, reuse the value from the previous invocation
///     - if not, re-invoke the function. If the value is the same, backdate
///       the `DerivedNode`.
///   - if not, execute this function. Create and store a [`DerivedNode`].
///
/// In all cases, we set the [`DerivedNode`]'s `verified_at` to the current
/// epoch.
///
/// **Checking whether we can reuse a previous invocation**:
///   - This is done by checking whether
///     [any dependency has changed](`any_dependency_changed`) since this
///     [`DerivedNode`] was last verified.
///   - Memoized functions are assumed to be pure functions of their params
///     and the dependencies that they read. So, if no dependency has
///     changed, we know we can reuse the value from the previous invocation.
///
/// **Backdating**:
///   - when we re-invoke a function (whose dependencies have changed), if
///     the new value is identical, we backdate the [`DerivedNode`]'s
///     `time_updated` field to match the previous invocation's `time_updated`.
///   - Consider `type_check -> ast -> source`. If `source` changes, but not
///     in a way that affects `ast`, we can re-use the value returned by
///     `type_check`, because its dependencies have not changed. However,
///     this relies on backdating, i.e. `ast`'s `time_updated` field must not
///     increase, even though we re-invoked it.
///
/// After this function is called, we guarantee that a [`DerivedNode`]
/// (with a value identical to what we would get if we actually invoked the
/// function) is present in the [`Storage`].
pub fn memo<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    inner_fn: fn(&Db, ParamId) -> Box<dyn DynEq>,
) -> DidRecalculate {
    let (time_updated, did_recalculate) =
        if let Some(derived_node) = db.storage().get_derived_node(derived_node_id) {
            if any_dependency_changed(db, derived_node) {
                update_derived_node(db, derived_node_id, derived_node.value.as_ref(), inner_fn)
            } else {
                db.storage().verify_derived_node(derived_node_id);
                (db.current_epoch(), DidRecalculate::ReusedMemoizedValue)
            }
        } else {
            create_derived_node(db, derived_node_id, inner_fn)
        };
    register_dependency_in_parent_memoized_fn(db, NodeKind::Derived(derived_node_id), time_updated);
    did_recalculate
}

pub fn register_dependency_in_parent_memoized_fn<Db: Database>(
    db: &Db,
    node: NodeKind,
    time_updated: Epoch,
) {
    db.storage().dependency_stack.push_checked(|| {
        (
            time_updated,
            Dependency {
                node_to: node,
                time_verified_or_updated: db.current_epoch(),
            },
        )
    });
}

fn create_derived_node<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    inner_fn: fn(&Db, ParamId) -> Box<dyn DynEq>,
) -> (Epoch, DidRecalculate) {
    let (value, dependencies, time_updated) =
        call_inner_fn_and_collect_dependencies(db, derived_node_id.param_id, inner_fn);
    db.storage().insert_derived_node(
        derived_node_id,
        DerivedNode {
            dependencies,
            inner_fn,
            value,
        },
    );
    db.storage()
        .insert_derived_node_rev(derived_node_id, time_updated, db.current_epoch());
    (time_updated, DidRecalculate::Recalculated)
}

fn update_derived_node<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    prev_value: &dyn DynEq,
    inner_fn: fn(&Db, ParamId) -> Box<dyn DynEq>,
) -> (Epoch, DidRecalculate) {
    let mut did_recalculate = DidRecalculate::ReusedMemoizedValue;
    let (value, dependencies, time_updated) =
        call_inner_fn_and_collect_dependencies(db, derived_node_id.param_id, inner_fn);
    if *prev_value != *value {
        did_recalculate = DidRecalculate::Recalculated;
        db.storage()
            .set_derive_node_time_updated(derived_node_id, time_updated);
    }
    db.storage().verify_derived_node(derived_node_id);
    db.storage().insert_derived_node(
        derived_node_id,
        DerivedNode {
            dependencies,
            inner_fn,
            value,
        },
    );
    (time_updated, did_recalculate)
}

fn any_dependency_changed<Db: Database>(db: &Db, derived_node: &DerivedNode<Db>) -> bool {
    derived_node
        .dependencies
        .iter()
        .filter_map(|dep| {
            if dep.time_verified_or_updated != db.current_epoch() {
                Some(*dep)
            } else {
                None
            }
        })
        .any(|dependency| match dependency.node_to {
            NodeKind::Source(key) => {
                source_node_changed_since(db, key, dependency.time_verified_or_updated)
            }
            NodeKind::Derived(dep_node_id) => {
                derived_node_changed_since(db, dep_node_id, dependency.time_verified_or_updated)
            }
        })
}

fn source_node_changed_since<Db: Database>(db: &Db, key: Key, since: Epoch) -> bool {
    match db.storage().source_nodes.get(&key) {
        Some(source) => source.time_updated > since,
        None => panic!("Source node not found. This indicates a bug in Pico."),
    }
}

fn derived_node_changed_since<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    since: Epoch,
) -> bool {
    if !db.storage().contains_param(derived_node_id.param_id) {
        return true;
    }
    let inner_fn = if let Some(derived_node) = db.storage().get_derived_node(derived_node_id) {
        if let Some(rev) = db.storage().get_derived_node_rev(derived_node_id) {
            if rev.time_updated > since {
                return true;
            }
        } else {
            return true;
        }
        derived_node.inner_fn
    } else {
        return true;
    };
    let did_recalculate = memo(db, derived_node_id, inner_fn);
    matches!(did_recalculate, DidRecalculate::Recalculated)
}

fn call_inner_fn_and_collect_dependencies<Db: Database>(
    db: &Db,
    param_id: ParamId,
    inner_fn: fn(&Db, ParamId) -> Box<dyn DynEq>,
) -> (
    Box<dyn DynEq>,  /* value */
    Vec<Dependency>, /* dependencies */
    Epoch,           /* time_updated */
) {
    let (value, registered_dependencies) = with_dependency_tracking(db, param_id, inner_fn);
    let (dependencies, time_updated) = registered_dependencies.into_iter().fold(
        (vec![], Epoch::new()),
        |(mut deps, mut max_time_updated), (time_updated, dep)| {
            deps.push(dep);
            max_time_updated = std::cmp::max(max_time_updated, time_updated);
            (deps, max_time_updated)
        },
    );
    (value, dependencies, time_updated)
}

fn with_dependency_tracking<Db>(
    db: &Db,
    param_id: ParamId,
    inner_fn: fn(&Db, ParamId) -> Box<dyn DynEq>,
) -> (Box<dyn DynEq>, Vec<(Epoch, Dependency)>)
where
    Db: Database,
{
    db.storage().dependency_stack.enter();
    let value = inner_fn(db, param_id);
    let registered_dependencies = db.storage().dependency_stack.leave();
    (value, registered_dependencies)
}
