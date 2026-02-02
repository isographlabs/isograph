use common_lang_types::{ConstExportName, RelativePathToSourceFile, WithEmbeddedLocation};
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use std::fmt::Debug;

use crate::{
    ClientObjectSelectableNameWrapper, ClientScalarSelectableNameWrapper, EntityNameWrapper,
    IsographFieldDirective, IsographSemanticToken, ObjectSelectionPath, Selection,
    TypeAnnotationDeclaration, VariableDeclaration, isograph_resolved_node::IsographResolvedNode,
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
    pub description: Option<WithEmbeddedLocation<Description>>,
    #[resolve_field]
    pub selection_set: WithEmbeddedLocation<SelectionSet>,
    pub directive_set: WithEmbeddedLocation<Vec<WithEmbeddedLocation<IsographFieldDirective>>>,
    #[resolve_field]
    pub variable_definitions: Vec<WithEmbeddedLocation<VariableDeclaration>>,
    pub definition_path: RelativePathToSourceFile,

    pub semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>,
}

pub type ClientFieldDeclarationPath<'a> = PositionResolutionPath<&'a ClientFieldDeclaration, ()>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub struct ClientPointerDeclaration {
    pub const_export_name: ConstExportName,
    #[resolve_field]
    pub parent_type: WithEmbeddedLocation<EntityNameWrapper>,
    #[resolve_field]
    pub client_pointer_name: WithEmbeddedLocation<ClientObjectSelectableNameWrapper>,
    #[resolve_field]
    pub target_type: WithEmbeddedLocation<TypeAnnotationDeclaration>,
    pub directives: WithEmbeddedLocation<Vec<WithEmbeddedLocation<IsographFieldDirective>>>,
    #[resolve_field]
    pub description: Option<WithEmbeddedLocation<Description>>,
    #[resolve_field]
    pub selection_set: WithEmbeddedLocation<SelectionSet>,
    #[resolve_field]
    pub variable_definitions: Vec<WithEmbeddedLocation<VariableDeclaration>>,
    pub definition_path: RelativePathToSourceFile,

    pub semantic_tokens: Vec<WithEmbeddedLocation<IsographSemanticToken>>,
}

pub type ClientPointerDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientPointerDeclaration, ()>;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash, ResolvePosition)]
#[resolve_position(parent_type=SelectionSetParentType<'a>, resolved_node=IsographResolvedNode<'a>)]
pub struct SelectionSet {
    #[resolve_field]
    pub selections: Vec<WithEmbeddedLocation<Selection>>,
}

#[derive(Debug)]
pub enum SelectionSetParentType<'a> {
    ObjectSelection(Box<ObjectSelectionPath<'a>>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

pub type SelectionSetPath<'a> =
    PositionResolutionPath<&'a SelectionSet, SelectionSetParentType<'a>>;
