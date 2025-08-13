use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, DescriptionPath,
    EntrypointDeclarationPath, ObjectSelectionPath, ParentTypePath, ScalarSelectionPath,
};

#[derive(Debug)]
pub enum IsographResolvedNode<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ParentType(ParentTypePath<'a>),
    Description(DescriptionPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a, ()>),
    ObjectSelection(ObjectSelectionPath<'a, (), ()>),
}
