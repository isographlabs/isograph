use common_lang_types::{
    ArgumentName, DescriptionValue, DirectiveName, EntityName, InputValueName, SelectableName,
    ValueKeyName,
};
use nonempty::NonEmpty;
use span::WithSpan;

use crate::{ConstantValue, EnumLiteralValueWrapper, TypeAnnotation};

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

/// Spec 3.8 `UnionTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub member_types: Vec<WithSpan<EntityNameWrapper>>,
}

/// Spec 3.9 `EnumTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub values: Vec<WithSpan<EnumValueDefinition>>,
}

/// Spec 3.9 `EnumValueDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumValueDefinition {
    pub description: Option<WithSpan<Description>>,
    pub value: WithSpan<EnumLiteralValueWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
}

/// Spec 3.10 `InputObjectTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputObjectTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<InputValueDefinition>>,
}

/// Spec 3.3 `SchemaDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDefinition {
    pub description: Option<WithSpan<Description>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub root_operation_types: Vec<WithSpan<RootOperationTypeDefinition>>,
}

/// Spec 3.3 `RootOperationTypeDefinition : OperationType : NamedType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RootOperationTypeDefinition {
    pub operation_type: WithSpan<OperationTypeWrapper>,
    pub type_name: WithSpan<EntityNameWrapper>,
}

/// Spec 2.4 `OperationType`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
#[strum(serialize_all = "lowercase")]
pub enum OperationType {
    Query,
    Mutation,
    Subscription,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct OperationTypeWrapper(pub OperationType);

/// Spec 3.13 `DirectiveDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DirectiveDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<DirectiveNameWrapper>,
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
    pub repeatable: DirectiveRepeatability,
    pub locations: Vec<WithSpan<DirectiveLocationWrapper>>,
}

/// Whether a directive definition has the `repeatable` keyword (spec 3.13).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DirectiveRepeatability {
    Repeatable,
    NotRepeatable,
}

/// Spec 3.13 `DirectiveLocation`: `ExecutableDirectiveLocation` then
/// `TypeSystemDirectiveLocation`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, strum::EnumString)]
#[strum(serialize_all = "SCREAMING_SNAKE_CASE")]
pub enum DirectiveLocation {
    Query,
    Mutation,
    Subscription,
    Field,
    FragmentDefinition,
    FragmentSpread,
    InlineFragment,
    VariableDefinition,
    Schema,
    Scalar,
    Object,
    FieldDefinition,
    ArgumentDefinition,
    Interface,
    Union,
    Enum,
    EnumValue,
    InputObject,
    InputFieldDefinition,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub struct DirectiveLocationWrapper(pub DirectiveLocation);

/// Spec 3.1 `TypeSystemDefinitionOrExtension`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeSystemDefinitionOrExtension {
    Definition(TypeSystemDefinition),
    Extension(TypeSystemExtension),
}

/// Spec 3 `TypeSystemDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeSystemDefinition {
    Schema(SchemaDefinition),
    Type(TypeDefinition),
    Directive(DirectiveDefinition),
}

/// Spec 3.4 `TypeDefinition`: the definition of a named entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeDefinition {
    Scalar(ScalarTypeDefinition),
    Object(ObjectTypeDefinition),
    Interface(InterfaceTypeDefinition),
    Union(UnionTypeDefinition),
    Enum(EnumTypeDefinition),
    InputObject(InputObjectTypeDefinition),
}

/// Spec 3.1 `TypeSystemExtension`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeSystemExtension {
    Schema(SchemaExtension),
    Type(TypeExtension),
}

/// Spec 3.4.3 `TypeExtension`: an extension of a named entity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeExtension {
    Scalar(ScalarTypeExtension),
    Object(ObjectTypeExtension),
    Interface(InterfaceTypeExtension),
    Union(UnionTypeExtension),
    Enum(EnumTypeExtension),
    InputObject(InputObjectTypeExtension),
}

/// Spec 3.6.3 `ObjectTypeExtension`. At least one of `interfaces`, `directives`, `fields`
/// is non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

/// Spec 3.7.1 `InterfaceTypeExtension`. At least one of `interfaces`, `directives`,
/// `fields` is non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

/// Spec 3.5.6 `ScalarTypeExtension : extend scalar Name Directives[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: NonEmpty<WithSpan<Directive>>,
}

/// Spec 3.8.1 `UnionTypeExtension`. At least one of `directives`, `member_types` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub member_types: Vec<WithSpan<EntityNameWrapper>>,
}

/// Spec 3.9.1 `EnumTypeExtension`. At least one of `directives`, `values` is non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub values: Vec<WithSpan<EnumValueDefinition>>,
}

/// Spec 3.10.2 `InputObjectTypeExtension`. At least one of `directives`, `fields` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputObjectTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<InputValueDefinition>>,
}

/// Spec 3.3.2 `SchemaExtension`. At least one of `directives`, `root_operation_types` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaExtension {
    pub directives: Vec<WithSpan<Directive>>,
    pub root_operation_types: Vec<WithSpan<RootOperationTypeDefinition>>,
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
