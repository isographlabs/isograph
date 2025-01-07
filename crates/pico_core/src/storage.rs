use std::any::Any;

use crate::{
    container::Container,
    database::Database,
    dyn_eq::DynEq,
    epoch::Epoch,
    node::{Dependency, DerivedNode, NodeId, SourceNode},
    params::ParamId,
    source::SourceKey,
};

pub trait Storage<Db: Database> {
    fn nodes(&self) -> &impl Container<NodeId, DerivedNode<Db>>;
    fn values(&self) -> &impl Container<NodeId, Box<dyn DynEq>>;
    fn sources(&self) -> &impl Container<SourceKey, SourceNode>;
    fn source_values(&self) -> &impl Container<SourceKey, Box<dyn DynEq>>;
    fn params(&self) -> &impl Container<ParamId, Box<dyn Any>>;
    fn current_epoch(&self) -> Epoch;
}

pub trait StorageMut<Db: Database> {
    fn nodes(&mut self) -> &mut impl Container<NodeId, DerivedNode<Db>>;
    fn values(&mut self) -> &mut impl Container<NodeId, Box<dyn DynEq>>;
    fn sources(&mut self) -> &mut impl Container<SourceKey, SourceNode>;
    fn source_values(&mut self) -> &mut impl Container<SourceKey, Box<dyn DynEq>>;
    fn params(&mut self) -> &mut impl Container<ParamId, Box<dyn Any>>;
    fn increment_epoch(&mut self) -> Epoch;
    fn dependency_stack(&mut self) -> &mut Vec<Vec<(Epoch, Dependency)>>;
}
