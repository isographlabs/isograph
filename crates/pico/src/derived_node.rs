use std::{fmt, hash::Hash};

use intern::{InternId, intern_struct};
use serde::{Deserialize, Serialize};
use tinyvec::ArrayVec;

use crate::{
    Database,
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
    index::Index,
    intern::{Key, ParamId},
};

#[derive(Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub struct DerivedNodeDescriptor {
    pub key: Key,
    pub params: ArrayVec<[ParamId; 8]>,
}

intern_struct! {
    pub struct DerivedNodeId = Intern<DerivedNodeDescriptor> {}
}

impl DerivedNodeId {
    pub fn new(key: Key, params: ArrayVec<[ParamId; 8]>) -> Self {
        Self::intern(DerivedNodeDescriptor { key, params })
    }
}

impl From<ParamId> for DerivedNodeId {
    fn from(value: ParamId) -> Self {
        DerivedNodeId::from_index_checked(**value.inner() as u32).unwrap()
    }
}

#[derive(Debug)]
pub struct InnerFn<Db: Database>(pub fn(&Db, DerivedNodeId) -> Option<Box<dyn DynEq>>);

impl<Db: Database> InnerFn<Db> {
    pub fn new(inner_fn: fn(&Db, DerivedNodeId) -> Option<Box<dyn DynEq>>) -> Self {
        InnerFn(inner_fn)
    }
}

impl<Db: Database> Clone for InnerFn<Db> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<Db: Database> Copy for InnerFn<Db> {}

pub struct DerivedNode<Db: Database> {
    pub dependencies: Vec<Dependency>,
    pub inner_fn: InnerFn<Db>,
    pub value: Box<dyn DynEq>,
}

impl<Db: Database> fmt::Debug for DerivedNode<Db> {
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
    pub index: Index<DerivedNodeId>,
}
