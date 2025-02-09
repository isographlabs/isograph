use std::{any::Any, num::NonZeroUsize};

use crate::{
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    source::{Source, SourceId, SourceNode},
    u64_types::{Key, ParamId},
    with_dependency_tracking, DerivedNodeIndex, DidRecalculate, InnerFn,
};
use boxcar::Vec as BoxcarVec;
use dashmap::{mapref::one::Ref, DashMap, Entry};
use lru::LruCache;

use crate::derived_node::{DerivedNode, DerivedNodeId};

#[derive(Debug)]
pub struct Database {
    pub(crate) dependency_stack: DependencyStack,
    pub(crate) params: DashMap<ParamId, Box<dyn Any>>,
    pub(crate) source_nodes: DashMap<Key, SourceNode>,

    pub(crate) current_epoch: Epoch,

    // We store the derived nodes in this map, and when accessing them
    // record the access in the access_vec. Later, when we garbage collect,
    // we transfer the accesses to the lru cache, and remove remaining
    // nodes from the derived_nodes
    pub(crate) derived_nodes: DashMap<DerivedNodeId, DerivedNode>,
    pub(crate) access_vec: BoxcarVec<DerivedNodeId>,
    pub(crate) derived_node_lru_cache: LruCache<DerivedNodeId, ()>,

    pub(crate) derived_node_values: BoxcarVec<Box<dyn DynEq>>,
}

impl Database {
    pub fn new() -> Self {
        Database::new_with_capacity(1000.try_into().unwrap())
    }

    pub fn new_with_capacity(capacity: NonZeroUsize) -> Self {
        let current_epoch = Epoch::new();
        Self {
            current_epoch,
            dependency_stack: DependencyStack::new(),
            params: DashMap::new(),
            derived_nodes: DashMap::new(),
            source_nodes: DashMap::new(),
            access_vec: BoxcarVec::new(),
            derived_node_lru_cache: LruCache::new(capacity),
            derived_node_values: BoxcarVec::new(),
        }
    }

    /// Note: this function is also inlined into [Database::set]
    pub fn increment_epoch(&mut self) -> Epoch {
        self.current_epoch.increment()
    }

    pub(crate) fn contains_param(&self, param_id: ParamId) -> bool {
        self.params.contains_key(&param_id)
    }

    pub(crate) fn get_param<'db>(
        &'db self,
        param_id: ParamId,
    ) -> Option<impl std::ops::Deref<Target = Box<dyn Any>> + 'db> {
        self.params.get(&param_id)
    }

    pub(crate) fn get_derived_node<'db>(
        &'db self,
        derived_node_id: DerivedNodeId,
    ) -> Option<Ref<'db, DerivedNodeId, DerivedNode>> {
        eprintln!("getting {:?}", derived_node_id);
        self.access_vec.push(derived_node_id);
        self.derived_nodes.get(&derived_node_id)
    }

    pub(crate) fn get_derived_node_value(
        &self,
        derived_node_index: DerivedNodeIndex,
    ) -> &Box<dyn DynEq> {
        self.derived_node_values.get(derived_node_index.0).expect(
            "Expected value to exist. This is indicative of a bug in pico, or you did some GC",
        )
    }

    pub(crate) fn get_derived_node_mut<'db>(
        &'db self,
        derived_node_id: DerivedNodeId,
    ) -> Option<impl std::ops::DerefMut<Target = DerivedNode> + 'db> {
        eprintln!("getting {:?}", derived_node_id);
        self.access_vec.push(derived_node_id);
        self.derived_nodes.get_mut(&derived_node_id)
    }

    pub fn garbage_collect(&mut self) {
        for (_, derived_node_id) in self.access_vec.iter() {
            self.derived_node_lru_cache.put(*derived_node_id, ());
        }
        self.access_vec = BoxcarVec::new();

        let old_derived_nodes = std::mem::replace(&mut self.derived_nodes, DashMap::new());
        let mut old_derived_nodes_values =
            std::mem::replace(&mut self.derived_node_values, BoxcarVec::new());
        // Now, self.derived_nodes and self.derived_node_values are the ones we need to modify
        // i.e. the ones that contain the derived nodes that survived garbage collection.

        for (retained_id, _) in self.derived_node_lru_cache.iter() {
            let retained_derived_node = old_derived_nodes.get(retained_id).expect("should exist");
            let existing_retained_derived_index = retained_derived_node.derived_node_index;
            let retained_derived_node_value = std::mem::replace(
                unsafe {
                    old_derived_nodes_values.get_unchecked_mut(existing_retained_derived_index.0)
                },
                Box::new(()),
            );
            let derived_node_index = self
                .derived_node_values
                .push(retained_derived_node_value)
                .into();
            self.derived_nodes.insert(
                *retained_id,
                DerivedNode {
                    dependencies: retained_derived_node.dependencies.clone(),
                    inner_fn: retained_derived_node.inner_fn,
                    derived_node_index,
                    time_updated: retained_derived_node.time_updated,
                    time_verified: retained_derived_node.time_verified,
                },
            );
        }
    }

    pub(crate) fn create_derived_node(
        &self,
        derived_node_id: DerivedNodeId,
        inner_fn: InnerFn,
    ) -> (Epoch, DidRecalculate) {
        eprintln!("create 1");
        let (value, tracked_dependencies) =
            with_dependency_tracking(self, derived_node_id.param_id, inner_fn);
        eprintln!("create 2");
        let derived_node_index = self.derived_node_values.push(value).into();

        let derived_node = DerivedNode {
            dependencies: tracked_dependencies.dependencies,
            inner_fn,
            derived_node_index,
            time_updated: tracked_dependencies.max_time_updated,
            time_verified: self.current_epoch,
        };
        self.derived_nodes.insert(derived_node_id, derived_node);
        eprintln!("create 3");
        (
            tracked_dependencies.max_time_updated,
            DidRecalculate::Recalculated,
        )
    }

    pub fn get<T: Clone + 'static>(&self, id: SourceId<T>) -> T {
        let time_updated = self
            .source_nodes
            .get(&id.key)
            .expect("node should exist. This is indicative of a bug in Pico.")
            .time_updated;
        self.register_dependency_in_parent_memoized_fn(NodeKind::Source(id.key), time_updated);
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
    pub fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        let id = SourceId::new(&source);
        match self.source_nodes.entry(id.key) {
            Entry::Occupied(mut occupied_entry) => {
                let existing_node = occupied_entry.get();
                if !existing_node.value.dyn_eq(&source) {
                    // [Database::set] has been inlined here!
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

    pub fn remove<T>(&mut self, id: SourceId<T>) {
        self.increment_epoch();
        self.source_nodes.remove(&id.key);
    }

    pub(crate) fn register_dependency_in_parent_memoized_fn(
        &self,
        node: NodeKind,
        time_updated: Epoch,
    ) {
        self.dependency_stack.push_if_not_empty(
            Dependency {
                node_to: node,
                time_verified_or_updated: self.current_epoch,
            },
            time_updated,
        );
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new()
    }
}
