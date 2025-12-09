use std::{any::Any, hash::Hash, num::NonZeroUsize};

use crate::{
    InnerFn, MemoRef, MemoRefKind, RawPtr, Singleton,
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    index::Index,
    intern::{Key, ParamId},
    macro_fns::{hash, init_param_vec},
    source::{Source, SourceId, SourceNode},
};
use boxcar::Vec as BoxcarVec;
use dashmap::{DashMap, Entry};
use lru::LruCache;

use crate::derived_node::{DerivedNode, DerivedNodeId, DerivedNodeRevision};

pub trait DatabaseDyn {
    fn get_storage_dyn(&self) -> &dyn StorageDyn;
}
pub trait Database: DatabaseDyn + Sized {
    fn get_storage(&self) -> &Storage<Self>;
    fn get<T: 'static>(&self, id: SourceId<T>) -> &T;
    /// Because `T` is a `Singleton`, unlike `get` this does not require `id: SourceId<T>`
    fn get_singleton<T: 'static + Singleton>(&self) -> Option<&T>;
    fn intern_value<T: Clone + Hash + DynEq + 'static>(&self, value: T) -> MemoRef<T>;
    fn intern_ref<T: Clone + Hash + DynEq + 'static>(&self, value: &T) -> MemoRef<T>;
    fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T>;
    fn remove<T>(&mut self, id: SourceId<T>);
    fn remove_singleton<T: Singleton + 'static>(&mut self);
    fn run_garbage_collection(&mut self);
}

pub trait StorageDyn {
    fn get_derived_node_value_and_revision(
        &self,
        id: DerivedNodeId,
    ) -> Option<(&dyn Any, DerivedNodeRevision)>;

    fn register_dependency_in_parent_memoized_fn(&self, node: NodeKind, time_updated: Epoch);
}

#[derive(Debug)]
pub struct Storage<Db: Database> {
    pub(crate) dependency_stack: DependencyStack,
    pub(crate) internal: InternalStorage<Db>,
    pub(crate) top_level_calls: BoxcarVec<DerivedNodeId>,
    pub(crate) top_level_call_lru_cache: LruCache<DerivedNodeId, ()>,
    pub(crate) retained_calls: DashMap<DerivedNodeId, usize>,
}

impl<Db: Database> StorageDyn for Storage<Db> {
    fn get_derived_node_value_and_revision(
        &self,
        id: DerivedNodeId,
    ) -> Option<(&dyn Any, DerivedNodeRevision)> {
        self.internal
            .get_derived_node_and_revision(id)
            .map(|(node, revision)| (node.value.as_ref().as_any(), revision))
    }

    fn register_dependency_in_parent_memoized_fn(&self, node: NodeKind, time_updated: Epoch) {
        Storage::register_dependency_in_parent_memoized_fn(self, node, time_updated);
    }
}

#[derive(Debug)]
pub(crate) struct InternalStorage<Db: Database> {
    pub(crate) param_id_to_index: DashMap<ParamId, Index<ParamId>>,
    pub(crate) derived_node_id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
    pub(crate) source_node_key_to_index: DashMap<Key, Index<SourceNode>>,

    pub(crate) derived_nodes: BoxcarVec<DerivedNode<Db>>,
    pub(crate) derived_node_dependencies: BoxcarVec<Vec<Dependency>>,
    pub(crate) source_nodes: BoxcarVec<Option<SourceNode>>,
    pub(crate) params: BoxcarVec<Box<dyn Any>>,
    pub(crate) current_epoch: Epoch,
}

static DEFAULT_CAPACITY: usize = 10_000;

impl<Db: Database> Storage<Db> {
    pub fn new() -> Self {
        Storage::new_with_capacity(DEFAULT_CAPACITY.try_into().unwrap())
    }

