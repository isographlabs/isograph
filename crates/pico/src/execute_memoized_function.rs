use dashmap::Entry;
use tracing::{Level, debug_span, event, trace_span};

use crate::{
    Database, InnerFn,
    dependency::{NodeKind, TrackedDependencies},
    derived_node::{DerivedNode, DerivedNodeId},
    dyn_eq::DynEq,
    epoch::Epoch,
    intern::Key,
};

pub enum DidRecalculate {
    ReusedMemoizedValue,
    Recalculated,
    Error,
}

/// [`execute_memoized_function`] is the workhorse function of pico. Given
/// a [`DerivedNodeId`], which uniquely identifies the function being called
/// along with the parameters passed, it:
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
/// In all cases, we the [`DerivedNode`]'s `verified_at` will end up being the
/// current epoch.
///
/// **Checking whether we can reuse a previous invocation**:
///   - This is done by checking:
///     - whether the dependency was
///       [verified in the current epoch][crate::DatabaseStorage::node_verified_in_current_epoch], or
///     - whether [any dependency has changed][any_dependency_changed]
///       since this [`DerivedNode`] was last verified.
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
pub fn execute_memoized_function<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn<Db>,
) -> DidRecalculate {
    if db.get_storage().dependency_stack.is_empty() {
        // This is the outermost call to a memoized function. Keep track of all top_level_calls
        // for the purposes of later garbage collection. (Note that we also cannot update the LRU
        // cache right now, as that would require a mutable reference to the Database, which we do
        // not have.)
        db.get_storage().top_level_calls.push(derived_node_id);
    }

    let (did_recalculate, time_updated) = if let Some((derived_node, revision)) = db
        .get_storage()
        .internal
        .get_derived_node_and_revision(derived_node_id)
    {
        if db
            .get_storage()
            .internal
            .node_verified_in_current_epoch(derived_node_id)
        {
            event!(Level::TRACE, "epoch not changed");
            (DidRecalculate::ReusedMemoizedValue, revision.time_updated)
        } else {
            db.get_storage()
                .internal
                .verify_derived_node(derived_node_id);
            if any_dependency_changed(db, derived_node_id) {
                let _recalc_span = trace_span!("recalculating_due_to_dependency_change").entered();
                update_derived_node(db, derived_node_id, derived_node.value.as_ref(), inner_fn)
            } else {
                event!(Level::TRACE, "dependencies up-to-date");
                (DidRecalculate::ReusedMemoizedValue, revision.time_updated)
            }
        }
    } else {
        let _create_span = debug_span!("creating_new_derived_node").entered();
        create_derived_node(db, derived_node_id, inner_fn)
    };
    db.get_storage().register_dependency_in_parent_memoized_fn(
        NodeKind::Derived(derived_node_id),
        time_updated,
    );
    did_recalculate
}

fn create_derived_node<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn<Db>,
) -> (DidRecalculate, Epoch) {
    let (value, tracked_dependencies) =
        invoke_with_dependency_tracking(db, derived_node_id, inner_fn).expect(
            "InnerFn call cannot fail for a new derived node. This is indicative of a bug in Pico.",
        );
    let node_index = db
        .get_storage()
        .internal
        .insert_derived_node(DerivedNode { inner_fn, value });
    let dependency_index = db
        .get_storage()
        .internal
        .insert_dependencies(tracked_dependencies.dependencies);
    db.get_storage().internal.insert_derived_node_revision(
        derived_node_id,
        tracked_dependencies.max_time_updated,
        db.get_storage().internal.current_epoch,
        node_index,
        dependency_index,
    );
    (
        DidRecalculate::Recalculated,
        tracked_dependencies.max_time_updated,
    )
}

fn update_derived_node<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    prev_value: &dyn DynEq,
    inner_fn: InnerFn<Db>,
) -> (DidRecalculate, Epoch) {
    match invoke_with_dependency_tracking(db, derived_node_id, inner_fn) {
        Some((value, tracked_dependencies)) => {
            let mut occupied = if let Entry::Occupied(occupied) = db
                .get_storage()
                .internal
                .derived_node_id_to_revision
                .entry(derived_node_id)
            {
                occupied
            } else {
                panic!("Expected derived_node_id_to_revision to not be empty at this time");
            };

            let dependency_index = db
                .get_storage()
                .internal
                .insert_dependencies(tracked_dependencies.dependencies);
            let rev = occupied.get_mut();
            rev.dependency_index = dependency_index;

            let did_recalculate = if *prev_value != *value {
                event!(Level::TRACE, "value changed");
                let node_index = db
                    .get_storage()
                    .internal
                    .insert_derived_node(DerivedNode { inner_fn, value });
                rev.time_updated = tracked_dependencies.max_time_updated;
                rev.node_index = node_index;
                DidRecalculate::Recalculated
            } else {
                event!(Level::TRACE, "value up-to-date");
                DidRecalculate::ReusedMemoizedValue
            };

            (did_recalculate, tracked_dependencies.max_time_updated)
        }
        None => (DidRecalculate::Error, Epoch::new()),
    }
}

fn any_dependency_changed<Db: Database>(db: &Db, derived_node_id: DerivedNodeId) -> bool {
    let dependencies = db
        .get_storage()
        .internal
        .get_dependencies(derived_node_id)
        .expect("Expected dependencies to be present. This is indicative of a bug in Pico.");

    dependencies
        .iter()
        .filter(|dep| dep.time_verified_or_updated != db.get_storage().internal.current_epoch)
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
    match db.get_storage().internal.get_source_node(key) {
        Some(source) => source.time_updated > since,
        None => true,
    }
}

fn derived_node_changed_since<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    since: Epoch,
) -> bool {
    let inner_fn =
        if let Some(derived_node) = db.get_storage().internal.get_derived_node(derived_node_id) {
            if let Some(rev) = db
                .get_storage()
                .internal
                .get_derived_node_revision(derived_node_id)
            {
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
    let did_recalculate = execute_memoized_function(db, derived_node_id, inner_fn);
    matches!(
        did_recalculate,
        DidRecalculate::Recalculated | DidRecalculate::Error
    )
}

fn invoke_with_dependency_tracking<Db: Database>(
    db: &Db,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn<Db>,
) -> Option<(Box<dyn DynEq>, TrackedDependencies)> {
    let guard = db.get_storage().dependency_stack.enter(derived_node_id);
    let result = inner_fn.0(db, derived_node_id);
    let dependencies = guard.release();
    Some((result?, dependencies))
}
