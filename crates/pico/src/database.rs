use std::any::Any;

use crate::{
    dependency::{Dependency, DependencyStack, NodeKind},
    dyn_eq::DynEq,
    epoch::{Epoch, GcEpoch},
    index::Index,
    intern::{Key, ParamId},
    source::{Source, SourceId, SourceNode},
};
use boxcar::Vec as BoxcarVec;
use dashmap::{DashMap, Entry};

use crate::derived_node::{DerivedNode, DerivedNodeId, DerivedNodeRevision};

#[derive(Debug)]
pub struct Database {
    pub(crate) dependency_stack: DependencyStack,
    pub(crate) param_id_to_index: DashMap<ParamId, Index<ParamId>>,
    pub(crate) derived_node_id_to_index: DashMap<DerivedNodeId, Index<DerivedNodeId>>,
    pub(crate) derived_node_id_to_revision: DashMap<DerivedNodeId, DerivedNodeRevision>,
    pub(crate) source_nodes: DashMap<Key, SourceNode>,

    pub(crate) derived_nodes: BoxcarVec<DerivedNode>,
    pub(crate) params: BoxcarVec<Box<dyn Any>>,
    pub(crate) current_epoch: Epoch,
    pub(crate) gc_epoch: GcEpoch,
}

impl Database {
    pub fn new() -> Self {
        let current_epoch = Epoch::new();
        Self {
            dependency_stack: DependencyStack::new(),
            param_id_to_index: DashMap::new(),
            derived_node_id_to_index: DashMap::new(),
            derived_node_id_to_revision: DashMap::new(),

            source_nodes: DashMap::new(),
            derived_nodes: BoxcarVec::new(),
            params: BoxcarVec::new(),

            current_epoch,
            gc_epoch: GcEpoch::new(),
        }
    }

    pub(crate) fn increment_epoch(&mut self) -> Epoch {
        self.current_epoch.increment()
    }

    pub(crate) fn get_param(&self, param_id: ParamId) -> Option<&Box<dyn Any>> {
        let index = self.param_id_to_index.get(&param_id)?;
        Some(self.params.get(index.idx).expect(
            "indexes should always be valid. \
                This is indicative of a bug in Isograph.",
        ))
    }

    pub(crate) fn get_derived_node(&self, derived_node_id: DerivedNodeId) -> Option<&DerivedNode> {
        let index = self.derived_node_id_to_index.get(&derived_node_id)?;
        Some(self.derived_nodes.get(index.idx).expect(
            "indexes should always be valid. \
                This is indicative of a bug in Isograph.",
        ))
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
        let idx = self.derived_nodes.push(derived_node);
        self.derived_node_id_to_index
            .insert(derived_node_id, Index::new(idx));
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
