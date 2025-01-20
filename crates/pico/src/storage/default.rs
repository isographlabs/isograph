use std::{any::Any, fmt::Debug};

use pico_macros::Storage;

use pico_core::{
    database::Database,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, SourceNode},
    params::ParamId,
};

use crate::container::DefaultContainer;

#[derive(Debug, Storage)]
pub struct DefaultStorage<Db: Database> {
    pub current_epoch: Epoch,
    pub dependency_stack: Vec<Vec<(Epoch, Dependency)>>,
    pub derived_nodes: DefaultContainer<DerivedNodeId, DerivedNode<Db>>,
    pub source_nodes: DefaultContainer<Key, SourceNode>,
    pub params: DefaultContainer<ParamId, Box<dyn Any>>,
}

impl<Db: Database> DefaultStorage<Db> {
    pub fn new() -> Self {
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: vec![],
            derived_nodes: DefaultContainer::new(),
            source_nodes: DefaultContainer::new(),
            params: DefaultContainer::new(),
        }
    }
}

impl<Db: Database> Default for DefaultStorage<Db> {
    fn default() -> Self {
        Self::new()
    }
}