    pub fn new_with_capacity(capacity: NonZeroUsize) -> Self {
        Self {
            dependency_stack: DependencyStack::new(),
            internal: InternalStorage {
                param_id_to_index: DashMap::new(),
                derived_node_id_to_revision: DashMap::new(),
                source_node_key_to_index: DashMap::new(),

                source_nodes: BoxcarVec::new(),
                derived_nodes: BoxcarVec::new(),
                derived_node_dependencies: BoxcarVec::new(),
                params: BoxcarVec::new(),

                current_epoch: Epoch::new(),
            },
            top_level_calls: BoxcarVec::new(),
            top_level_call_lru_cache: LruCache::new(capacity),
            retained_calls: DashMap::new(),
        }
    }

    pub(crate) fn register_dependency_in_parent_memoized_fn(
        &self,
        node: NodeKind,
        time_updated: Epoch,
    ) {
        self.dependency_stack.push_if_not_empty(
            Dependency {
                node_to: node,
                time_verified_or_updated: self.internal.current_epoch,
            },
            time_updated,
        );
    }

    pub fn get<T: 'static>(&self, id: SourceId<T>) -> &T {
        self.get_impl(id.key).expect(
            "Source node not found in database. SourceId should not be used \
            after the corresponding source node is removed.",
        )
    }

    pub fn get_singleton<T: 'static + Singleton>(&self) -> Option<&T> {
        self.get_impl(T::get_singleton_key())
    }

    fn get_impl<T: 'static>(&self, key: Key) -> Option<&T> {
        let source_node = self.internal.get_source_node(key)?;

        self.register_dependency_in_parent_memoized_fn(
            NodeKind::Source(key),
            source_node.time_updated,
        );
        Some(
            source_node
                .value
                .as_ref()
                .as_any()
                .downcast_ref::<T>()
                .expect(
                    "unexpected struct type. \
            This is indicative of a bug in Pico.",
                ),
        )
    }

    /// As long as a memoized function is not currently being invoked
    /// modify the database by setting a source
    /// Increment the current epoch if it is a new and different item
    pub fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        self.assert_empty_dependency_stack();
        let source_id = SourceId::new(&source);
        self.internal.set_source(source, source_id);
        source_id
    }

    pub fn remove<T>(&mut self, id: SourceId<T>) {
        self.assert_empty_dependency_stack();
        self.internal.remove_source(id);
    }

    pub fn remove_singleton<T: Singleton + 'static>(&mut self) {
        self.assert_empty_dependency_stack();
        self.internal
            .remove_source::<T>(T::get_singleton_key().into());
    }

    pub fn run_garbage_collection(&mut self) {
        self.assert_empty_dependency_stack();

        let top_level_function_calls = std::mem::take(&mut self.top_level_calls);

        for derived_node_id in top_level_function_calls {
            self.top_level_call_lru_cache.put(derived_node_id, ());
        }

        // Retain the queries in the LRU cache and the queries that are permanently retained,
        // and everything reachable from them.
        //
        // If we have a functon f(input) -> X, then input changed values, and f(input) -> Y,
        // then the X node is inaccessible and will be garbage collected.
        let retained_derived_node_ids = self
            .top_level_call_lru_cache
            .iter()
            .map(|(k, _v)| *k)
            .chain(self.retained_calls.iter().map(|ref_multi| *ref_multi.key()));

        self.internal
            .run_garbage_collection(retained_derived_node_ids);
    }

    fn assert_empty_dependency_stack(&self) {
        assert!(
            self.dependency_stack.is_empty(),
            "Cannot modify database while a memoized function is being invoked."
        );
    }
}

