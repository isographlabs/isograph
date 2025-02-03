use std::any::Any;

use dashmap::DashMap;
use once_map::OnceMap;

use crate::{
    dependency_stack::DependencyStack,
    dyn_eq::DynEq,
    epoch::Epoch,
    generation::Generation,
    index::Index,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, DerivedNodeRevision, NodeKind, SourceNode},
    params::ParamId,
    source::{Source, SourceId},
};

#[derive(Debug)]
pub struct Database {
    pub current_epoch: Epoch,
    pub dependency_stack: DependencyStack<(Epoch, Dependency)>,
    pub param_id_to_index: DashMap<ParamId, Index>,
    pub derived_nodes: DerivedNodesStore,
    pub source_nodes: DashMap<Key, SourceNode>,
    pub capacity: usize,
    pub epoch_to_generation_map: OnceMap<Epoch, Box<Generation>>,
}

impl Database {
    pub fn new(capacity: usize) -> Self {
        let epoch_to_generation_map = OnceMap::new();
        epoch_to_generation_map.insert(Epoch::new(), |_| Box::new(Generation::new()));
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: DependencyStack::new(),
            param_id_to_index: DashMap::default(),
            derived_nodes: DerivedNodesStore::default(),
            source_nodes: DashMap::default(),
            capacity,
            epoch_to_generation_map,
        }
    }

    pub fn increment_epoch(&mut self) -> Epoch {
        let current_epoch = self.current_epoch.increment();
        self.epoch_to_generation_map
            .insert(current_epoch, |_| Box::new(Generation::new()));
        // TODO: not the best way to do it, worth to create something like OnceOrderedMap
        if self.epoch_to_generation_map.read_only_view().len() >= self.capacity {
            let min_epoch = *self
                .epoch_to_generation_map
                .read_only_view()
                .keys()
                .min()
                .unwrap();
            self.epoch_to_generation_map.remove(&min_epoch);
        }
        current_epoch
    }

    pub fn contains_param(&self, param_id: ParamId) -> bool {
        if let Some(index) = self.param_id_to_index.get(&param_id) {
            self.epoch_to_generation_map.contains_key(&index.epoch)
        } else {
            false
        }
    }

    pub fn insert_param<T: Clone + 'static>(&self, param_id: ParamId, param: T) {
        let idx = self
            .epoch_to_generation_map
            .get(&self.current_epoch)
            .unwrap()
            .params
            .push(Box::new(param));
        self.param_id_to_index
            .insert(param_id, Index::new(self.current_epoch, idx));
    }

    pub fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(
            self.epoch_to_generation_map
                .get(&index.epoch)?
                .params
                .get(index.idx),
        )
    }

    pub fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
        let index = self.derived_nodes.id_to_index.get(&derived_node_id)?;
        Some(
            self.epoch_to_generation_map
                .get(&index.epoch)?
                .derived_nodes
                .get(index.idx),
        )
    }

    pub fn set_derive_node_time_updated(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
    ) {
        let mut rev = self
            .derived_nodes
            .id_to_revision
            .get_mut(&derived_node_id)
            .unwrap();
        rev.time_updated = time_updated;
    }

    pub fn verify_derived_node(&self, derived_node_id: DerivedNodeId) {
        let mut rev = self
            .derived_nodes
            .id_to_revision
            .get_mut(&derived_node_id)
            .unwrap();
        rev.time_verified = self.current_epoch;
    }

    pub fn get_derived_node_rev(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<DerivedNodeRevision> {
        self.derived_nodes
            .id_to_revision
            .get(&derived_node_id)
            .map(|rev| *rev)
    }

    pub fn insert_derived_node_rev(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
        time_verified: Epoch,
    ) {
        self.derived_nodes.id_to_revision.insert(
            derived_node_id,
            DerivedNodeRevision {
                time_updated,
                time_verified,
            },
        );
    }

    pub fn insert_derived_node(&self, derived_node_id: DerivedNodeId, derived_node: DerivedNode) {
        let idx = self
            .epoch_to_generation_map
            .get(&self.current_epoch)
            .unwrap()
            .derived_nodes
            .push(derived_node);
        self.derived_nodes
            .id_to_index
            .insert(derived_node_id, Index::new(self.current_epoch, idx));
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

    pub fn set<T: Source + DynEq>(&mut self, source: T) -> SourceId<T> {
        let id = SourceId::new(&source);
        let time_updated = if self.source_nodes.contains_key(&id.key) {
            self.increment_epoch()
        } else {
            self.current_epoch
        };
        self.source_nodes.insert(
            id.key,
            SourceNode {
                time_updated,
                value: Box::new(source),
            },
        );
        id
    }

    pub fn remove<T>(&mut self, id: SourceId<T>) {
        self.increment_epoch();
        self.source_nodes.remove(&id.key);
    }

    pub fn register_dependency_in_parent_memoized_fn(&self, node: NodeKind, time_updated: Epoch) {
        self.dependency_stack.push_if_not_empty(|| {
            (
                time_updated,
                Dependency {
                    node_to: node,
                    time_verified_or_updated: self.current_epoch,
                },
            )
        });
    }
}

impl Default for Database {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[derive(Debug, Default)]
pub struct DerivedNodesStore {
    pub id_to_index: DashMap<DerivedNodeId, Index>,
    pub id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
}
