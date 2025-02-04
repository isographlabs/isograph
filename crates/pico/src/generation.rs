use std::any::Any;

use crate::arena::Arena;

use crate::derived_node::DerivedNode;

#[derive(Debug)]
pub struct Generation {
    pub params: Arena<Box<dyn Any>>,
    pub derived_nodes: Arena<DerivedNode>,
}

impl Generation {
    pub fn new() -> Self {
        Self {
            params: Arena::new(),
            derived_nodes: Arena::new(),
        }
    }
}

impl Default for Generation {
    fn default() -> Self {
        Self::new()
    }
}
