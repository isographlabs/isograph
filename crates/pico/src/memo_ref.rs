use std::{marker::PhantomData, ops::Deref};

use intern::InternId;

use crate::{Database, DerivedNodeId, ParamId};

#[derive(Debug)]
pub struct MemoRef<T, Db: Database> {
    pub(crate) db: *const Db,
    pub(crate) derived_node_id: DerivedNodeId,
    phantom: PhantomData<T>,
}

impl<T, Db: Database> Copy for MemoRef<T, Db> {}

impl<T, Db: Database> Clone for MemoRef<T, Db> {
    fn clone(&self) -> Self {
        *self
    }
}

impl<T, Db: Database> PartialEq for MemoRef<T, Db> {
    fn eq(&self, other: &Self) -> bool {
        self.db == other.db && self.derived_node_id == other.derived_node_id
    }
}

impl<T, Db: Database> Eq for MemoRef<T, Db> {}

impl<T: 'static + Clone, Db: Database> MemoRef<T, Db> {
    pub fn new(db: &Db, derived_node_id: DerivedNodeId) -> Self {
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

impl<T, Db: Database> From<MemoRef<T, Db>> for ParamId {
    fn from(val: MemoRef<T, Db>) -> Self {
        let idx: u64 = val.derived_node_id.index().into();
        ParamId::from(idx)
    }
}

impl<T: 'static, Db: Database> Deref for MemoRef<T, Db> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        // SAFETY: `db` must outlive `MemoRef`
        let db = unsafe { &*self.db };
        db.get_storage()
            .internal
            .get_derived_node(self.derived_node_id)
            .unwrap()
            .value
            .as_any()
            .downcast_ref::<T>()
            .unwrap()
    }
}
