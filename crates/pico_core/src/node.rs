use std::fmt;
use std::hash::Hash;

use crate::database::Database;
use crate::dyn_eq::DynEq;
use crate::epoch::Epoch;
use crate::key::Key;
use crate::params::ParamId;

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

pub struct DerivedNode<Db: Database + ?Sized> {
    pub time_verified: Epoch,
    pub time_calculated: Epoch,
    pub dependencies: Vec<Dependency>,
    pub inner_fn: fn(&mut Db, ParamId) -> Box<dyn DynEq>,
    pub value: Box<dyn DynEq>,
}

impl<Db: Database> fmt::Debug for DerivedNode<Db> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DerivedNode")
            .field("time_verified", &self.time_verified)
            .field("time_calculated", &self.time_calculated)
            .field("dependencies", &self.dependencies)
            .field("value", &self.value)
            .finish()
    }
}

#[derive(Debug)]
pub struct SourceNode {
    pub time_calculated: Epoch,
    pub value: Box<dyn DynEq>,
}

#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    pub node_to: NodeKind,
    pub time_verified_or_calculated: Epoch,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Source(Key),
    Derived(DerivedNodeId),
}