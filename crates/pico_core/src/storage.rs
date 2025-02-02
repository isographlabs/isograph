use std::any::Any;

use dashmap::DashMap;
use once_map::OnceMap;

use crate::{
    database::Database,
    dependency_stack::DependencyStack,
    epoch::Epoch,
    generation::Generation,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, DerivedNodeRevision, SourceNode},
    params::ParamId,
};

#[derive(Debug)]
pub struct Storage<Db: Database + ?Sized> {
    pub current_epoch: Epoch,
    pub dependency_stack: DependencyStack<(Epoch, Dependency)>,
    pub params_map: DashMap<ParamId, Index>,
    pub derived_nodes: DerivedNodesStore,
    pub source_nodes: DashMap<Key, SourceNode>,
    pub capacity: usize,
    pub map: OnceMap<Epoch, Box<Generation<Db>>>,
}

impl<Db: Database + ?Sized> Storage<Db> {
    pub fn new(capacity: usize) -> Self {
        let map = OnceMap::new();
        map.insert(Epoch::new(), |_| Box::new(Generation::new()));
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: DependencyStack::new(),
            params_map: DashMap::default(),
            derived_nodes: DerivedNodesStore::default(),
            source_nodes: DashMap::default(),
            capacity,
            map,
        }
    }
}

impl<Db: Database + ?Sized> Storage<Db> {
    pub fn increment_epoch(&mut self) -> Epoch {
        let current_epoch = self.current_epoch.increment();
        self.map
            .insert(current_epoch, |_| Box::new(Generation::new()));
        // TODO: not the best way to do it, worth to create something like OnceOrderedMap
        if self.map.read_only_view().len() >= self.capacity {
            let min_epoch = *self.map.read_only_view().keys().min().unwrap();
            self.map.remove(&min_epoch);
        }
        current_epoch
    }

    pub fn contains_param(&self, param_id: ParamId) -> bool {
        if let Some(index) = self.params_map.get(&param_id) {
            self.map.contains_key(&index.epoch)
        } else {
            false
        }
    }

    pub fn insert_param<T: Clone + 'static>(&self, param_id: ParamId, param: T) {
        let idx = self
            .map
            .get(&self.current_epoch)
            .unwrap()
            .params
            .push(Box::new(param));
        self.params_map
            .insert(param_id, Index::new(self.current_epoch, idx));
    }

    pub fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.params_map.get(&param_id)?;
        Some(self.map.get(&index.epoch)?.params.get(index.idx))
    }

    pub fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode<Db>> {
        let index = self.derived_nodes.map.get(&derived_node_id)?;
        Some(self.map.get(&index.epoch)?.derived_nodes.get(index.idx))
    }

    pub fn set_derive_node_time_updated(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
    ) {
        let mut rev = self.derived_nodes.rev.get_mut(&derived_node_id).unwrap();
        rev.time_updated = time_updated;
    }

    pub fn verify_derived_node(&self, derived_node_id: DerivedNodeId) {
        let mut rev = self.derived_nodes.rev.get_mut(&derived_node_id).unwrap();
        rev.time_verified = self.current_epoch;
    }

    pub fn get_derived_node_rev(
        &self,
        derived_node_id: DerivedNodeId,
    ) -> Option<DerivedNodeRevision> {
        self.derived_nodes.rev.get(&derived_node_id).map(|rev| *rev)
    }

    pub fn insert_derived_node_rev(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
        time_verified: Epoch,
    ) {
        self.derived_nodes.rev.insert(
            derived_node_id,
            DerivedNodeRevision {
                time_updated,
                time_verified,
            },
        );
    }

    pub fn insert_derived_node(
        &self,
        derived_node_id: DerivedNodeId,
        derived_node: DerivedNode<Db>,
    ) {
        let idx = self
            .map
            .get(&self.current_epoch)
            .unwrap()
            .derived_nodes
            .push(derived_node);
        self.derived_nodes
            .map
            .insert(derived_node_id, Index::new(self.current_epoch, idx));
    }
}

impl<Db: Database + ?Sized> Default for Storage<Db> {
    fn default() -> Self {
        Self::new(10000)
    }
}

#[derive(Debug, Default)]
pub struct DerivedNodesStore {
    pub map: DashMap<DerivedNodeId, Index>,
    pub rev: DashMap<DerivedNodeId, DerivedNodeRevision>,
}

#[derive(Debug, Clone, Copy)]
pub struct Index {
    pub epoch: Epoch,
    pub idx: usize,
}

impl Index {
    pub fn new(epoch: Epoch, idx: usize) -> Self {
        Self { epoch, idx }
    }
}
