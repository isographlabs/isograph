use std::marker::PhantomData;

use crate::{Database, Singleton};

pub trait Counter: Singleton + Default + Copy + Eq + 'static {
    fn increment(self) -> Self;
}

pub type Projector<Db, T> = for<'a> fn(&'a Db) -> &'a T;

pub struct View<'a, Db: Database, T, C: Counter> {
    db: &'a Db,
    projector: Projector<Db, T>,
    phantom: PhantomData<C>,
}

impl<'a, Db: Database, T, C: Counter> View<'a, Db, T, C> {
    pub fn new(db: &'a Db, projector: Projector<Db, T>) -> Self {
        Self {
            db,
            projector,
            phantom: PhantomData,
        }
    }

    /// Provides tracked access to the underlying field. On a tracked map, tracked access is
    /// required for correctness if adding or removing an item from the underlying map
    /// affects the result. For example, when iterating, use tracked access.
    pub fn tracked(&'a self) -> &'a T {
        let _ = self.db.get_singleton::<C>();
        (self.projector)(self.db)
    }

    /// Provides untracked access to the underlying field. On a tracked map, untracked access is
    /// only correct if every accessed value is later tracked, and unrelated additions to or
    /// removals from the underlying map do not affect the result. For example, if the map's values
    /// are themselves [`SourceId`](SourceId) and you are accessing a single, specific value by
    /// key, it is safe to use an untracked access.
    ///
    /// This may improve performance, as it means that unrelated additions and removals to the map
    /// will not invalidate this function call in pico.
    pub fn untracked(&'a self) -> &'a T {
        (self.projector)(self.db)
    }
}

pub type ProjectorMut<Db, T> = for<'a> fn(&'a mut Db) -> &'a mut T;

pub struct MutView<'a, Db: Database, T, C: Counter> {
    db: &'a mut Db,
    projector: ProjectorMut<Db, T>,
    phantom: PhantomData<C>,
}

impl<'a, Db: Database, T, C: Counter> MutView<'a, Db, T, C> {
    pub fn new(db: &'a mut Db, projector: ProjectorMut<Db, T>) -> Self {
        Self {
            db,
            projector,
            phantom: PhantomData,
        }
    }

    /// Provides tracked access to the underlying field. On a tracked map, tracked access is
    /// required for correctness if adding or removing an item from the underlying map
    /// affects the result. For example, when iterating, use tracked access.
    pub fn tracked(&'a mut self) -> &'a mut T {
        let next = self
            .db
            .get_singleton::<C>()
            .map_or(C::default(), |c| c.increment());
        self.db.set(next);
        (self.projector)(self.db)
    }
}
