use std::any::Any;

use crate::{
    container::Container,
    database::Database,
    epoch::Epoch,
    key::Key,
    node::{Dependency, DerivedNode, DerivedNodeId, SourceNode},
    params::ParamId,
};

pub trait Storage<Db: Database + ?Sized> {
    fn derived_nodes(&self) -> &impl Container<DerivedNodeId, DerivedNode<Db>>;
    fn source_nodes(&self) -> &impl Container<Key, SourceNode>;
    fn params(&self) -> &impl Container<ParamId, Box<dyn Any>>;
    fn current_epoch(&self) -> Epoch;
}

pub trait StorageMut<Db: Database + ?Sized> {
    fn derived_nodes(&mut self) -> &mut impl Container<DerivedNodeId, DerivedNode<Db>>;
    fn source_nodes(&mut self) -> &mut impl Container<Key, SourceNode>;
    fn params(&mut self) -> &mut impl Container<ParamId, Box<dyn Any>>;
    fn increment_epoch(&mut self) -> Epoch;
    fn dependency_stack(&mut self) -> &mut Vec<Vec<(Epoch, Dependency)>>;
}
