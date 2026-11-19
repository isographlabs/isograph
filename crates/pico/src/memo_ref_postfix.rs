use std::hash::Hash;

use crate::{Database, DynEq, MemoRef};

pub trait MemoRefPostfix
where
    Self: Sized,
{
    #[inline(always)]
    fn interned_value(self, db: &impl Database) -> MemoRef<Self>
    where
        Self: Clone + Hash + DynEq,
    {
        db.intern_value(self)
    }

    #[inline(always)]
    fn interned_ref(&self, db: &impl Database) -> MemoRef<Self>
    where
        Self: Clone + Hash + DynEq,
    {
        db.intern_ref(self)
    }
}

impl<T> MemoRefPostfix for T {}
