use std::hash::Hash;

use pico::{Database, DynEq, MemoRef};

pub trait Postfix
where
    Self: Sized,
{
    #[inline(always)]
    fn wrap<T>(self, f: impl FnOnce(Self) -> T) -> T {
        f(self)
    }

    #[inline(always)]
    fn wrap_ok<E>(self) -> Result<Self, E> {
        Ok(self)
    }

    #[inline(always)]
    fn wrap_err<T>(self) -> Result<T, Self> {
        Err(self)
    }

    #[inline(always)]
    fn wrap_some(self) -> Option<Self> {
        Some(self)
    }

    #[inline(always)]
    fn boxed(self) -> Box<Self> {
        Box::new(self)
    }

    #[inline(always)]
    fn to<T>(self) -> T
    where
        Self: Into<T>,
    {
        self.into()
    }

    #[inline(always)]
    fn dbg(self) -> Self
    where
        Self: std::fmt::Debug,
    {
        dbg!(self)
    }

    #[inline(always)]
    fn note_todo(self, #[allow(unused)] message: &'static str) -> Self {
        self
    }

    #[inline(always)]
    fn note_do_not_commit(self, #[allow(unused)] message: &'static str) -> Self {
        self
    }

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

impl<T> Postfix for T {}

pub trait ErrClone {
    type Target<'a>
    where
        Self: 'a;

    fn clone_err<'a>(&'a self) -> Self::Target<'a>;
}

impl<T, E: Clone> ErrClone for Result<T, E> {
    type Target<'a>
        = Result<&'a T, E>
    where
        T: 'a,
        E: 'a;

    fn clone_err<'a>(&'a self) -> Self::Target<'a> {
        self.as_ref().map_err(Clone::clone)
    }
}
