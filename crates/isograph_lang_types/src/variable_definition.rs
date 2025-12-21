use std::fmt::Debug;

use common_lang_types::WithEmbeddedLocation;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;

use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, ConstantValue, IsographResolvedNode,
    TypeAnnotation, VariableNameWrapper,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=VariableDefinitionParentType<'a>, resolved_node=IsographResolvedNode<'a>)]
pub struct VariableDefinition {
    #[resolve_field]
    pub name: WithEmbeddedLocation<VariableNameWrapper>,
    #[resolve_field]
    pub type_: WithEmbeddedLocation<TypeAnnotation>,
    pub default_value: Option<WithEmbeddedLocation<ConstantValue>>,
}

pub type VariableDefinitionPath<'a> =
    PositionResolutionPath<&'a VariableDefinition, VariableDefinitionParentType<'a>>;

#[derive(Debug)]
pub enum VariableDefinitionParentType<'a> {
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}
