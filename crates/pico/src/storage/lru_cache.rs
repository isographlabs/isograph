use std::{any::Any, fmt::Debug};

use pico_macros::Storage;

use pico_core::{
    database::Database,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, SourceNode},
    params::ParamId,
};

use crate::container::LruCacheContainer;

#[derive(Debug, Storage)]
pub struct LruCacheStorage<Db: Database> {
    pub current_epoch: Epoch,
    pub dependency_stack: Vec<Vec<(Epoch, Dependency)>>,
    pub derived_nodes: LruCacheContainer<DerivedNodeId, DerivedNode<Db>>,
    pub source_nodes: LruCacheContainer<Key, SourceNode>,
    pub params: LruCacheContainer<ParamId, Box<dyn Any>>,
}

impl<Db: Database> LruCacheStorage<Db> {
    pub fn new(cache_size: usize) -> Self {
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: vec![],
            derived_nodes: LruCacheContainer::new(cache_size),
            source_nodes: LruCacheContainer::new(cache_size),
            params: LruCacheContainer::new(cache_size),
        }
    }
}

impl<Db: Database> Default for LruCacheStorage<Db> {
    fn default() -> Self {
        Self::new(10000)
    }
}
