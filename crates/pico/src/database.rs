use std::any::Any;

use crate::{
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::Epoch,
    index::Index,
    source::{Source, SourceId, SourceNode},
    u64_types::{Key, ParamId},
};
use dashmap::{DashMap, Entry};
use once_map::OnceMap;

use crate::{
    derived_node::{DerivedNode, DerivedNodeId, DerivedNodeRevision},
    generation::Generation,
};

#[derive(Debug)]
pub struct Database {
    pub(crate) dependency_stack: DependencyStack,
    pub(crate) param_id_to_index: DashMap<ParamId, Index<ParamId>>,
    pub(crate) derived_node_id_to_index: DashMap<DerivedNodeId, Index<DerivedNodeId>>,
    pub(crate) derived_node_id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
    pub(crate) source_nodes: DashMap<Key, SourceNode>,

    /// The oldest epoch currently in the epoch to generation map (inclusive)
    pub(crate) earliest_epoch: Epoch,
    pub(crate) current_epoch: Epoch,
    pub(crate) epoch_to_generation_map: OnceMap<Epoch, Box<Generation>>,
}

impl Database {
    pub fn new() -> Self {
        let epoch_to_generation_map = OnceMap::new();
        let current_epoch = Epoch::new();
        epoch_to_generation_map.insert(current_epoch, |_| Box::new(Generation::new()));
        Self {
            current_epoch,
            earliest_epoch: current_epoch,
            dependency_stack: DependencyStack::new(),
            param_id_to_index: DashMap::new(),
            derived_node_id_to_index: DashMap::new(),
            derived_node_id_to_revision: DashMap::new(),
            source_nodes: DashMap::new(),
            epoch_to_generation_map,
        }
    }

    /// Note: this function is also inlined into [Database::set]
    pub fn increment_epoch(&mut self) -> Epoch {
        let next_epoch = self.current_epoch.increment();
        self.epoch_to_generation_map
            .insert(next_epoch, |_| Box::new(Generation::new()));
        next_epoch
    }

    /// Drop epochs until first_epoch_to_keep.
    pub fn drop_epochs(&mut self, first_epoch_to_keep: Epoch) {
        debug_assert!(
            first_epoch_to_keep < self.current_epoch,
            "Cannot drop the current epoch."
        );

        if first_epoch_to_keep < self.earliest_epoch {
            return;
        }

        for epoch_to_drop in self.earliest_epoch.to(first_epoch_to_keep) {
            self.epoch_to_generation_map.remove(&epoch_to_drop);
        }
        self.earliest_epoch = first_epoch_to_keep;
    }

    pub(crate) fn contains_param(&self, param_id: ParamId) -> bool {
        if let Some(index) = self.param_id_to_index.get(&param_id) {
            self.epoch_to_generation_map.contains_key(&index.epoch)
        } else {
            false
        }
    }

    pub(crate) fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(
            self.epoch_to_generation_map
                .get(&index.epoch)?
                .get_param(*index),
        )
    }

    pub(crate) fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
        let index = self.derived_node_id_to_index.get(&derived_node_id)?;
        Some(
            self.epoch_to_generation_map
                .get(&index.epoch)?
                .get_derived_node(*index),
        )
    }

    pub(crate) fn set_derive_node_time_updated(
        &self,
        derived_node_id: DerivedNodeId,
        time_updated: Epoch,
    ) {
        let mut rev = self
            .derived_node_id_to_revision
            .get_mut(&derived_node_id)
            .unwrap();
        rev.time_updated = time_updated;
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
    ) {
        self.derived_node_id_to_revision.insert(
            derived_node_id,
            DerivedNodeRevision {
                time_updated,
                time_verified,
            },
        );
    }

    pub(crate) fn insert_derived_node(
        &self,
        derived_node_id: DerivedNodeId,
        derived_node: DerivedNode,
    ) {
        let idx = self
            .epoch_to_generation_map
            .get(&self.current_epoch)
            .unwrap()
            .insert_derived_node(derived_node);
        self.derived_node_id_to_index
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
                    self.epoch_to_generation_map
                        .insert(next_epoch, |_| Box::new(Generation::new()));
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
