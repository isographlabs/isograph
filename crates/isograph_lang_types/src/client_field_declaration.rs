use common_lang_types::{
    ConstExportName, RelativePathToSourceFile, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use std::fmt::Debug;

use crate::{
    isograph_resolved_node::IsographResolvedNode, string_key_wrappers::Description,
    ClientFieldDirectiveSet, ClientObjectSelectableNameWrapper, ClientScalarSelectableNameWrapper,
    IsographFieldDirective, IsographSemanticToken, ServerObjectEntityNameWrapper,
    UnvalidatedSelection, VariableDefinition,
};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub struct ClientFieldDeclaration {
    pub const_export_name: ConstExportName,
    #[resolve_field]
    pub parent_type: WithSpan<ServerObjectEntityNameWrapper>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    pub selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    pub client_field_directive_set: ClientFieldDirectiveSet,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
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
    pub parent_type: WithSpan<ServerObjectEntityNameWrapper>,
    // TODO this should be WithSpan<GraphQLAnnotation<ParentType>>, and we need to
    // impl<T: ResolvePosition> ResolvePosition for GraphQLTypeAnnotation<T>?
    pub target_type: GraphQLTypeAnnotation<UnvalidatedTypeName>,
    #[resolve_field]
    pub client_pointer_name: WithLocation<ClientObjectSelectableNameWrapper>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    pub selection_set: Vec<WithSpan<UnvalidatedSelection>>,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
    pub definition_path: RelativePathToSourceFile,

    pub semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, ()>;
