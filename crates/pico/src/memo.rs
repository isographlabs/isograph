use dashmap::Entry;

use crate::{
    database::Database,
    dependency::{NodeKind, TrackedDependencies},
    derived_node::{DerivedNode, DerivedNodeId},
    dyn_eq::DynEq,
    epoch::Epoch,
    intern::Key,
    InnerFn,
};

pub enum DidRecalculate {
    ReusedMemoizedValue,
    Recalculated,
    Error,
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
///     [any dependency has changed][`any_dependency_changed`] since this
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
    let (time_updated, did_recalculate) =
        if let Some(derived_node) = db.get_derived_node(derived_node_id) {
            if db.node_verified_in_current_epoch(derived_node_id) {
                (db.current_epoch, DidRecalculate::ReusedMemoizedValue)
            } else {
                db.verify_derived_node(derived_node_id);
                if any_dependency_changed(db, derived_node) {
                    update_derived_node(db, derived_node_id, derived_node.value.as_ref(), inner_fn)
                } else {
                    (db.current_epoch, DidRecalculate::ReusedMemoizedValue)
                }
            }
        } else {
            create_derived_node(db, derived_node_id, inner_fn)
        };
    db.register_dependency_in_parent_memoized_fn(NodeKind::Derived(derived_node_id), time_updated);
    did_recalculate
}

fn create_derived_node(
    db: &Database,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn,
) -> (Epoch, DidRecalculate) {
    let (value, tracked_dependencies) = with_dependency_tracking(db, derived_node_id, inner_fn)
        .expect(
            "InnerFn call cannot fail for a new derived node. This is indicative of a bug in Pico.",
        );
    let index = db.insert_derived_node(DerivedNode {
        dependencies: tracked_dependencies.dependencies,
        inner_fn,
        value,
    });
    db.insert_derived_node_revision(
        derived_node_id,
        tracked_dependencies.max_time_updated,
        db.current_epoch,
        index,
    );
    (
        tracked_dependencies.max_time_updated,
        DidRecalculate::Recalculated,
    )
}

fn update_derived_node(
    db: &Database,
    derived_node_id: DerivedNodeId,
    prev_value: &dyn DynEq,
    inner_fn: InnerFn,
) -> (Epoch, DidRecalculate) {
    match with_dependency_tracking(db, derived_node_id, inner_fn) {
        Some((value, tracked_dependencies)) => {
            let mut occupied = if let Entry::Occupied(occupied) =
                db.derived_node_id_to_revision.entry(derived_node_id)
            {
                occupied
            } else {
                panic!("Expected derived_node_id_to_revision to not be empty at this time");
            };

            let did_recalculate = if *prev_value != *value {
                occupied.get_mut().time_updated = tracked_dependencies.max_time_updated;
                DidRecalculate::Recalculated
            } else {
                DidRecalculate::ReusedMemoizedValue
            };

            let index = db.insert_derived_node(DerivedNode {
                dependencies: tracked_dependencies.dependencies,
                inner_fn,
                value,
            });

            occupied.get_mut().index = index;

            (tracked_dependencies.max_time_updated, did_recalculate)
        }
        None => (Epoch::new(), DidRecalculate::Error),
    }
}

fn any_dependency_changed(db: &Database, derived_node: &DerivedNode) -> bool {
    derived_node
        .dependencies
        .iter()
        .filter(|dep| dep.time_verified_or_updated != db.current_epoch)
        .any(|dependency| match dependency.node_to {
            NodeKind::Source(key) => {
                source_node_changed_since(db, key, dependency.time_verified_or_updated)
            }
            NodeKind::Derived(dep_node_id) => {
                derived_node_changed_since(db, dep_node_id, dependency.time_verified_or_updated)
            }
        })
}

fn source_node_changed_since(db: &Database, key: Key, since: Epoch) -> bool {
    match db.source_nodes.get(&key) {
        Some(source) => source.time_updated > since,
        None => panic!(
            "Source node not found. This may occur if \
            a `SourceId` is used after the source node has been removed."
        ),
    }
}

fn derived_node_changed_since(db: &Database, derived_node_id: DerivedNodeId, since: Epoch) -> bool {
    let inner_fn = if let Some(derived_node) = db.get_derived_node(derived_node_id) {
        if let Some(rev) = db.get_derived_node_revision(derived_node_id) {
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
    matches!(
        did_recalculate,
        DidRecalculate::Recalculated | DidRecalculate::Error
    )
}

fn with_dependency_tracking(
    db: &Database,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn,
) -> Option<(Box<dyn DynEq>, TrackedDependencies)> {
    let guard = db.dependency_stack.enter();
    let result = inner_fn.0(db, derived_node_id);
    let dependencies = guard.release();
    Some((result?, dependencies))
}
