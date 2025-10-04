use std::{
    any::Any,
    hash::{Hash, Hasher},
    marker::PhantomData,
    ops::Deref,
};

use crate::{DatabaseDyn, DerivedNodeId, ParamId, dependency::NodeKind};
use intern::InternId;

type MemoRefProjectorStep = for<'a> fn(&'a dyn Any) -> &'a dyn Any;

const MAX_PROJECTOR_STEPS: usize = 4;

#[inline(always)]
fn step_identity(value: &dyn Any) -> &dyn Any {
    value
}

#[inline(always)]
fn step_result_ok<T: 'static, E: 'static>(value: &dyn Any) -> &dyn Any {
    match value
        .downcast_ref::<Result<T, E>>()
        .expect("MemoRef<Result<..>>: underlying value has unexpected type")
    {
        Ok(t) => t as &dyn Any,
        Err(_) => unreachable!("Ok projection used only after Ok check"),
    }
}

#[inline(always)]
fn step_option_some<T: 'static>(value: &dyn Any) -> &dyn Any {
    match value
        .downcast_ref::<Option<T>>()
        .expect("MemoRef<Option<..>>: underlying value has unexpected type")
    {
        Some(t) => t as &dyn Any,
        None => unreachable!("Some projection used only after Some check"),
    }
}

#[inline(always)]
fn step_tuple_0<T0: 'static, T1: 'static>(value: &dyn Any) -> &dyn Any {
    let (t0, _) = value
        .downcast_ref::<(T0, T1)>()
        .expect("MemoRef<(..)>: underlying value has unexpected type");
    t0 as &dyn Any
}

#[inline(always)]
fn step_tuple_1<T0: 'static, T1: 'static>(value: &dyn Any) -> &dyn Any {
    let (_, t1) = value
        .downcast_ref::<(T0, T1)>()
        .expect("MemoRef<(..)>: underlying value has unexpected type");
    t1 as &dyn Any
}

#[derive(Debug)]
pub struct MemoRef<T> {
    pub(crate) db: *const dyn DatabaseDyn,
    pub(crate) derived_node_id: DerivedNodeId,
    projectors: [MemoRefProjectorStep; MAX_PROJECTOR_STEPS],
    projectors_len: u8,
    phantom: PhantomData<T>,
}

impl<T> Copy for MemoRef<T> {}

impl<T> Clone for MemoRef<T> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T> PartialEq for MemoRef<T> {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.db, other.db) && self.derived_node_id == other.derived_node_id
    }
}

impl<T> Eq for MemoRef<T> {}

#[allow(clippy::unnecessary_cast)]
impl<T> Hash for MemoRef<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        let data_ptr = self.db as *const dyn DatabaseDyn as *const ();
        data_ptr.hash(state);
        self.derived_node_id.hash(state);
    }
}

#[allow(clippy::unnecessary_cast)]
impl<T: 'static + Clone> MemoRef<T> {
    pub fn new(db: &dyn DatabaseDyn, derived_node_id: DerivedNodeId) -> Self {
        Self {
            db: db as *const _ as *const dyn DatabaseDyn,
            derived_node_id,
            projectors: [step_identity; MAX_PROJECTOR_STEPS],
            projectors_len: 0,
            phantom: PhantomData,
        }
    }

    pub fn to_owned(&self) -> T {
        self.deref().clone()
    }
}

impl<T> From<MemoRef<T>> for ParamId {
    fn from(val: MemoRef<T>) -> Self {
        let idx: u64 = val.derived_node_id.index().into();
        ParamId::from(idx)
    }
}

impl<T: 'static> Deref for MemoRef<T> {
    type Target = T;

    fn deref(&self) -> &T {
        // SAFETY: Database outlives this MemoRef
        let db = unsafe { &*self.db };
        let storage = db.get_storage_dyn();
        let (value, revision) = storage
            .get_derived_node_value_and_revision(self.derived_node_id)
            .unwrap();
        storage.register_dependency_in_parent_memoized_fn(
            NodeKind::Derived(self.derived_node_id),
            revision.time_updated,
        );
        if self.projectors_len == 0 {
            return value
                .downcast_ref::<T>()
                .expect("MemoRef: projector chain produced unexpected type");
        }
        let mut any_ref = value;
        let len = self.projectors_len as usize;
        for step in &self.projectors[..len] {
            any_ref = (step)(any_ref);
        }
        any_ref
            .downcast_ref::<T>()
            .expect("MemoRef: projector chain produced unexpected type")
    }
}

impl<T: 'static, E: 'static + Clone> MemoRef<Result<T, E>> {
    pub fn try_ok(self) -> Result<MemoRef<T>, E> {
        match self.deref() {
            Ok(_) => {
                let mut next = MemoRef::<T> {
                    db: self.db,
                    derived_node_id: self.derived_node_id,
                    projectors: self.projectors,
                    projectors_len: self.projectors_len + 1,
                    phantom: PhantomData,
                };
                let idx = self.projectors_len as usize;
                next.projectors[idx] = step_result_ok::<T, E>;
                Ok(next)
            }
            Err(err) => Err(err.clone()),
        }
    }
}

impl<T: 'static> MemoRef<Option<T>> {
    pub fn try_some(self) -> Option<MemoRef<T>> {
        match self.deref() {
            Some(_) => {
                let mut next = MemoRef::<T> {
                    db: self.db,
                    derived_node_id: self.derived_node_id,
                    projectors: self.projectors,
                    projectors_len: self.projectors_len + 1,
                    phantom: PhantomData,
                };
                let idx = self.projectors_len as usize;
                next.projectors[idx] = step_option_some::<T>;
                Some(next)
            }
            None => None,
        }
    }
}

impl<T0: 'static, T1: 'static> MemoRef<(T0, T1)> {
    pub fn split(self) -> (MemoRef<T0>, MemoRef<T1>) {
        let mut left = MemoRef::<T0> {
            db: self.db,
            derived_node_id: self.derived_node_id,
            projectors: self.projectors,
            projectors_len: self.projectors_len + 1,
            phantom: PhantomData,
        };
        let idx = self.projectors_len as usize;
        left.projectors[idx] = step_tuple_0::<T0, T1>;

        let mut right = MemoRef::<T1> {
            db: self.db,
            derived_node_id: self.derived_node_id,
            projectors: self.projectors,
            projectors_len: self.projectors_len + 1,
            phantom: PhantomData,
        };
        let idx = self.projectors_len as usize;
        right.projectors[idx] = step_tuple_1::<T0, T1>;

        (left, right)
    }
}
