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

    pub fn tracked(&'a self) -> &'a T {
        let _ = self.db.get_singleton::<C>();
        (self.projector)(self.db)
    }

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

    pub fn tracked(&'a mut self) -> &'a mut T {
        let next = self
            .db
            .get_singleton::<C>()
            .map_or(C::default(), |c| c.increment());
        self.db.set(next);
        (self.projector)(self.db)
    }

    pub fn untracked(&'a mut self) -> &'a mut T {
        (self.projector)(self.db)
    }
}
