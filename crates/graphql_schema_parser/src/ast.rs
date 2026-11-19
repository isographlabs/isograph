use common_lang_types::{
    ArgumentName, DescriptionValue, DirectiveName, EntityName, InputValueName, SelectableName,
    ValueKeyName,
};
use span::WithSpan;

use crate::{ConstantValue, TypeAnnotation};

/// Spec 3.6 `ObjectTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

/// Spec 3.7 `InterfaceTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

/// Spec 3.6 `FieldDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FieldDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<SelectableNameWrapper>,
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
    pub type_: WithSpan<TypeAnnotation>,
    pub directives: Vec<WithSpan<Directive>>,
}

/// Spec 3.6.1 `InputValueDefinition`: a field argument, a directive argument, or an input
/// object field (spec 3.10 `InputFieldsDefinition`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputValueDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<InputValueNameWrapper>,
    pub type_: WithSpan<TypeAnnotation>,
    pub default_value: Option<WithSpan<ConstantValue>>,
    pub directives: Vec<WithSpan<Directive>>,
}

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

/// A field definition's name.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct SelectableNameWrapper(pub SelectableName);

/// An input value definition's name.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct InputValueNameWrapper(pub InputValueName);

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
