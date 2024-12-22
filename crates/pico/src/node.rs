use std::fmt;

use crate::params::ParamId;
use crate::{database::Database, dyn_eq::DynEq};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Source,
    Derived,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId {
    key: &'static str,
    pub kind: NodeKind,
    pub param_id: ParamId,
}

impl NodeId {
    pub fn new(kind: NodeKind, key: &'static str, param_id: ParamId) -> Self {
        Self {
            key,
            kind,
            param_id,
        }
    }

    pub fn source(key: &'static str, param_id: ParamId) -> Self {
        Self::new(NodeKind::Source, key, param_id)
    }

    pub fn derived(key: &'static str, param_id: ParamId) -> Self {
        Self::new(NodeKind::Derived, key, param_id)
    }
}

pub struct DerivedNode {
    pub time_verified: u64,
    pub time_calculated: u64,
    pub dependencies: Vec<Dependency>,
    pub inner_fn: fn(&mut Database, ParamId) -> Box<dyn DynEq>,
}

impl DerivedNode {
    pub fn call_inner_fn(&self, db: &mut Database, param_id: ParamId) -> Box<dyn DynEq> {
        (self.inner_fn)(db, param_id)
    }
}

impl fmt::Debug for DerivedNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("DerivedNode")
            .field("time_verified", &self.time_verified)
            .field("time_calculated", &self.time_calculated)
            .field("dependencies", &self.dependencies)
            .finish()
    }
}

pub struct SourceNode {
    pub time_calculated: u64,
}

impl fmt::Debug for SourceNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("SourceNode")
            .field("time_calculated", &self.time_calculated)
            .finish()
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Dependency {
    pub node_to: NodeId,
    pub time_verified_or_calculated: u64,
}
