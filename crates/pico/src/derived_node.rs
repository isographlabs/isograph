use std::{fmt, hash::Hash};

use crate::{
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
    u64_types::{Key, ParamId},
    Database,
};

pub type DerivedNodeValue = Box<dyn DynEq>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DerivedNodeId {
    pub key: Key,
    pub param_id: ParamId,
}

impl DerivedNodeId {
    pub fn new(key: Key, param_id: ParamId) -> Self {
        Self { key, param_id }
    }
}

#[derive(Debug, Copy, Clone)]
pub struct InnerFn(pub fn(&Database, ParamId) -> Box<dyn DynEq>);
impl InnerFn {
    pub fn new(inner_fn: fn(&Database, ParamId) -> Box<dyn DynEq>) -> Self {
        InnerFn(inner_fn)
    }
}

// TODO every time GC is run, derived node indexes get invalidated,
// so we should keep track of how many times we had run GC (e.g. a
// GC Generation)
#[derive(Debug, Clone, Copy)]
pub struct DerivedNodeIndex(pub usize);

impl From<usize> for DerivedNodeIndex {
    fn from(value: usize) -> Self {
        Self(value)
    }
}

#[derive(Debug)]
pub struct DerivedNode {
    pub dependencies: Vec<Dependency>,
    pub inner_fn: InnerFn,
    pub derived_node_index: DerivedNodeIndex,
    pub time_updated: Epoch,
    pub time_verified: Epoch,
}