impl<Db: Database> InternalStorage<Db> {
    pub(crate) fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(self.params.get(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        ))
    }

    pub(crate) fn get_derived_node(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<&DerivedNode<Db>> {
        self.get_derived_node_and_revision(derived_node_id)
            .map(|(node, _)| node)
    }

    pub(crate) fn get_derived_node_from_derived_node_revision(
        &self,
        revision: &DerivedNodeRevision,
    ) -> &DerivedNode<Db> {
        self.derived_nodes.get(revision.node_index.idx).expect(
            "Indexes should always be valid. \
            This is indicative of a bug in Pico.",
        )
    }

    pub(crate) fn get_derived_node_and_revision(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<(&DerivedNode<Db>, DerivedNodeRevision)> {
        let revision = *self.derived_node_id_to_revision.get(&derived_node_id)?;

        let node = self.get_derived_node_from_derived_node_revision(&revision);

        Some((node, revision))
    }

    pub(crate) fn get_dependencies(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<&Vec<Dependency>> {
        let revision = *self.derived_node_id_to_revision.get(&derived_node_id)?;
        self.derived_node_dependencies
            .get(revision.dependency_index.idx)
    }

    pub(crate) fn insert_dependencies(&self, dependencies: Vec<Dependency>) -> Index<Dependency> {
        Index::new(self.derived_node_dependencies.push(dependencies))
    }

    pub(crate) fn node_verified_in_current_epoch(&self, derived_node_id: DerivedNodeId) -> bool {
        self.derived_node_id_to_revision
            .get(&derived_node_id)
            .map(|rev| rev.time_verified == self.current_epoch)
            .unwrap()
    }

    pub(crate) fn verify_derived_node(&self, derived_node_id: DerivedNodeId) {
        let mut rev = self
            .derived_node_id_to_revision
            .get_mut(&derived_node_id)
            .unwrap();
        rev.time_verified = self.current_epoch;
    }

    pub(crate) fn get_derived_node_revision(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<DerivedNodeRevision> {
        self.derived_node_id_to_revision
            .get(&derived_node_id)
            .map(|rev| *rev)
    }

    pub(crate) fn insert_derived_node_revision(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
        time_verified: Epoch,
        node_index: Index<DerivedNodeId>,
        dependency_index: Index<Dependency>,
    ) {
        self.derived_node_id_to_revision.insert(
            derived_node_id,
            DerivedNodeRevision {
                time_updated,
                time_verified,
                node_index,
                dependency_index,
            },
        );
    }

    pub(crate) fn insert_derived_node(
        &self,
        derived_node: DerivedNode<Db>,
    ) -> Index<DerivedNodeId> {
        Index::new(self.derived_nodes.push(derived_node))
    }

    pub fn get_source_node(&self, key: Key) -> Option<&SourceNode> {
        let index = self.source_node_key_to_index.get(&key)?;
        self.source_nodes
            .get(index.idx)
            .expect(
                "indexes should always be valid. \
                This is indicative of a bug in Pico.",
            )
            .as_ref()
    }

    pub(crate) fn insert_source_node(&self, source_node: SourceNode) -> Index<SourceNode> {
        Index::new(self.source_nodes.push(Some(source_node)))
    }

    /// Sets a source in the database. If there is an existing item and it does not equal
    /// the new source, increment the current epoch.
    fn set_source<T: DynEq>(&mut self, source: T, source_id: SourceId<T>) {
        match self.source_node_key_to_index.entry(source_id.key) {
            Entry::Occupied(occupied_entry) => {
                let source_node = self
                    .source_nodes
                    .get_mut(occupied_entry.get().idx)
                    .expect(
                        "indexes should always be valid. \
                        This is indicative of a bug in Pico.",
                    )
                    .as_mut()
                    .expect(
                        "indexes should always point to a non-empty source node. \
                        This is indicative of a bug in Pico.",
                    );
                if !source_node.value.as_ref().dyn_eq(&source) {
                    // We cannot call self.increment_epoch() because that borrows
                    // the entire struct, but self.source_nodes is already borrowed
                    let next_epoch = self.current_epoch.increment();
                    *source_node = SourceNode {
                        time_updated: next_epoch,
                        value: Box::new(source),
                    };
                } else {
                    source_node.time_updated = self.current_epoch;
                }
            }
            Entry::Vacant(vacant_entry) => {
                let index = self.insert_source_node(SourceNode {
                    time_updated: self.current_epoch,
                    value: Box::new(source),
                });
                vacant_entry.insert(index);
            }
        }
    }

    pub fn remove_source<T>(&mut self, id: SourceId<T>) {
        if let Some((_, index)) = self.source_node_key_to_index.remove(&id.key) {
            self.current_epoch.increment();
            self.source_nodes
                .get_mut(index.idx)
                .expect(
                    "indexes should always be valid. \
                    This is indicative of a bug in Pico.",
                )
                .take();
        }
    }
}

impl<Db: Database> Default for Storage<Db> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn intern_value<Db: Database, T: Clone + Hash + DynEq + 'static>(
    db: &Db,
    value: T,
) -> MemoRef<T> {
    let wrapped_value = InternValueWrapper(value);
    let param_id = hash(&wrapped_value).into();
    let value = wrapped_value.0;

    let mut param_ids = init_param_vec();
    param_ids.push(param_id);
    let derived_node_id = DerivedNodeId::new(param_id.inner().into(), param_ids);

    let current_epoch = db.get_storage().internal.current_epoch;

    let time_updated = match db
        .get_storage()
        .internal
        .derived_node_id_to_revision
        .entry(derived_node_id)
    {
        Entry::Vacant(vacant) => {
            let node_index = db.get_storage().internal.insert_derived_node(DerivedNode {
                inner_fn: InnerFn::new(|_, _| {
                    unreachable!("interned derived node should never be executed")
                }),
                value: Box::new(value),
            });

            let dependency_index = db.get_storage().internal.insert_dependencies(Vec::new());

            vacant.insert(DerivedNodeRevision {
                time_updated: current_epoch,
                time_verified: current_epoch,
                node_index,
                dependency_index,
            });

            current_epoch
        }
        Entry::Occupied(mut occupied) => {
            let revision = occupied.get_mut();

            if revision.time_verified != current_epoch {
                revision.time_verified = current_epoch;
            }

            revision.time_updated
        }
    };

    db.get_storage().register_dependency_in_parent_memoized_fn(
        NodeKind::Derived(derived_node_id),
        time_updated,
    );

    MemoRef::new(derived_node_id)
}

/// In this function, we create a [`MemoRef`], which is a wrapper around a pointer.
///
/// We want to ensure several things:
/// - that when the MemoRef is read (via `memo_ref.lookup(db)`), the value it points to has
///   not been garbage collected, and
/// - that when a function reads the MemoRef, the time_updated of the derived node revision of
///   the MemoRef is the epoch when the MemoRef was initially created (i.e. to the first epoch when
///   `db.intern_ref(&value)` was called for a particular value.) This allows for short circuiting.
///
/// This is trickier than it sounds, because we can have multiple MemoRef's that point to identical
/// data, but which is held in different memory locations. In particular, consider the following
/// scenario:
///
/// fn get_tuple(db) {
///   (db.get_source(), "identical value".to_string())
/// }
/// fn get_memo_ref_to_second_item_in_tuple(db) {
///   db.intern_ref(&get_tuple().1) // returns a MemoRef to &"identical value".to_string()
/// }
/// fn outer_fn_1() {
///   let memo_ref = get_memo_ref_to_second_item_in_tuple(db);
///   memo_ref.lookup(db);
/// }
/// fn outer_fn_2() {
///   let memo_ref = get_memo_ref_to_second_item_in_tuple(db);
///   memo_ref.lookup(db);
/// }
///
/// - dependency graph: source <- get_tuple <- get_memo_ref_to_second_item_in_tuple <-- outer_fn_1
/// - dependency graph: source <- get_tuple <- get_memo_ref_to_second_item_in_tuple <-- outer_fn_2
///
/// - epoch 0
/// - call outer_fn_1. All functions executed.
/// - update source, enter epoch 1
/// - call outer_fn_1.
///   - outer_fn is checked
///     - get_memo_ref_to_second_item_in_tuple is checked
///       - get_tuple is checked; its dependencies have changed, so it is re-executed
///     - get_memo_ref_to_second_item's dependencies have changed, so it is re-executed
///       - however, the derived node revision of the memo_ref continues to have time_updated: 0
///   - outer_fn_1's dependencies have *not* changed, so we short circuit
/// - GC everything from epoch 0
/// - call outer_fn_2
///   - check get_memo_ref_to_second_item_in_tuple. It has been verified in the current epoch
///     (and its dependencies have not changed), so reuse the existing value. (*)
///   - outer_fn_2 calls `memo_ref.lookup(db)`
///
/// Uh oh! What data does the memo ref point to? It better point to the tuple created during epoch 1.
///
/// In other words, "reuse the existing value" is not quite right. We must return a MemoRef that points
/// to data from epoch 1, but indicate (to pico) that it has not changed since epoch 0. i.e. we employ
/// interior mutability to give the illusion that this is the same MemoRef.
///
/// This is achieved as follows:
/// - A MemoRef's identity (param_id) is based on the value it points to, i.e. not the the memory
///   address it points to. So the MemoRef's created in get_memo_ref_to_second_item_in_tuple in various
///   epochs have the same identity.
/// - If a MemoRef is created and already exists, then we silently mutate the MemoRef to point to the
///   memory address of whatever was most recently passed in.
/// - The derived node revision of the MemoRef's time_updated remains unchanged, i.e. it remains the
///   epoch from when the MemoRef (with a given identity) was created.
///
/// Note that if we call `db.intern_ref(different_value)`, then this MemoRef has a different identity,
/// so there are no problems there.
pub fn intern_ref<Db: Database, T: Clone + Hash + DynEq + 'static>(
    db: &Db,
    value: &T,
) -> MemoRef<T> {
    // the param_id (identity) of the MemoRef is the hash of the value, *not* of the memory address.
    let param_id = hash(value).into();

    let mut param_ids = init_param_vec();
    param_ids.push(param_id);
    let derived_node_id = DerivedNodeId::new(param_id.inner().into(), param_ids);

    let new_ptr = RawPtr::from_ref(value);
    let current_epoch = db.get_storage().internal.current_epoch;
    let time_updated = match db
        .get_storage()
        .internal
        .derived_node_id_to_revision
        .entry(derived_node_id)
    {
        Entry::Vacant(vacant) => {
            let node_index = db.get_storage().internal.insert_derived_node(DerivedNode {
                inner_fn: InnerFn::new(|_, _| {
                    unreachable!("intern_ref derived node should never be executed")
                }),
                value: Box::new(new_ptr),
            });

            let dependency_index = db.get_storage().internal.insert_dependencies(Vec::new());

            vacant.insert(DerivedNodeRevision {
                time_updated: current_epoch,
                time_verified: current_epoch,
                node_index,
                dependency_index,
            });

            current_epoch
        }
        Entry::Occupied(mut occupied) => {
            let revision = occupied.get_mut();

            if revision.time_verified != current_epoch {
                // Note: we cannot call internal.get_derived_node(derived_node_id) here, because
                // - have a mutable reference to the item in the dashmap via
                //   derived_node_id_to_revision.entry(derived_node_id), and
                // - get_derived_node calls derived_node_id_to_revision.get(derived_node_id)
                //
                // This will deadlock.
                let existing_node = db
                    .get_storage()
                    .internal
                    .get_derived_node_from_derived_node_revision(revision);

                let existing_ptr = existing_node
                    .value
                    .as_ref()
                    .as_any()
                    .downcast_ref::<RawPtr<T>>()
                    .expect("Unexpected memoized value type. This is indicative of a bug in Pico.");

                let pointer_changed = *existing_ptr != new_ptr;
                if pointer_changed {
                    // If we get here, then we have called db.intern_ref(&value) with a value pointing to a new
                    // memory location, but with an identical value.
                    //
                    // Here, we mutate the revision to point to a DerivedNode which points to the new memory address.
                    //
                    // We do *not* update the time_updated, though.
                    let node_index = db.get_storage().internal.insert_derived_node(DerivedNode {
                        inner_fn: existing_node.inner_fn,
                        value: Box::new(new_ptr),
                    });
                    revision.node_index = node_index;
                }

                revision.time_verified = current_epoch;
            }

            revision.time_updated
        }
    };

    db.get_storage().register_dependency_in_parent_memoized_fn(
        NodeKind::Derived(derived_node_id),
        time_updated,
    );

    MemoRef::new_with_kind(derived_node_id, MemoRefKind::RawPtr)
}

// This is wrapper exists solely so that if we call db.intern(val) and db.intern_ref(&val)
// for the same value, then we do not calculate the same hash, and thus return a MemoRef
// for the first when looking up the second.
//
// We make a somewhat arbitrary choice and choose to wrap interned values.
#[derive(Hash)]
struct InternValueWrapper<T>(T);
