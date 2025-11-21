use std::{any::Any, hash::Hash, num::NonZeroUsize};

use crate::{
    InnerFn, MemoRef, MemoRefKind, RawPtr, Singleton,
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    execute_memoized_function,
    index::Index,
    intern::{Key, ParamId},
    macro_fns::{get_param, hash, init_param_vec, intern_owned_param},
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
    fn intern<T: Clone + Hash + DynEq + 'static>(&self, value: T) -> MemoRef<T>;
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

        let top_level_function_calls =
            std::mem::replace(&mut self.top_level_calls, BoxcarVec::new());

        eprintln!("gc 1 {:?}", top_level_function_calls);
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

    pub(crate) fn get_derived_node_and_revision(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<(&DerivedNode<Db>, DerivedNodeRevision)> {
        let revision = *self.derived_node_id_to_revision.get(&derived_node_id)?;

        let node = self.derived_nodes.get(revision.index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        );

        Some((node, revision))
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
        index: Index<DerivedNodeId>,
    ) {
        self.derived_node_id_to_revision.insert(
            derived_node_id,
            DerivedNodeRevision {
                time_updated,
                time_verified,
                index,
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

pub fn intern<Db: Database, T: Clone + Hash + DynEq + 'static>(db: &Db, value: T) -> MemoRef<T> {
    let param_id = intern_owned_param(db, value);
    intern_from_param(db, param_id, MemoRefKind::Value)
}

pub fn intern_ref<Db: Database, T: Clone + Hash + DynEq + 'static>(
    db: &Db,
    value: &T,
) -> MemoRef<T> {
    let param_id = hash(value).into();
    if let Entry::Vacant(v) = db.get_storage().internal.param_id_to_index.entry(param_id) {
        let idx = db
            .get_storage()
            .internal
            .params
            .push(Box::new(RawPtr::from_ref(value)));
        v.insert(Index::new(idx));
    }
    intern_from_param(db, param_id, MemoRefKind::RawPtr)
}

fn intern_from_param<Db: Database, T: Clone + DynEq>(
    db: &Db,
    param_id: ParamId,
    kind: MemoRefKind,
) -> MemoRef<T> {
    let mut param_ids = init_param_vec();
    param_ids.push(param_id);
    let derived_node_id = DerivedNodeId::new(param_id.inner().into(), param_ids);
    let inner_fn = match kind {
        MemoRefKind::Value => InnerFn::new(|db, derived_node_id| {
            let param_ref = get_param(db, derived_node_id.params[0])?;
            let param = param_ref
                .downcast_ref::<T>()
                .expect("Unexpected param type. This is indicative of a bug in Pico.");
            Some(Box::new(param.clone()))
        }),
        MemoRefKind::RawPtr => InnerFn::new(|db, derived_node_id| {
            let param_ref = get_param(db, derived_node_id.params[0])?;
            let param = param_ref
                .downcast_ref::<RawPtr<T>>()
                .expect("Unexpected param type. This is indicative of a bug in Pico.");
            Some(Box::new(*param))
        }),
    };
    execute_memoized_function(db, derived_node_id, inner_fn);
    MemoRef::new_with_kind(derived_node_id, kind)
}
