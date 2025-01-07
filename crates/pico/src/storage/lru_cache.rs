use std::{any::Any, fmt::Debug};

use pico_macros::Storage;

use pico_core::{
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    node::{Dependency, DerivedNode, NodeId, SourceNode},
    params::ParamId,
    source::SourceKey,
};

use crate::container::LruCacheContainer;

#[derive(Debug, Storage)]
pub struct LruCacheStorage<Db: Database> {
    pub current_epoch: Epoch,
    pub dependency_stack: Vec<Vec<(Epoch, Dependency)>>,
    pub nodes: LruCacheContainer<NodeId, DerivedNode<Db>>,
    pub values: LruCacheContainer<NodeId, Box<dyn DynEq>>,
    pub sources: LruCacheContainer<SourceKey, SourceNode>,
    pub source_values: LruCacheContainer<SourceKey, Box<dyn DynEq>>,
    pub params: LruCacheContainer<ParamId, Box<dyn Any>>,
}

impl<Db: Database> LruCacheStorage<Db> {
    pub fn new(cache_size: usize) -> Self {
        Self {
            current_epoch: Epoch::new(),
            dependency_stack: vec![],
            nodes: LruCacheContainer::new(cache_size),
            sources: LruCacheContainer::new(cache_size),
            params: LruCacheContainer::new(cache_size),
            values: LruCacheContainer::new(cache_size),
            source_values: LruCacheContainer::new(cache_size),
        }
    }
}

impl<Db: Database> Default for LruCacheStorage<Db> {
    fn default() -> Self {
        Self::new(10000)
    }
}
