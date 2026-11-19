# GraphQL schema parser: AST

Requires parsing-standards.md.

What `parse_schema` returns. Every type follows the spec production it names (GraphQL September 2025; section numbers are that edition's). Origin: upstream isograph `crates/graphql_lang_types` (`graphql_sdl.rs`, `graphql_type_annotation.rs`, `graphql_directives.rs`, `value.rs`) in `../isograph`, and iso `crates/isograph_parser` for wrappers, values, and type annotations. Delta from upstream isograph:

- `WithSpan` on every node that is in the source; no `WithEmbeddedLocation`.
- Interned-key wrappers per role (Wrappers below), the i2 pattern.
- The `GraphQL` prefix is dropped. Enum variants do not repeat the payload's suffix (`Object(ObjectTypeDefinition)`).
- All seven extensions exist (upstream has only `ObjectTypeExtension`), and the schema definition stores its root operation types as the spec's list.
- `DirectiveDefinition.repeatable` is `DirectiveRepeatability`, not `Option<WithEmbeddedLocation<()>>`.
- `NonNull` follows spec 2.12 `NonNullType : NamedType ! | ListType !`, so `[String!]!!` is not representable.
- `FloatValue` has no `From<f64>`, `From<i64>`, or `Display`; `DirectiveLocation` has no `Display`. Nothing reads them.
- A description is any spec `StringValue` (block or not) and its value is decoded (parsing-standards.md, String values).

String keys from `common_lang_types`: `EntityName`, `SelectableName`, `InputValueName`, `ArgumentName`, `ValueKeyName`, `DirectiveName`, `EnumLiteralValue`, `StringLiteralValue`, `DescriptionValue`.

No type derives `ResolvePosition` (parsing-standards.md, Later). Every type is `pub` because `graphql_network_protocol` reads it.

## Parsed schema

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
/// One schema file: every top-level definition and extension, in source order, each parsed
/// or failed on its own.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParsedSchema {
    pub definitions: Vec<Result<WithSpan<TypeSystemDefinitionOrExtension>, WithSpan<DefinitionError>>>,
}

/// A top-level definition or extension whose parse failed. The wrapping span is the text the
/// definition occupied, from its first token to the last token recovery skipped.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DefinitionError {
    /// The name the failed definition declared; `None` when the parse failed before the
    /// name, and for a schema definition or extension, which has no name.
    pub name: Option<DefinitionName>,
    pub error: WithSpan<ParseError>,
}

/// The name a top-level definition or extension declares.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DefinitionName {
    Entity(EntityName),
    Directive(DirectiveName),
}
```

`ParsedSchema` is a named-field struct with one field because semantic tokens become its second field with the first LSP feature on schema files. `DefinitionError.name` is what lets a lookup of `User` see that `type User { ... }` failed (integration.md); without it, a failed `User` would look like a missing `User`. `name` holds the interned key without a span; nothing reads the name's span on a failed definition.

## Document

```rust
// from crates/graphql_schema_parser/src/ast.rs
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
```

The nesting is the spec's. `entity_named` (integration.md) collects `TypeDefinition`s and `TypeExtension`s by name; a schema or directive definition cannot reach it. `TypeDefinition`, `TypeExtension`, `TypeSystemDefinition`, and `TypeSystemExtension` are not wrapped in `WithSpan` inside their parent enum: the span is on the entry's `WithSpan<TypeSystemDefinitionOrExtension>`, and `entity_named` reuses it.

A definition's or extension's span (on the `Ok` entry's `WithSpan`) runs from its first token (its description when it has one) to its last. Spans are byte offsets in the file.

## Object and interface

```rust
// from crates/graphql_schema_parser/src/ast.rs
/// Spec 3.6 `ObjectTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<FieldDefinition>>,
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

