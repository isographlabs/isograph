use std::{any::Any, fmt::Debug};

use pico_macros::Storage;

use pico_core::{
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, NodeId, SourceNode},
    params::ParamId,
};

use crate::container::DefaultContainer;

#[derive(Debug, Storage)]
pub struct DefaultStorage<Db: Database> {
    pub current_epoch: Epoch,
    pub dependency_stack: Vec<Vec<(Epoch, Dependency)>>,
    pub nodes: DefaultContainer<NodeId, DerivedNode<Db>>,
    pub values: DefaultContainer<NodeId, Box<dyn DynEq>>,
    pub sources: DefaultContainer<Key, SourceNode>,
    pub source_values: DefaultContainer<Key, Box<dyn DynEq>>,
    pub params: DefaultContainer<ParamId, Box<dyn Any>>,
}

impl<Db: Database> DefaultStorage<Db> {
    pub fn new() -> Self {
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: vec![],
            nodes: DefaultContainer::new(),
            sources: DefaultContainer::new(),
            params: DefaultContainer::new(),
            values: DefaultContainer::new(),
            source_values: DefaultContainer::new(),
        }
    }
}

impl<Db: Database> Default for DefaultStorage<Db> {
    fn default() -> Self {
        Self::new()
    }
}
