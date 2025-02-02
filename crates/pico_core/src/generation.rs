use std::any::Any;

use crate::{arena::Arena, database::Database, node::DerivedNode};

#[derive(Debug)]
pub struct Generation<Db: Database + ?Sized> {
    pub params: Arena<Box<dyn Any>>,
    pub derived_nodes: Arena<DerivedNode<Db>>,
}

impl<Db: Database + ?Sized> Generation<Db> {
    pub fn new() -> Self {
        Self {
            params: Arena::new(),
            derived_nodes: Arena::new(),
        }
    }
}

impl<Db: Database + ?Sized> Default for Generation<Db> {
    fn default() -> Self {
        Self::new()
    }
}
