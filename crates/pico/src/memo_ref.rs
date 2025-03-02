use std::{marker::PhantomData, ops::Deref};

use intern::InternId;

use crate::{Database, DerivedNodeId, ParamId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MemoRef<T> {
    pub(crate) db: *const Database,
    pub(crate) derived_node_id: DerivedNodeId,
    phantom: PhantomData<T>,
}

impl<T: 'static + Clone> MemoRef<T> {
    pub fn new(db: &Database, derived_node_id: DerivedNodeId) -> Self {
        Self {
            db,
            derived_node_id,
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

    fn deref(&self) -> &Self::Target {
        // SAFETY: `db` must outlive `MemoRef`
        let db = unsafe { &*self.db };
        db.storage
            .get_derived_node(self.derived_node_id)
            .unwrap()
            .value
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }
}