/// Spec 3.7 `InterfaceTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InterfaceTypeDefinition {
    pub description: Option<WithSpan<Description>>,
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
```

`fields` and `arguments` are empty when the source has no `{ ... }` or `( ... )`. The spec requires at least one item inside a present group (`FieldsDefinition : { FieldDefinition+ }`, `ArgumentsDefinition : ( InputValueDefinition+ )`), so an empty vec means absent and nothing else.

The spec's object extension alternatives (3.6.3) all require at least one of implements, directives, or fields. The parser enforces that with `Expectation::ImplementsDirectiveOrOpenBrace` at the token after the name; the struct admits all three empty and the parser never builds it. Same for the interface, union, enum, input object, and schema extensions below.

## Scalar, union, enum, input object

```rust
// from crates/graphql_schema_parser/src/ast.rs
/// Spec 3.5 `ScalarTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
}

/// Spec 3.5.6 `ScalarTypeExtension : extend scalar Name Directives[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScalarTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: NonEmpty<WithSpan<Directive>>,
}

/// Spec 3.8 `UnionTypeDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub member_types: Vec<WithSpan<EntityNameWrapper>>,
}

/// Spec 3.8.1 `UnionTypeExtension`. At least one of `directives`, `member_types` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnionTypeExtension {
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

/// Spec 3.9.1 `EnumTypeExtension`. At least one of `directives`, `values` is non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EnumTypeExtension {
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

/// Spec 3.10.2 `InputObjectTypeExtension`. At least one of `directives`, `fields` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InputObjectTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Vec<WithSpan<Directive>>,
    pub fields: Vec<WithSpan<InputValueDefinition>>,
}
```

`member_types` is empty when there is no `=` (spec 3.8 `UnionMemberTypes?`). `ScalarTypeExtension.directives` is `NonEmpty` because spec 3.5.6 has one alternative and it requires directives; `nonempty::NonEmpty::from_vec` turns the parsed vec into it, and an empty vec is `Expectation::Directive`.

## Schema and directive definitions

```rust
// from crates/graphql_schema_parser/src/ast.rs
/// Spec 3.3 `SchemaDefinition`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaDefinition {
    pub description: Option<WithSpan<Description>>,
    pub directives: Vec<WithSpan<Directive>>,
    pub root_operation_types: Vec<WithSpan<RootOperationTypeDefinition>>,
}

/// Spec 3.3.2 `SchemaExtension`. At least one of `directives`, `root_operation_types` is
/// non-empty.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SchemaExtension {
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
```

`root_operation_types` is the spec's `RootOperationTypeDefinition+` in source order. Two entries with the same operation type parse; spec 3.3 Schema Validation rejects them, and that check belongs to the pass that builds the schema, not the parser. Upstream isograph stores three options and reports `DuplicateRootOperation` while parsing.

`repeatable` has no span; nothing reads the keyword's position. `DirectiveLocationWrapper` and `OperationTypeWrapper` wrap the unit enums so each location and operation type is a `WithSpan` node, the shape of iso `BooleanValue(pub Boolean)`.

## Directives and arguments

```rust
// from crates/graphql_schema_parser/src/ast.rs
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
```

`arguments` is empty when there is no `( ... )`; a present `()` is an error (spec 2.7 `Arguments : ( Argument+ )`).

## Type annotation

```rust
// from crates/graphql_schema_parser/src/type_annotation.rs
/// Spec 2.12 `Type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    NonNull(NonNullTypeAnnotation),
}

/// Spec 2.12 `NonNullType`: a named or list type followed by `!`. The span of the inner
/// type excludes the `!`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonNullTypeAnnotation(pub WithSpan<NullableTypeAnnotation>);

/// The type a `NonNullType` wraps: spec 2.12 `NamedType` or `ListType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NullableTypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

/// Spec 2.12 `NamedType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedTypeAnnotation(pub WithSpan<EntityNameWrapper>);

/// Spec 2.12 `ListType : [ Type ]`. The span includes the brackets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListTypeAnnotation(pub WithSpan<TypeAnnotation>);

