use std::fmt::Debug;

use common_lang_types::WithEmbeddedLocation;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;

use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, ConstantValue, IsographResolvedNode,
    TypeAnnotationDeclaration, VariableNameWrapper,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=VariableDeclarationParentType<'a>, resolved_node=IsographResolvedNode<'a>)]
pub struct VariableDeclarationInner {
    #[resolve_field]
    pub name: WithEmbeddedLocation<VariableNameWrapper>,
    #[resolve_field]
    pub type_: WithEmbeddedLocation<TypeAnnotationDeclaration>,
    pub default_value: Option<WithEmbeddedLocation<ConstantValue>>,
}

pub type VariableDeclaration = VariableDeclarationInner;

pub type VariableDeclarationPath<'a> =
    PositionResolutionPath<&'a VariableDeclaration, VariableDeclarationParentType<'a>>;

#[derive(Debug)]
pub enum VariableDeclarationParentType<'a> {
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}
