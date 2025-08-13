use crate::{
    ClientFieldDeclarationPath, ClientPointerDeclarationPath, DescriptionPath,
    EntrypointDeclarationPath, ParentTypePath,
};

#[derive(Debug)]
pub enum IsographResolvedNode<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ParentType(ParentTypePath<'a>),
    Description(DescriptionPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    ClientPointerDeclaration(ClientPointerDeclarationPath<'a>),
}
