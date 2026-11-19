use common_lang_types::{ArgumentName, DescriptionValue, DirectiveName, EntityName, ValueKeyName};
use span::WithSpan;

use crate::ConstantValue;

/// Spec 3.5 `ScalarTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
}

/// Spec 2.13 `Directive[Const] : @ Name Arguments[Const]?`. The span runs from `@` through
/// the arguments.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Directive {
    pub name: WithSpan<DirectiveNameWrapper>,
    pub arguments: Vec<WithSpan<Argument>>,
}

/// Spec 2.7 `Argument[Const] : Name : Value[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Argument {
    pub name: WithSpan<ArgumentNameWrapper>,
    pub value: WithSpan<ConstantValue>,
}

/// The name of an entity: a type's own name, an implemented interface, a union member, a
/// root operation type, or a named type in a type annotation.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EntityNameWrapper(pub EntityName);

/// A directive argument's name.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ArgumentNameWrapper(pub ArgumentName);

/// An object value field's name.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ValueKeyNameWrapper(pub ValueKeyName);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DirectiveNameWrapper(pub DirectiveName);

/// The decoded value of a description, quotes excluded.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Description(pub DescriptionValue);
