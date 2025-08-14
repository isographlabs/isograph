use common_lang_types::{
    ConstExportName, RelativePathToSourceFile, UnvalidatedTypeName, VariableName, WithLocation,
    WithSpan,
};
use graphql_lang_types::GraphQLTypeAnnotation;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use serde::Deserialize;
use std::fmt::Debug;

use crate::{
    isograph_resolved_node::IsographResolvedNode, string_key_wrappers::Description,
    ClientFieldDirectiveSet, ClientObjectSelectableNameWrapper, ClientScalarSelectableNameWrapper,
    ConstantValue, IsographFieldDirective, IsographSemanticToken, ServerObjectEntityNameWrapper,
    UnvalidatedSelection,
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
    pub client_pointer_name: WithSpan<ClientObjectSelectableNameWrapper>,
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

#[derive(Deserialize, Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Copy, Default, Hash)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoadableDirectiveParameters {
    #[serde(default)]
    complete_selection_set: bool,
    #[serde(default)]
    pub lazy_load_artifact: bool,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct VariableDefinition<TValue: Ord + Debug> {
    pub name: WithLocation<VariableName>,
    pub type_: GraphQLTypeAnnotation<TValue>,
    pub default_value: Option<WithLocation<ConstantValue>>,
}

impl<TValue: Ord + Debug> VariableDefinition<TValue> {
    pub fn map<TNewValue: Ord + Debug>(
        self,
        map: &mut impl FnMut(TValue) -> TNewValue,
    ) -> VariableDefinition<TNewValue> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.map(map),
            default_value: self.default_value,
        }
    }

    pub fn and_then<TNewValue: Ord + Debug, E>(
        self,
        map: &mut impl FnMut(TValue) -> Result<TNewValue, E>,
    ) -> Result<VariableDefinition<TNewValue>, E> {
        Ok(VariableDefinition {
            name: self.name,
            type_: self.type_.and_then(map)?,
            default_value: self.default_value,
        })
    }
}
