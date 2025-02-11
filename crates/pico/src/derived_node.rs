use std::{fmt, hash::Hash, marker::PhantomData, ops::Deref};

use intern::{intern_struct, InternId};
use serde::{Deserialize, Serialize};
use tinyvec::ArrayVec;

use crate::{
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::Epoch,
    intern::{Key, ParamId},
    Database,
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

#[derive(Debug, Copy, Clone)]
pub struct InnerFn(pub fn(&Database, DerivedNodeId) -> Box<dyn DynEq>);
impl InnerFn {
    pub fn new(inner_fn: fn(&Database, DerivedNodeId) -> Box<dyn DynEq>) -> Self {
        InnerFn(inner_fn)
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

#[derive(Debug, Clone, Copy)]
pub struct MemoRef<'db, T> {
    db: &'db Database,
    derived_node_id: DerivedNodeId,
    phantom: PhantomData<T>,
}

impl<'db, T> MemoRef<'db, T> {
    pub fn new(db: &'db Database, derived_node_id: DerivedNodeId) -> Self {
        Self {
            db,
            derived_node_id,
            phantom: PhantomData,
        }
    }
}

impl<T: 'static + Clone> MemoRef<'_, T> {
    pub fn to_owned(&self) -> T {
        self.deref().clone()
    }
}

impl<T: 'static> Deref for MemoRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        self.db
            .get_derived_node(self.derived_node_id)
            .unwrap()
            .value
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }
}
