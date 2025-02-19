use std::{fmt, hash::Hash, marker::PhantomData, ops::Deref};

use intern::{intern_struct, InternId};
use serde::{Deserialize, Serialize};
use tinyvec::ArrayVec;

use crate::{
    dependency::Dependency,
    dyn_eq::DynEq,
    epoch::{Epoch, GcEpoch},
    index::Index,
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
pub struct InnerFn(pub fn(&Database, DerivedNodeId) -> Option<Box<dyn DynEq>>);
impl InnerFn {
    pub fn new(inner_fn: fn(&Database, DerivedNodeId) -> Option<Box<dyn DynEq>>) -> Self {
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
    pub index: Index<DerivedNodeId>,
}

#[derive(Debug, Clone, Copy)]
pub struct MemoRef<'db, T> {
    db: &'db Database,
    derived_node_id: DerivedNodeId,
    phantom: PhantomData<T>,
    valid_during_gc_epoch: GcEpoch,
}

impl<'db, T: 'static + Clone> MemoRef<'db, T> {
    pub fn new(db: &'db Database, derived_node_id: DerivedNodeId) -> Self {
        Self {
            db,
            derived_node_id,
            phantom: PhantomData,
            valid_during_gc_epoch: db.gc_epoch,
        }
    }

    pub fn to_owned(&self) -> T {
        self.deref().clone()
    }
}

impl<T> From<MemoRef<'_, T>> for ParamId {
    fn from(val: MemoRef<'_, T>) -> Self {
        let idx: u64 = val.derived_node_id.index().into();
        ParamId::from(idx)
    }
}

impl<T: 'static> Deref for MemoRef<'_, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        if self.db.gc_epoch != self.valid_during_gc_epoch {
            panic!(
                "Attempted to dereference MemoRef after garbage collection. \
                This is disallowed."
            )
        }

        self.db
            .get_derived_node(self.derived_node_id)
            .unwrap()
            .value
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }
}
