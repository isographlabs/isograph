use dashmap::Entry;

use crate::{
    database::Database,
    dependency::{NodeKind, TrackedDependencies},
    derived_node::{DerivedNode, DerivedNodeId},
    dyn_eq::DynEq,
    epoch::Epoch,
    intern::Key,
    InnerFn, KeyOrTypeId,
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
impl Database {
    pub(crate) fn execute_memoized_function(
        &self,
        derived_node_id: DerivedNodeId,
        inner_fn: InnerFn,
    ) -> DidRecalculate {
        if self.dependency_stack.is_empty() {
            // This is the outermost call to a memoized function. Keep track of all top_level_calls
            // for the purposes of later garbage collection. (Note that we also cannot update the LRU
            // cache right now, as that would require a mutable reference to the Database, which we do
            // not have.)
            self.top_level_calls.push(derived_node_id);
        }

        let (time_updated, did_recalculate) =
            if let Some(derived_node) = self.storage.get_derived_node(derived_node_id) {
                if self.storage.node_verified_in_current_epoch(derived_node_id) {
                    (
                        self.storage.current_epoch,
                        DidRecalculate::ReusedMemoizedValue,
                    )
                } else {
                    self.storage.verify_derived_node(derived_node_id);
                    if any_dependency_changed(self, derived_node) {
                        update_derived_node(
                            self,
                            derived_node_id,
                            derived_node.value.as_ref(),
                            inner_fn,
                        )
                    } else {
                        (
                            self.storage.current_epoch,
                            DidRecalculate::ReusedMemoizedValue,
                        )
                    }
                }
            } else {
                create_derived_node(self, derived_node_id, inner_fn)
            };
        self.register_dependency_in_parent_memoized_fn(
            NodeKind::Derived(derived_node_id),
            time_updated,
        );
        did_recalculate
    }
}

fn create_derived_node(
    db: &Database,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn,
) -> (Epoch, DidRecalculate) {
    let (value, tracked_dependencies) =
        invoke_with_dependency_tracking(db, derived_node_id, inner_fn).expect(
            "InnerFn call cannot fail for a new derived node. This is indicative of a bug in Pico.",
        );
    let index = db.storage.insert_derived_node(DerivedNode {
        dependencies: tracked_dependencies.dependencies,
        inner_fn,
        value,
    });
    db.storage.insert_derived_node_revision(
        derived_node_id,
        tracked_dependencies.max_time_updated,
        db.storage.current_epoch,
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
    match invoke_with_dependency_tracking(db, derived_node_id, inner_fn) {
        Some((value, tracked_dependencies)) => {
            let mut occupied = if let Entry::Occupied(occupied) = db
                .storage
                .derived_node_id_to_revision
                .entry(derived_node_id)
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

            let index = db.storage.insert_derived_node(DerivedNode {
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
        .filter(|dep| dep.time_verified_or_updated != db.storage.current_epoch)
        .any(|dependency| match dependency.node_to {
            NodeKind::Source(key_or_type_id) => {
                source_node_changed_since(db, key_or_type_id, dependency.time_verified_or_updated)
            }
            NodeKind::Derived(dep_node_id) => {
                derived_node_changed_since(db, dep_node_id, dependency.time_verified_or_updated)
            }
        })
}

fn source_node_changed_since(db: &Database, key_or_type_id: KeyOrTypeId, since: Epoch) -> bool {
    match db.storage.get_source_node(key_or_type_id) {
        Some(source) => source.time_updated > since,
        None => panic!(
            "Source node not found. This may occur if \
            a `SourceId` is used after the source node has been removed."
        ),
    }
}

fn derived_node_changed_since(db: &Database, derived_node_id: DerivedNodeId, since: Epoch) -> bool {
    let inner_fn = if let Some(derived_node) = db.storage.get_derived_node(derived_node_id) {
        if let Some(rev) = db.storage.get_derived_node_revision(derived_node_id) {
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
    let did_recalculate = db.execute_memoized_function(derived_node_id, inner_fn);
    matches!(
        did_recalculate,
        DidRecalculate::Recalculated | DidRecalculate::Error
    )
}

fn invoke_with_dependency_tracking(
    db: &Database,
    derived_node_id: DerivedNodeId,
    inner_fn: InnerFn,
) -> Option<(Box<dyn DynEq>, TrackedDependencies)> {
    let guard = db.dependency_stack.enter(derived_node_id);
    let result = inner_fn.0(db, derived_node_id);
    let dependencies = guard.release();
    Some((result?, dependencies))
}
