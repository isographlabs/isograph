use common_lang_types::{
    ConstExportName, Diagnostic, EntityName, RelativePathToSourceFile, WithEmbeddedLocation,
    WithSpan,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use std::fmt::Debug;

use crate::{
    ClientObjectSelectableNameWrapper, ClientScalarSelectableDirectiveSet,
    ClientScalarSelectableNameWrapper, EntityNameWrapper, IsographFieldDirective,
    IsographSemanticToken, ObjectSelectionPath, SelectionTypeContainingSelections,
    VariableDefinition, isograph_resolved_node::IsographResolvedNode,
    string_key_wrappers::Description,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub struct ClientFieldDeclaration {
    pub const_export_name: ConstExportName,
    #[resolve_field]
    pub parent_type: WithEmbeddedLocation<EntityNameWrapper>,
    #[resolve_field]
    pub client_field_name: WithEmbeddedLocation<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    pub selection_set: WithSpan<SelectionSet<(), ()>>,
    pub client_scalar_selectable_directive_set:
        Result<ClientScalarSelectableDirectiveSet, Diagnostic>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<EntityName>>>,
    pub definition_path: RelativePathToSourceFile,

    pub semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub type ClientFieldDeclarationPath<'a> = PositionResolutionPath<&'a ClientFieldDeclaration, ()>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub struct ClientPointerDeclaration {
    pub directives: Vec<WithSpan<IsographFieldDirective>>,
    pub const_export_name: ConstExportName,
    #[resolve_field]
    pub parent_type: WithEmbeddedLocation<EntityNameWrapper>,
    #[resolve_field]
    pub target_type: GraphQLTypeAnnotation<EntityNameWrapper>,
    #[resolve_field]
    pub client_pointer_name: WithEmbeddedLocation<ClientObjectSelectableNameWrapper>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    pub selection_set: WithSpan<SelectionSet<(), ()>>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<EntityName>>>,
    pub definition_path: RelativePathToSourceFile,

    pub semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, ()>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=SelectionSetParentType<'a>, resolved_node=IsographResolvedNode<'a>, self_type_generics=<(), ()>)]
pub struct SelectionSet<TScalar, TLinked> {
    #[resolve_field]
    pub selections: Vec<WithSpan<SelectionTypeContainingSelections<TScalar, TLinked>>>,
}

#[derive(Debug)]
pub enum SelectionSetParentType<'a> {
    ObjectSelection(Box<ObjectSelectionPath<'a>>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

pub type SelectionSetPath<'a> =
    PositionResolutionPath<&'a SelectionSet<(), ()>, SelectionSetParentType<'a>>;
