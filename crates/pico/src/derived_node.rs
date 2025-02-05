use std::{fmt, hash::Hash};

use crate::{
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
    u64_types::{Key, ParamId},
    InnerFn,
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

pub struct DerivedNode {
    pub dependencies: Vec<Dependency>,
    pub inner_fn: InnerFn,
    pub value: Box<dyn DynEq>,
}

impl fmt::Debug for DerivedNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DerivedNode")
            .field("dependencies", &self.dependencies)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DerivedNodeRevision {
    pub time_updated: Epoch,
    pub time_verified: Epoch,
}
