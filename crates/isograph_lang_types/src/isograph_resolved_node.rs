use crate::{
    ClientFieldDeclarationPath, ClientObjectSelectableNameWrapperPath,
    ClientPointerDeclarationPath, ClientScalarSelectableNameWrapperPath, DescriptionPath,
    EntrypointDeclarationPath, ObjectSelectionPath, ScalarSelectionPath,
    ServerObjectEntityNameWrapperPath,
};

#[derive(Debug)]
pub enum IsographResolvedNode<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ServerObjectEntityNameWrapper(ServerObjectEntityNameWrapperPath<'a>),
    Description(DescriptionPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a, ()>),
    ObjectSelection(ObjectSelectionPath<'a, (), ()>),
    ClientScalarSelectableNameWrapper(ClientScalarSelectableNameWrapperPath<'a>),
    ClientObjectSelectableNameWrapper(ClientObjectSelectableNameWrapperPath<'a>),
}
