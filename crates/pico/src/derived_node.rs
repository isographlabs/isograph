use std::{fmt, hash::Hash};

use crate::{
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
    u64_types::{Key, ParamId},
    Database,
};

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

pub struct DerivedNode {
    pub dependencies: Vec<Dependency>,
    pub inner_fn: InnerFn,
    pub value: Box<dyn DynEq>,
    pub time_updated: Epoch,
    pub time_verified: Epoch,
}

impl fmt::Debug for DerivedNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DerivedNode")
            .field("dependencies", &self.dependencies)
            .field("value", &self.value)
            .finish()
    }
}
