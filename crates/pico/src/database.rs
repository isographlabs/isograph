use std::{any::Any, num::NonZeroUsize};

use crate::{
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    index::Index,
    intern::{Key, ParamId},
    source::{Source, SourceId, SourceNode},
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
}

#[derive(Debug)]
pub(crate) struct DatabaseStorage {
    pub(crate) param_id_to_index: DashMap<ParamId, Index<ParamId>>,
    pub(crate) derived_node_id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
    pub(crate) source_nodes: DashMap<Key, SourceNode>,

    pub(crate) derived_nodes: BoxcarVec<DerivedNode>,
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

                source_nodes: DashMap::new(),
                derived_nodes: BoxcarVec::new(),
                params: BoxcarVec::new(),

                current_epoch: Epoch::new(),
            },
            top_level_calls: BoxcarVec::new(),
            top_level_call_lru_cache: LruCache::new(capacity),
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

    pub fn get<T: Clone + 'static>(&self, id: SourceId<T>) -> T {
        let time_updated = self
            .storage
            .source_nodes
            .get(&id.key)
            .expect("node should exist. This is indicative of a bug in Pico.")
            .time_updated;
        self.register_dependency_in_parent_memoized_fn(NodeKind::Source(id.key), time_updated);
        self.storage.get_source(id)
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

        // GC will be a noop if there's no possibility that a top-level call will be evicted
        // from the cache. If so, we bail early.
        let gc_will_be_noop = self.top_level_calls.count() + self.top_level_call_lru_cache.len()
            < self.top_level_call_lru_cache.cap().into();

        let top_level_function_calls =
            std::mem::replace(&mut self.top_level_calls, BoxcarVec::new());

        for derived_node_id in top_level_function_calls {
            self.top_level_call_lru_cache.put(derived_node_id, ());
        }

        if gc_will_be_noop {
            return;
        }

        self.storage
            .run_garbage_collection(self.top_level_call_lru_cache.iter().map(|(k, _v)| *k));
    }

    fn assert_empty_dependency_stack(&self) {
        assert!(
            self.dependency_stack.is_empty(),
            "Cannot modify database while a memoized function is being invoked."
        );
    }
}

impl DatabaseStorage {
    pub(crate) fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(self.params.get(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Isograph.",
        ))
    }

    pub(crate) fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
        let index = self
            .derived_node_id_to_revision
            .get(&derived_node_id)?
            .index;
        Some(self.derived_nodes.get(index.idx).expect(
            "indexes should always be valid. \
            This is indicative of a bug in Isograph.",
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

    pub fn get_source<T: Clone + 'static>(&self, id: SourceId<T>) -> T {
        self.source_nodes
            .get(&id.key)
            .expect("value should exist. This is indicative of a bug in Pico.")
            .value
            .as_any()
            .downcast_ref::<T>()
            .expect("unexpected struct type. This is indicative of a bug in Pico.")
            .clone()
    }

    /// Sets a source in the database. If there is an existing item and it does not equal
    /// the new source, increment the current epoch.
    pub fn set_source<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        let id = SourceId::new(&source);
        match self.source_nodes.entry(id.key) {
            Entry::Occupied(mut occupied_entry) => {
                let existing_node = occupied_entry.get();
                if !existing_node.value.dyn_eq(&source) {
                    // We cannot call self.increment_epoch() because that borrows
                    // the entire struct, but self.source_nodes is already borrowed
                    let next_epoch = self.current_epoch.increment();

                    *(occupied_entry.get_mut()) = SourceNode {
                        time_updated: next_epoch,
                        value: Box::new(source),
                    };
                } else {
                    occupied_entry.get_mut().time_updated = self.current_epoch;
                };
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(SourceNode {
                    time_updated: self.current_epoch,
                    value: Box::new(source),
                });
            }
        };
        id
    }

    pub fn remove_source<T>(&mut self, id: SourceId<T>) {
        self.current_epoch.increment();
        self.source_nodes.remove(&id.key);
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
