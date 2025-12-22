use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, EntrypointDeclarationPath,
    IsographResolvedNode, VariableDeclarationPath,
};

macro_rules! define_wrapper {
    ($struct_name:ident, $inner:ty, $parent_type:ty, $path_type:ident) => {
        #[derive(
            Debug,
            Copy,
            Clone,
            Eq,
            PartialEq,
            Hash,
            PartialOrd,
            Ord,
            ::resolve_position_macros::ResolvePosition,
        )]
        #[resolve_position(parent_type=$parent_type, resolved_node=IsographResolvedNode<'a>)]
        pub struct $struct_name(pub $inner);

        impl From<$inner> for $struct_name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = $inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }

        pub type $path_type<'a> =
            ::resolve_position::PositionResolutionPath<&'a $struct_name, $parent_type>;
    };
}

define_wrapper!(
    Description,
    common_lang_types::DescriptionValue,
    DescriptionParent<'a>,
    DescriptionPath
);

#[derive(Debug)]
pub enum DescriptionParent<'a> {
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

define_wrapper!(
    EntityNameWrapper,
    common_lang_types::EntityName,
    EntityNameWrapperParent<'a>,
    EntityNameWrapperPath
);

#[derive(Debug)]
pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

define_wrapper!(
    ClientScalarSelectableNameWrapper,
    common_lang_types::SelectableName,
    ClientScalarSelectableNameWrapperParent<'a>,
    ClientScalarSelectableNameWrapperPath
);

#[derive(Debug)]
pub enum ClientScalarSelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}

define_wrapper!(
    ClientObjectSelectableNameWrapper,
    common_lang_types::SelectableName,
    ClientObjectSelectableNameWrapperParent<'a>,
    ClientObjectSelectableNameWrapperPath
);

#[derive(Debug)]
pub enum ClientObjectSelectableNameWrapperParent<'a> {
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}

define_wrapper!(
    VariableNameWrapper,
    common_lang_types::VariableName,
    VariableNameWrapperParentType<'a>,
    VariableNameWrapperPath
);

#[derive(Debug)]
pub enum VariableNameWrapperParentType<'a> {
    VariableDeclarationInner(VariableDeclarationPath<'a>),
}
