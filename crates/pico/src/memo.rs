use crate::{
    database::Database,
    dependency::{NodeKind, TrackedDependencies},
    derived_node::{DerivedNode, DerivedNodeId},
    dyn_eq::DynEq,
    epoch::Epoch,
    u64_types::{Key, ParamId},
    InnerFn,
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
/// function) is present in the [`Database`].
pub fn memo(db: &Database, derived_node_id: DerivedNodeId, inner_fn: InnerFn) -> DidRecalculate {
    eprintln!("memo 1");
    let (time_updated, did_recalculate) =
        if let Some(mut derived_node) = db.get_derived_node_mut(derived_node_id) {
            eprintln!("memo 2");
            derived_node.time_verified = db.current_epoch;
            // db.verify_derived_node(derived_node_id);
            eprintln!("memo 3");
            if any_dependency_changed(db, &derived_node) {
                eprintln!("memo 4");
                update_derived_node(db, derived_node_id, &mut derived_node, inner_fn)
            } else {
                (db.current_epoch, DidRecalculate::ReusedMemoizedValue)
            }
        } else {
            eprintln!("memo 5");
            create_derived_node(db, derived_node_id, inner_fn)
        };
    eprintln!("memo 7");
    db.register_dependency_in_parent_memoized_fn(NodeKind::Derived(derived_node_id), time_updated);
    eprintln!("memo 8");
    did_recalculate
}

fn create_derived_node(
    db: &Database,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn,
) -> (Epoch, DidRecalculate) {
    eprintln!("create 1");
    let (value, tracked_dependencies) =
        with_dependency_tracking(db, derived_node_id.param_id, inner_fn);
    eprintln!("create 2");
    db.insert_derived_node(
        derived_node_id,
        DerivedNode {
            dependencies: tracked_dependencies.dependencies,
            inner_fn,
            value,
            time_updated: tracked_dependencies.max_time_updated,
            time_verified: db.current_epoch,
        },
    );
    eprintln!("create 3");
    (
        tracked_dependencies.max_time_updated,
        DidRecalculate::Recalculated,
    )
}

fn update_derived_node(
    db: &Database,
    derived_node_id: DerivedNodeId,
    derived_node: &mut DerivedNode,
    inner_fn: InnerFn,
) -> (Epoch, DidRecalculate) {
    eprintln!("update 1");
    let (new_value, tracked_dependencies) =
        with_dependency_tracking(db, derived_node_id.param_id, inner_fn);
    eprintln!("update 2");
    let did_recalculate = if *derived_node.value != *new_value {
        eprintln!("update 3");
        derived_node.time_updated = tracked_dependencies.max_time_updated;
        DidRecalculate::Recalculated
    } else {
        DidRecalculate::ReusedMemoizedValue
    };
    eprintln!("update 4");
    *derived_node = DerivedNode {
        dependencies: tracked_dependencies.dependencies,
        inner_fn,
        value: new_value,
        time_updated: tracked_dependencies.max_time_updated,
        time_verified: db.current_epoch,
    };
    eprintln!("update 5");
    (tracked_dependencies.max_time_updated, did_recalculate)
}

fn any_dependency_changed(db: &Database, derived_node: &DerivedNode) -> bool {
    derived_node
        .dependencies
        .iter()
        .filter(|dep| dep.time_verified_or_updated != db.current_epoch)
        .any(|dependency| match dependency.node_to {
            NodeKind::Source(key) => {
                eprintln!("any dep getting source {key:?}");
                source_node_changed_since(db, key, dependency.time_verified_or_updated)
            }
            NodeKind::Derived(dep_node_id) => {
                eprintln!("any dep getting derived {dep_node_id:?}");
                derived_node_changed_since(db, dep_node_id, dependency.time_verified_or_updated)
            }
        })
}

fn source_node_changed_since(db: &Database, key: Key, since: Epoch) -> bool {
    match db.source_nodes.get(&key) {
        Some(source) => source.time_updated > since,
        None => panic!("Source node not found. This may occur if `SourceId` is used after the source node has been removed."),
    }
}

fn derived_node_changed_since(db: &Database, derived_node_id: DerivedNodeId, since: Epoch) -> bool {
    if !db.contains_param(derived_node_id.param_id) {
        return true;
    }
    let inner_fn = if let Some(derived_node) = db.get_derived_node(derived_node_id) {
        if derived_node.time_updated > since {
            return true;
        }
        derived_node.inner_fn
    } else {
        return true;
    };
    let did_recalculate = memo(db, derived_node_id, inner_fn);
    matches!(did_recalculate, DidRecalculate::Recalculated)
}

fn with_dependency_tracking(
    db: &Database,
    param_id: ParamId,
    inner_fn: InnerFn,
) -> (Box<dyn DynEq>, TrackedDependencies) {
    let guard = db.dependency_stack.enter();
    let value = inner_fn.0(db, param_id);
    eprintln!("wdt 1");
    let tracked_dependencies = guard.release();
    eprintln!("wdt released");
    (value, tracked_dependencies)
}
