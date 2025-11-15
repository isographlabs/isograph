use std::marker::PhantomData;

use intern::InternId;

use crate::{DatabaseDyn, DerivedNodeId, ParamId, RawPtr, dependency::NodeKind};

#[derive(Debug)]
pub struct MemoRef<T> {
    pub(crate) db: *const dyn DatabaseDyn,
    pub(crate) derived_node_id: DerivedNodeId,
    kind: MemoRefKind,
    phantom: PhantomData<T>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MemoRefKind {
    Value,
    RawPtr,
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

#[expect(clippy::unnecessary_cast)]
impl<T: 'static> MemoRef<T> {
    pub fn new(db: &dyn DatabaseDyn, derived_node_id: DerivedNodeId) -> Self {
        Self {
            derived_node_id,
            db: db as *const _ as *const dyn DatabaseDyn,
            kind: MemoRefKind::Value,
            phantom: PhantomData,
        }
    }

    pub(crate) fn new_with_kind(
        db: &dyn DatabaseDyn,
        derived_node_id: DerivedNodeId,
        kind: MemoRefKind,
    ) -> Self {
        Self {
            db: db as *const _ as *const dyn DatabaseDyn,
            derived_node_id,
            kind,
            phantom: PhantomData,
        }
    }

    pub fn lookup_tracked<'db>(&self) -> &'db T {
        // SAFETY: Database outlives this MemoRef
        let db = unsafe { &*self.db };
        let storage = db.get_storage_dyn();
        let (value, revision) = storage
            .get_derived_node_value_and_revision(self.derived_node_id)
            .expect("Derived node not found. This is indicative of a bug in Pico.");
        storage.register_dependency_in_parent_memoized_fn(
            NodeKind::Derived(self.derived_node_id),
            revision.time_updated,
        );
        match self.kind {
            MemoRefKind::Value => value
                .downcast_ref::<T>()
                .expect("Unexpected memoized value type. This is indicative of a bug in Pico."),
            // SAFETY: Caller guarantees that the provided database owns this derived node.
            MemoRefKind::RawPtr => unsafe {
                value
                    .downcast_ref::<RawPtr<T>>()
                    .expect("Unexpected memoized value type. This is indicative of a bug in Pico.")
                    .as_ref()
            },
        }
    }

    pub fn lookup<'db>(&self) -> &'db T {
        // SAFETY: Database outlives this MemoRef
        let db = unsafe { &*self.db };
        let storage = db.get_storage_dyn();
        let (value, _) = storage
            .get_derived_node_value_and_revision(self.derived_node_id)
            .expect("Derived node not found. This is indicative of a bug in Pico.");
        match self.kind {
            MemoRefKind::Value => value
                .downcast_ref::<T>()
                .expect("Unexpected memoized value type. This is indicative of a bug in Pico."),
            // SAFETY: Caller guarantees that the provided database owns this derived node.
            MemoRefKind::RawPtr => unsafe {
                value
                    .downcast_ref::<RawPtr<T>>()
                    .expect("Unexpected memoized value type. This is indicative of a bug in Pico.")
                    .as_ref()
            },
        }
    }
}

impl<T: 'static + Clone> MemoRef<T> {
    pub fn to_owned(&self) -> T {
        self.lookup().clone()
    }
}

impl<T: 'static, E: Clone + 'static> MemoRef<Result<T, E>> {
    pub fn try_lookup<'db>(&self) -> Result<&'db T, E> {
        self.lookup().as_ref().map_err(|e| e.clone())
    }

    pub fn try_lookup_tracked<'db>(&self) -> Result<&'db T, E> {
        self.lookup_tracked().as_ref().map_err(|e| e.clone())
    }
}

impl<T> From<MemoRef<T>> for ParamId {
    fn from(val: MemoRef<T>) -> Self {
        let idx: u64 = val.derived_node_id.index().into();
        ParamId::from(idx)
    }
}
