use std::{any::Any, hash::Hash, num::NonZeroUsize};

use crate::{
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    index::Index,
    intern::{Key, ParamId},
    macro_fns::{get_param, init_param_vec, intern_borrowed_param, intern_owned_param},
    source::{Source, SourceId, SourceNode},
    InnerFn, MemoRef,
};
use boxcar::Vec as BoxcarVec;
use dashmap::{DashMap, Entry};
use lru::LruCache;

use crate::derived_node::{DerivedNode, DerivedNodeId, DerivedNodeRevision};

#[derive(Debug)]
pub struct Database {
    pub(crate) dependency_stack: DependencyStack,
    pub(crate) storage: DatabaseStorage,
    pub(crate) top_level_calls: BoxcarVec<DerivedNodeId>,
    pub(crate) top_level_call_lru_cache: LruCache<DerivedNodeId, ()>,
    pub(crate) retained_calls: DashMap<DerivedNodeId, usize>,
}

#[derive(Debug)]
pub(crate) struct DatabaseStorage {
    pub(crate) param_id_to_index: DashMap<ParamId, Index<ParamId>>,
    pub(crate) derived_node_id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
    pub(crate) source_node_key_to_index: DashMap<Key, Index<SourceNode>>,

    pub(crate) derived_nodes: BoxcarVec<DerivedNode>,
    pub(crate) source_nodes: BoxcarVec<Option<SourceNode>>,
    pub(crate) params: BoxcarVec<Box<dyn Any>>,
    pub(crate) current_epoch: Epoch,
}

static DEFAULT_CAPACITY: usize = 10_000;

impl Database {
    pub fn new() -> Self {
        Database::new_with_capacity(DEFAULT_CAPACITY.try_into().unwrap())
    }

    pub fn new_with_capacity(capacity: NonZeroUsize) -> Self {
        Self {
            dependency_stack: DependencyStack::new(),
            storage: DatabaseStorage {
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
                time_verified_or_updated: self.storage.current_epoch,
            },
            time_updated,
        );
    }

    pub fn get<T: 'static>(&self, id: SourceId<T>) -> &T {
        let source_node = self.storage.get_source_node(id.key).expect(
            "source node not found. SourceId should not be used \
            after the corresponding source node is removed.",
        );
        self.register_dependency_in_parent_memoized_fn(
            NodeKind::Source(id.key),
            source_node.time_updated,
        );
        source_node.value.as_any().downcast_ref::<T>().expect(
            "unexpected struct type. \
            This is indicative of a bug in Pico.",
        )
    }

    pub fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        self.assert_empty_dependency_stack();
        self.storage.set_source(source)
    }

    pub fn remove<T>(&mut self, id: SourceId<T>) {
        self.assert_empty_dependency_stack();
        self.storage.remove_source(id)
    }

    pub fn run_garbage_collection(&mut self) {
        self.assert_empty_dependency_stack();

        let top_level_function_calls =
            std::mem::replace(&mut self.top_level_calls, BoxcarVec::new());

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

        self.storage
            .run_garbage_collection(retained_derived_node_ids);
    }

    fn assert_empty_dependency_stack(&self) {
        assert!(
            self.dependency_stack.is_empty(),
            "Cannot modify database while a memoized function is being invoked."
        );
    }

    pub fn intern<T: Clone + Hash + DynEq + 'static>(&self, value: T) -> MemoRef<T> {
        let param_id = intern_owned_param(self, value);
        intern_from_param(self, param_id)
    }

    pub fn intern_ref<T: Clone + Hash + DynEq + 'static>(&self, value: &T) -> MemoRef<T> {
        let param_id = intern_borrowed_param(self, value);
        intern_from_param(self, param_id)
    }
}

impl DatabaseStorage {
    pub(crate) fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(self.params.get(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        ))
    }

    pub(crate) fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
        let index = self
            .derived_node_id_to_revision
            .get(&derived_node_id)?
            .index;
        Some(self.derived_nodes.get(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Pico.",
        ))
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

    pub(crate) fn insert_derived_node(&self, derived_node: DerivedNode) -> Index<DerivedNodeId> {
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
    pub fn set_source<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        let id = SourceId::new(&source);
        match self.source_node_key_to_index.entry(id.key) {
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
                if !source_node.value.dyn_eq(&source) {
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
        id
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

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}

fn intern_from_param<T: Clone + DynEq>(db: &Database, param_id: ParamId) -> MemoRef<T> {
    let mut param_ids = init_param_vec();
    param_ids.push(param_id);
    let derived_node_id = DerivedNodeId::new(param_id.inner().into(), param_ids);
    db.execute_memoized_function(
        derived_node_id,
        InnerFn::new(|db, derived_node_id| {
            let param = get_param(db, derived_node_id.params[0])?
                .downcast_ref::<T>()
                .expect("Unexpected param type. This is indicative of a bug in Pico.");
            Some(Box::new(param.clone()))
        }),
    );
    MemoRef::new(db, derived_node_id)
}
