use std::any::Any;

use crate::{arena::Arena, derived_node::DerivedNode, index::Index, DerivedNodeId, ParamId};

#[derive(Debug)]
pub struct Generation {
    params: Arena<Box<dyn Any>>,
    derived_nodes: Arena<DerivedNode>,
}

impl Generation {
    pub fn new() -> Self {
        Self {
            params: Arena::new(),
            derived_nodes: Arena::new(),
        }
    }

    pub fn get_param(&self, param: Index<ParamId>) -> &Box<dyn Any> {
        self.params.get(param.idx)
    }

    pub fn insert_param(&self, param: Box<dyn Any>) -> usize {
        self.params.push(param)
    }

    pub fn get_derived_node(&self, derived_node: Index<DerivedNodeId>) -> &DerivedNode {
        self.derived_nodes.get(derived_node.idx)
    }

    pub fn insert_derived_node(&self, derived_node: DerivedNode) -> usize {
        self.derived_nodes.push(derived_node)
    }
}

impl Default for Generation {
    fn default() -> Self {
        Self::new()
    }
}