impl NullableTypeAnnotation {
    pub(crate) fn into_type_annotation(self) -> TypeAnnotation {
        match self {
            NullableTypeAnnotation::Named(named) => TypeAnnotation::Named(named),
            NullableTypeAnnotation::List(list) => TypeAnnotation::List(list),
        }
    }
}
```

Origin: iso `crates/isograph_parser/src/variables.rs` (`TypeAnnotation`, `NamedTypeAnnotation`, `ListTypeAnnotation`, and `NamedOrList::into_type_annotation`). Delta: iso has no `!` node because iso models nullability as a union with null; SDL models `NonNullType`, so `NonNullTypeAnnotation` and `NullableTypeAnnotation` replace iso's `UnionTypeAnnotation`. `NamedTypeAnnotation` and `ListTypeAnnotation` are newtypes (one field each; iso's `NamedTypeAnnotation` has a named field and iso's `ListTypeAnnotation` has an `extra` field for chunk leftovers that does not exist here). No `ResolvePosition` derives.

`String` is `Named`. `String!` is `NonNull` whose inner `Named` spans `String`. `[String]` is `List` spanning the brackets. `[String]!` is `NonNull` whose inner `List` spans `[String]`. `[String!]` is `List` of `NonNull(Named)`. The outer `WithSpan<TypeAnnotation>` of a non-null type includes the `!`; the inner `WithSpan<NullableTypeAnnotation>` does not.

## Constant values

```rust
// from crates/graphql_schema_parser/src/value.rs
/// Spec 2.10 `Value[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantValue {
    Int(IntegerValue),
    Float(FloatValue),
    String(StringLiteralValueWrapper),
    Boolean(BooleanValue),
    Null(NullValue),
    Enum(EnumLiteralValueWrapper),
    List(ListValue),
    Object(ObjectValue),
}

/// Spec 2.10.7 `ListValue[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListValue(pub Vec<WithSpan<ConstantValue>>);

/// Spec 2.10.8 `ObjectValue[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectValue(pub Vec<WithSpan<ObjectField>>);

/// Spec 2.10.8 `ObjectField[Const] : Name : Value[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectField {
    pub name: WithSpan<ValueKeyNameWrapper>,
    pub value: WithSpan<ConstantValue>,
}

/// Spec 2.10.1 `IntValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntegerValue(pub i64);

/// Spec 2.10.2 `FloatValue`, stored as its bits so the tree is `Eq`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FloatValue(u64);

impl FloatValue {
    pub fn new(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub fn as_float(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl std::fmt::Debug for FloatValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.as_float()))
    }
}

/// Spec 2.10.3 `BooleanValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

/// Spec 2.10.5 `NullValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NullValue;

/// Spec 2.10.4 `StringValue`, decoded.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StringLiteralValueWrapper(pub StringLiteralValue);

/// Spec 2.10.6 `EnumValue`: a constant enum value, or the value an enum value definition
/// defines.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EnumLiteralValueWrapper(pub EnumLiteralValue);
```

Origin: iso `crates/isograph_parser/src/arguments.rs` (`IntegerValue`, `BooleanValue`, `Boolean`, `NullValue`, `StringLiteralValueWrapper`) and upstream isograph `value.rs` (`FloatValue` with `new`, `as_float`, and its `Debug`). Delta: the variable arm of iso `NonConstantValue` is gone (spec 2.10 `[~Const] Variable`); `Float` and `Enum` are added; list and object items are plain `WithSpan` (iso wraps them in `Slot` for chunk recovery). `ListValue` and `ObjectValue` use the spec's names; iso's `ListLiteral` and `ObjectLiteral` are the non-constant iso forms. No `ResolvePosition` derives.

## Wrappers

```rust
// from crates/graphql_schema_parser/src/ast.rs
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
```

The i2 wrapper pattern (AGENTS.md, nits): inner field `pub`; construction is `token.interned().map(EntityNameWrapper)`; callers read `.0`; no `From<StringKey>` on a wrapper. These live in `graphql_schema_parser`. `EntityNameWrapper`, `SelectableNameWrapper`, `ArgumentNameWrapper`, `ValueKeyNameWrapper`, and `Description` share names with iso types of the same role; they are distinct types, and a crate that uses both path-qualifies.

An input value definition's name is `InputValueNameWrapper` because spec 3.6.1 uses one production, `InputValueDefinition`, for field arguments, directive arguments, and input object fields; upstream isograph names it `InputValueName`. A directive argument's name (spec 2.7 `Argument`) is `ArgumentNameWrapper`, and an object value field's name (spec 2.10.8 `ObjectField`) is `ValueKeyNameWrapper`, the two names iso uses for the same roles.
