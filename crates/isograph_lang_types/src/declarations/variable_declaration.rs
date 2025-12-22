use std::fmt::Debug;

use common_lang_types::{EmbeddedLocation, WithGenericLocation};
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;

use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, ConstantValueInner,
    IsographResolvedNode, TypeAnnotationDeclaration, VariableNameWrapper,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=VariableDeclarationParentType<'a>, resolved_node=IsographResolvedNode<'a>, self_type_generics=<EmbeddedLocation>)]
pub struct VariableDeclarationInner<TLocation> {
    #[resolve_field]
    pub name: WithGenericLocation<VariableNameWrapper, TLocation>,
    #[resolve_field]
    pub type_: WithGenericLocation<TypeAnnotationDeclaration, TLocation>,
    pub default_value: Option<WithGenericLocation<ConstantValueInner<TLocation>, TLocation>>,
}

pub type VariableDeclaration = VariableDeclarationInner<EmbeddedLocation>;

pub type VariableDeclarationPath<'a> =
    PositionResolutionPath<&'a VariableDeclaration, VariableDeclarationParentType<'a>>;

#[derive(Debug)]
pub enum VariableDeclarationParentType<'a> {
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}
