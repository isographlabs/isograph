use std::{marker::PhantomData, ops::Deref};

use intern::InternId;

use crate::{DatabaseDyn, DerivedNodeId, ParamId, dependency::NodeKind};

#[derive(Debug)]
pub struct MemoRef<T> {
    pub(crate) db: *const dyn DatabaseDyn,
    pub(crate) derived_node_id: DerivedNodeId,
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
impl<T: 'static + Clone> MemoRef<T> {
    pub fn new(db: &dyn DatabaseDyn, derived_node_id: DerivedNodeId) -> Self {
        Self {
            db: db as *const _ as *const dyn DatabaseDyn,
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
        value.downcast_ref::<T>().unwrap()
    }
}
