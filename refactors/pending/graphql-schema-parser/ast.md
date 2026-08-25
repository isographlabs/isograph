# GraphQL schema parser: AST

Requires parsing-standards.md.

The tree `parse_schema` / `parse_schema_extensions` return. A parse of file text. `__typename` is not a `FieldDefinition` here.

Origin: isograph `crates/graphql_lang_types` (`graphql_sdl.rs`, `graphql_type_annotation.rs`, `graphql_directives.rs`, `value.rs`) and the values `parse_schema.rs` builds. Delta: `WithSpan` on every source node; interned-key wrappers (`EntityNameWrapper`, `SelectableNameWrapper`, `ArgumentNameWrapper`, `DirectiveNameWrapper`, `EnumLiteralValueWrapper`, `StringLiteralValueWrapper`); `Description` newtype; `Boolean` / `BooleanValue` / `IntegerValue` / `NullValue` like iso `arguments.rs`; empty optional lists are `Vec::new()` (origin); `InputValueName` / `DirectiveArgumentName` / `ValueKeyName` are `ArgumentName`; `GraphQL` prefix dropped; enum variants drop the suffix that duplicated the payload type (`Object(ObjectTypeDefinition)` not `ObjectTypeDefinition(ObjectTypeDefinition)`); `NameValuePair` is `Argument` for directive arguments and constant-object fields; each `Directive` is `WithSpan<Directive>` covering `@` through its arguments (origin `GraphQLDirective` has no outer location); `repeatable` is `Option<Span>` (origin `Option<WithEmbeddedLocation<()>>`); `NamedTypeAnnotation` has `name: WithSpan<EntityNameWrapper>` so the identifier has a span; `NonNullTypeAnnotation` is a struct whose `inner` span excludes `!` (origin is an enum `Named` / `List` with no inner span; `!` is consumed, not a leaf); `ListTypeAnnotation` is a named-field struct so `#[resolve_field]` can sit on `inner` (iso `variables.rs` does the same for `NamedTypeAnnotation.name`); `ConstantList` / `ConstantObject` are tuple newtypes with `#[resolve_field]` on the vec (the derive accepts `self.0`); `DirectiveLocationWrapper` is the resolve leaf around the `DirectiveLocation` enum (the derive requires a single unnamed payload, so the unit-variant enum cannot derive `ResolvePosition`); `#[derive(ResolvePosition)]` on every node that is a resolve leaf or has spanned children.

String keys from `common_lang_types`: `EntityName` (every type name: object, scalar, interface, union, enum, input, schema operation type, implements, union member), `SelectableName` (field names), `ArgumentName` (input-value names, directive argument names, constant-object keys), `DirectiveName`, `EnumLiteralValue`, `StringLiteralValue`, `DescriptionValue`. `DirectiveLocation` has no interned-key equivalent; it is the enum below.

## Document

Most important first.

```rust
// from crates/graphql_schema_parser/src/ast.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = GraphQLResolutionNode<'a>)]
pub struct TypeSystemDocument(
    #[resolve_field]
    #[parent_variant(Document)]
    pub Vec<WithSpan<TypeSystemDefinition>>,
);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = GraphQLResolutionNode<'a>)]
pub struct TypeSystemExtensionDocument(
    #[resolve_field]
    pub Vec<WithSpan<TypeSystemExtensionOrDefinition>>,
);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemExtensionDocumentPath<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum TypeSystemExtensionOrDefinition {
    Definition(#[parent_variant(ExtensionOrDefinition)] TypeSystemDefinition),
    Extension(TypeSystemExtension),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum TypeSystemDefinition {
    Object(ObjectTypeDefinition),
    Scalar(ScalarTypeDefinition),
    Interface(InterfaceTypeDefinition),
    InputObject(InputObjectTypeDefinition),
    Enum(EnumTypeDefinition),
    Union(UnionTypeDefinition),
    Directive(DirectiveDefinition),
    Schema(SchemaDefinition),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemExtensionDocumentPath<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum TypeSystemExtension {
    Object(ObjectTypeExtension),
}

#[derive(Debug)]
pub enum TypeSystemDefinitionParent<'a> {
    Document(TypeSystemDocumentPath<'a>),
    ExtensionOrDefinition(TypeSystemExtensionDocumentPath<'a>),
}

pub type TypeSystemDocumentPath<'a> =
    PositionResolutionPath<&'a TypeSystemDocument, ()>;
pub type TypeSystemExtensionDocumentPath<'a> =
    PositionResolutionPath<&'a TypeSystemExtensionDocument, ()>;
pub type TypeSystemExtensionOrDefinitionPath<'a> = PositionResolutionPath<
    &'a TypeSystemExtensionOrDefinition,
    TypeSystemExtensionDocumentPath<'a>,
>;
pub type TypeSystemDefinitionPath<'a> =
    PositionResolutionPath<&'a TypeSystemDefinition, TypeSystemDefinitionParent<'a>>;
pub type TypeSystemExtensionPath<'a> =
    PositionResolutionPath<&'a TypeSystemExtension, TypeSystemExtensionDocumentPath<'a>>;
```

`parse_schema` returns `WithSpan<TypeSystemDocument>`. `parse_schema_extensions` returns `WithSpan<TypeSystemExtensionDocument>`.

## Definitions

```rust
// from crates/graphql_schema_parser/src/ast.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ObjectTypeDefinition {
    #[resolve_field]
    #[parent_variant(ObjectTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ObjectTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectImplements)]
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    #[resolve_field]
    #[parent_variant(ObjectTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(ObjectTypeDefinition)]
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemExtensionDocumentPath<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ObjectTypeExtension {
    #[resolve_field]
    #[parent_variant(ObjectTypeExtension)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectExtensionImplements)]
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    #[resolve_field]
    #[parent_variant(ObjectTypeExtension)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(ObjectTypeExtension)]
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ScalarTypeDefinition {
    #[resolve_field]
    #[parent_variant(ScalarTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ScalarTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ScalarTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct InterfaceTypeDefinition {
    #[resolve_field]
    #[parent_variant(InterfaceTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(InterfaceTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(InterfaceImplements)]
    pub interfaces: Vec<WithSpan<EntityNameWrapper>>,
    #[resolve_field]
    #[parent_variant(InterfaceTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(InterfaceTypeDefinition)]
    pub fields: Vec<WithSpan<FieldDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct InputObjectTypeDefinition {
    #[resolve_field]
    #[parent_variant(InputObjectTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(InputObjectTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(InputObjectTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(InputObjectTypeDefinition)]
    pub fields: Vec<WithSpan<InputValueDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct EnumTypeDefinition {
    #[resolve_field]
    #[parent_variant(EnumTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(EnumTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(EnumTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    pub enum_value_definitions: Vec<WithSpan<EnumValueDefinition>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct UnionTypeDefinition {
    #[resolve_field]
    #[parent_variant(UnionTypeDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(UnionTypeDefinition)]
    pub name: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(UnionTypeDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(UnionMember)]
    pub union_member_types: Vec<WithSpan<EntityNameWrapper>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct DirectiveDefinition {
    #[resolve_field]
    #[parent_variant(DirectiveDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(DirectiveDefinition)]
    pub name: WithSpan<DirectiveNameWrapper>,
    #[resolve_field]
    #[parent_variant(DirectiveDefinition)]
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
    pub repeatable: Option<Span>,
    #[resolve_field]
    pub locations: Vec<WithSpan<DirectiveLocationWrapper>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeSystemDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct SchemaDefinition {
    #[resolve_field]
    #[parent_variant(SchemaDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(SchemaDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
    #[resolve_field]
    #[parent_variant(SchemaQuery)]
    pub query: Option<WithSpan<EntityNameWrapper>>,
    #[resolve_field]
    #[parent_variant(SchemaMutation)]
    pub mutation: Option<WithSpan<EntityNameWrapper>>,
    #[resolve_field]
    #[parent_variant(SchemaSubscription)]
    pub subscription: Option<WithSpan<EntityNameWrapper>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = FieldDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct FieldDefinition {
    #[resolve_field]
    #[parent_variant(FieldDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(FieldDefinition)]
    pub name: WithSpan<SelectableNameWrapper>,
    #[resolve_field]
    #[parent_variant(FieldDefinition)]
    pub arguments: Vec<WithSpan<InputValueDefinition>>,
    #[resolve_field]
    #[parent_variant(Field)]
    pub type_: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(FieldDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
}

#[derive(Debug)]
pub enum FieldDefinitionParent<'a> {
    ObjectTypeDefinition(ObjectTypeDefinitionPath<'a>),
    ObjectTypeExtension(ObjectTypeExtensionPath<'a>),
    InterfaceTypeDefinition(InterfaceTypeDefinitionPath<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = InputValueDefinitionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct InputValueDefinition {
    #[resolve_field]
    #[parent_variant(InputValueDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(InputValueDefinition)]
    pub name: WithSpan<ArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(InputValue)]
    pub type_: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(InputValueDefault)]
    pub default_value: Option<WithSpan<ConstantValue>>,
    #[resolve_field]
    #[parent_variant(InputValueDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
}

#[derive(Debug)]
pub enum InputValueDefinitionParent<'a> {
    FieldDefinition(FieldDefinitionPath<'a>),
    InputObjectTypeDefinition(InputObjectTypeDefinitionPath<'a>),
    DirectiveDefinition(DirectiveDefinitionPath<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EnumTypeDefinitionPath<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct EnumValueDefinition {
    #[resolve_field]
    #[parent_variant(EnumValueDefinition)]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(EnumValueDefinition)]
    pub value: WithSpan<EnumLiteralValueWrapper>,
    #[resolve_field]
    #[parent_variant(EnumValueDefinition)]
    pub directives: Vec<WithSpan<Directive>>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = DirectiveParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct Directive {
    #[resolve_field]
    #[parent_variant(Directive)]
    pub name: WithSpan<DirectiveNameWrapper>,
    #[resolve_field]
    #[parent_variant(Directive)]
    pub arguments: Vec<WithSpan<Argument>>,
}

#[derive(Debug)]
pub enum DirectiveParent<'a> {
    ObjectTypeDefinition(ObjectTypeDefinitionPath<'a>),
    ObjectTypeExtension(ObjectTypeExtensionPath<'a>),
    ScalarTypeDefinition(ScalarTypeDefinitionPath<'a>),
    InterfaceTypeDefinition(InterfaceTypeDefinitionPath<'a>),
    InputObjectTypeDefinition(InputObjectTypeDefinitionPath<'a>),
    EnumTypeDefinition(EnumTypeDefinitionPath<'a>),
    UnionTypeDefinition(UnionTypeDefinitionPath<'a>),
    SchemaDefinition(SchemaDefinitionPath<'a>),
    FieldDefinition(FieldDefinitionPath<'a>),
    InputValueDefinition(InputValueDefinitionPath<'a>),
    EnumValueDefinition(EnumValueDefinitionPath<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct Argument {
    #[resolve_field]
    #[parent_variant(Argument)]
    pub name: WithSpan<ArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(ArgumentValue)]
    pub value: WithSpan<ConstantValue>,
}

#[derive(Debug)]
pub enum ArgumentParent<'a> {
    Directive(DirectivePath<'a>),
    ConstantObject(ConstantObjectPath<'a>),
}

pub type ObjectTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a ObjectTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type ObjectTypeExtensionPath<'a> =
    PositionResolutionPath<&'a ObjectTypeExtension, TypeSystemExtensionDocumentPath<'a>>;
pub type ScalarTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a ScalarTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type InterfaceTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a InterfaceTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type InputObjectTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a InputObjectTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type EnumTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a EnumTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type UnionTypeDefinitionPath<'a> =
    PositionResolutionPath<&'a UnionTypeDefinition, TypeSystemDefinitionParent<'a>>;
pub type DirectiveDefinitionPath<'a> =
    PositionResolutionPath<&'a DirectiveDefinition, TypeSystemDefinitionParent<'a>>;
pub type SchemaDefinitionPath<'a> =
    PositionResolutionPath<&'a SchemaDefinition, TypeSystemDefinitionParent<'a>>;
pub type FieldDefinitionPath<'a> =
    PositionResolutionPath<&'a FieldDefinition, FieldDefinitionParent<'a>>;
pub type InputValueDefinitionPath<'a> =
    PositionResolutionPath<&'a InputValueDefinition, InputValueDefinitionParent<'a>>;
pub type EnumValueDefinitionPath<'a> =
    PositionResolutionPath<&'a EnumValueDefinition, EnumTypeDefinitionPath<'a>>;
pub type DirectivePath<'a> = PositionResolutionPath<&'a Directive, DirectiveParent<'a>>;
pub type ArgumentPath<'a> = PositionResolutionPath<&'a Argument, ArgumentParent<'a>>;
```

`repeatable` is the span of the `repeatable` keyword when present. A position on that keyword resolves to `DirectiveDefinition`.

`SchemaDefinition` fields are three `Option`s, origin's `query` / `mutation` / `subscription`. A second `query:` is `ParseError::DuplicateRootOperation` at the second `query` keyword's span.

## Type annotation

`TypeAnnotation`, `NamedTypeAnnotation`, `ListTypeAnnotation`, `NonNullTypeAnnotation`, and `NonNullInner` share `parent_type = TypeAnnotationParent`, the same pattern as iso `variables.rs` (`TypeAnnotation` / `NamedTypeAnnotation` / `ListTypeAnnotation` share `TypeAnnotationParent`, and `UnionVariant` uses `#[parent_variant(Union)]`).

```rust
// from crates/graphql_schema_parser/src/type_annotation.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    NonNull(Box<NonNullTypeAnnotation>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct NamedTypeAnnotation {
    #[resolve_field]
    #[parent_variant(NamedTypeAnnotation)]
    pub name: WithSpan<EntityNameWrapper>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    #[resolve_field]
    #[parent_variant(List)]
    pub inner: WithSpan<TypeAnnotation>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct NonNullTypeAnnotation {
    #[resolve_field]
    #[parent_variant(NonNull)]
    pub inner: WithSpan<NonNullInner>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum NonNullInner {
    Named(NamedTypeAnnotation),
    List(ListTypeAnnotation),
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Field(FieldDefinitionPath<'a>),
    InputValue(InputValueDefinitionPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    NonNull(Box<NonNullTypeAnnotationPath<'a>>),
}

pub type TypeAnnotationPath<'a> =
    PositionResolutionPath<&'a TypeAnnotation, TypeAnnotationParent<'a>>;
pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;
pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;
pub type NonNullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NonNullTypeAnnotation, TypeAnnotationParent<'a>>;
pub type NonNullInnerPath<'a> =
    PositionResolutionPath<&'a NonNullInner, TypeAnnotationParent<'a>>;
```

`String` is `Named`. `String!` is `NonNull` whose `inner` is `Named` spanning `String`. `[String]` is `List`. `[String]!` is `NonNull` whose `inner` is `List` spanning `[String]`. `[String!]` is `List` of `NonNull(Named)`.

The identifier's span is `NamedTypeAnnotation.name`. The outer `TypeAnnotation` span from `spanning` covers the identifier and, when present, `!` or the brackets. `NonNullTypeAnnotation.inner` spans the named type or list, not `!`. A position on `!` matches no child and is `NonNullTypeAnnotation`. A position on `[` / `]` is `ListTypeAnnotation`.

`TypeAnnotation` and `NonNullInner` always delegate to a payload, so they are not resolution leaves. `Box` in `TypeAnnotationParent::List` / `NonNull` is iso's pattern for a recursive parent.

## Constant value

```rust
// from crates/graphql_schema_parser/src/value.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub enum ConstantValue {
    Int(IntegerValue),
    Float(FloatValue),
    String(StringLiteralValueWrapper),
    Boolean(BooleanValue),
    Null(NullValue),
    Enum(#[parent_variant(ConstantValue)] EnumLiteralValueWrapper),
    List(ConstantList),
    Object(ConstantObject),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ConstantList(
    #[resolve_field]
    #[parent_variant(List)]
    pub Vec<WithSpan<ConstantValue>>,
);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ConstantObject(
    #[resolve_field]
    #[parent_variant(ConstantObject)]
    pub Vec<WithSpan<Argument>>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[derive(Copy, Clone, Eq, PartialEq, Hash, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct FloatValue(u64);

impl FloatValue {
    pub fn new(v: f64) -> Self {
        Self(v.to_bits())
    }
    pub fn as_float(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl From<f64> for FloatValue {
    fn from(value: f64) -> Self {
        FloatValue::new(value)
    }
}

impl std::fmt::Debug for FloatValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.as_float()))
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct NullValue;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ConstantValueParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct StringLiteralValueWrapper(pub StringLiteralValue);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EnumLiteralValueWrapperParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct EnumLiteralValueWrapper(pub EnumLiteralValue);

#[derive(Debug)]
pub enum ConstantValueParent<'a> {
    ArgumentValue(ArgumentPath<'a>),
    InputValueDefault(InputValueDefinitionPath<'a>),
    List(ConstantListPath<'a>),
}

#[derive(Debug)]
pub enum EnumLiteralValueWrapperParent<'a> {
    ConstantValue(ConstantValueParent<'a>),
    EnumValueDefinition(EnumValueDefinitionPath<'a>),
}

pub type ConstantValuePath<'a> =
    PositionResolutionPath<&'a ConstantValue, ConstantValueParent<'a>>;
pub type ConstantListPath<'a> =
    PositionResolutionPath<&'a ConstantList, ConstantValueParent<'a>>;
pub type ConstantObjectPath<'a> =
    PositionResolutionPath<&'a ConstantObject, ConstantValueParent<'a>>;
pub type IntegerValuePath<'a> =
    PositionResolutionPath<&'a IntegerValue, ConstantValueParent<'a>>;
pub type FloatValuePath<'a> =
    PositionResolutionPath<&'a FloatValue, ConstantValueParent<'a>>;
pub type BooleanValuePath<'a> =
    PositionResolutionPath<&'a BooleanValue, ConstantValueParent<'a>>;
pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, ConstantValueParent<'a>>;
pub type StringLiteralValueWrapperPath<'a> =
    PositionResolutionPath<&'a StringLiteralValueWrapper, ConstantValueParent<'a>>;
pub type EnumLiteralValueWrapperPath<'a> = PositionResolutionPath<
    &'a EnumLiteralValueWrapper,
    EnumLiteralValueWrapperParent<'a>,
>;
```

`FloatValue` is origin: `new(f64)` stores `to_bits`, `as_float` is `from_bits`. Origin also has `From<i64>` and `Display`; nothing in this crate reads those, so they are omitted. `Null` is a unit struct so it is a resolve leaf. A constant object's fields are `Argument` (same type as a directive argument).

## Wrappers

```rust
// from crates/graphql_schema_parser/src/ast.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntityNameWrapperParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct EntityNameWrapper(pub EntityName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableNameWrapperParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct SelectableNameWrapper(pub SelectableName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentNameWrapperParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct ArgumentNameWrapper(pub ArgumentName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = DirectiveNameWrapperParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct DirectiveNameWrapper(pub DirectiveName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = DescriptionParent<'a>, resolved_node = GraphQLResolutionNode<'a>)]
pub struct Description(pub DescriptionValue);

#[derive(Debug)]
pub enum EntityNameWrapperParent<'a> {
    ObjectTypeDefinition(ObjectTypeDefinitionPath<'a>),
    ObjectTypeExtension(ObjectTypeExtensionPath<'a>),
    ScalarTypeDefinition(ScalarTypeDefinitionPath<'a>),
    InterfaceTypeDefinition(InterfaceTypeDefinitionPath<'a>),
    InputObjectTypeDefinition(InputObjectTypeDefinitionPath<'a>),
    EnumTypeDefinition(EnumTypeDefinitionPath<'a>),
    UnionTypeDefinition(UnionTypeDefinitionPath<'a>),
    SchemaQuery(SchemaDefinitionPath<'a>),
    SchemaMutation(SchemaDefinitionPath<'a>),
    SchemaSubscription(SchemaDefinitionPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ObjectImplements(ObjectTypeDefinitionPath<'a>),
    ObjectExtensionImplements(ObjectTypeExtensionPath<'a>),
    InterfaceImplements(InterfaceTypeDefinitionPath<'a>),
    UnionMember(UnionTypeDefinitionPath<'a>),
}

#[derive(Debug)]
pub enum SelectableNameWrapperParent<'a> {
    FieldDefinition(FieldDefinitionPath<'a>),
}

#[derive(Debug)]
pub enum ArgumentNameWrapperParent<'a> {
    InputValueDefinition(InputValueDefinitionPath<'a>),
    Argument(ArgumentPath<'a>),
}

#[derive(Debug)]
pub enum DirectiveNameWrapperParent<'a> {
    DirectiveDefinition(DirectiveDefinitionPath<'a>),
    Directive(DirectivePath<'a>),
}

#[derive(Debug)]
pub enum DescriptionParent<'a> {
    ObjectTypeDefinition(ObjectTypeDefinitionPath<'a>),
    ScalarTypeDefinition(ScalarTypeDefinitionPath<'a>),
    InterfaceTypeDefinition(InterfaceTypeDefinitionPath<'a>),
    InputObjectTypeDefinition(InputObjectTypeDefinitionPath<'a>),
    EnumTypeDefinition(EnumTypeDefinitionPath<'a>),
    UnionTypeDefinition(UnionTypeDefinitionPath<'a>),
    DirectiveDefinition(DirectiveDefinitionPath<'a>),
    SchemaDefinition(SchemaDefinitionPath<'a>),
    FieldDefinition(FieldDefinitionPath<'a>),
    InputValueDefinition(InputValueDefinitionPath<'a>),
    EnumValueDefinition(EnumValueDefinitionPath<'a>),
}

pub type EntityNameWrapperPath<'a> =
    PositionResolutionPath<&'a EntityNameWrapper, EntityNameWrapperParent<'a>>;
pub type SelectableNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectableNameWrapper, SelectableNameWrapperParent<'a>>;
pub type ArgumentNameWrapperPath<'a> =
    PositionResolutionPath<&'a ArgumentNameWrapper, ArgumentNameWrapperParent<'a>>;
pub type DirectiveNameWrapperPath<'a> =
    PositionResolutionPath<&'a DirectiveNameWrapper, DirectiveNameWrapperParent<'a>>;
pub type DescriptionPath<'a> = PositionResolutionPath<&'a Description, DescriptionParent<'a>>;
```

Inner field `pub`. Construction is `token.interned().map(EntityNameWrapper)` for identifiers. Callers read `.0`. No `From<StringKey>` on the wrapper. These wrappers live in `graphql_schema_parser`, not `isograph_parser`. Same names, different types. A crate that uses both path-qualifies.

`EntityNameWrapper` on an implements list or union member list uses the `ObjectImplements` / `UnionMember` / sibling variants so a name in those vecs does not collide with the definition's own `name` field. `#[parent_variant]` is per field, including on `Vec<WithSpan<T>>` (the derive walks the vec and wraps each item). Two `EntityNameWrapper` fields on one struct are this case.

## Directive location

The derive only accepts enum variants with a single unnamed payload, so `DirectiveLocation` (unit variants, origin) does not derive `ResolvePosition`. `DirectiveLocationWrapper` is the resolve leaf, same shape as `BooleanValue(pub Boolean)`.

```rust
// from crates/graphql_schema_parser/src/ast.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, strum::EnumString, strum::Display)]
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

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash, ResolvePosition)]
#[resolve_position(
    parent_type = DirectiveDefinitionPath<'a>,
    resolved_node = GraphQLResolutionNode<'a>,
    on_unmatched_span = from_path
)]
pub struct DirectiveLocationWrapper(pub DirectiveLocation);

pub type DirectiveLocationWrapperPath<'a> =
    PositionResolutionPath<&'a DirectiveLocationWrapper, DirectiveDefinitionPath<'a>>;
```

Origin `graphql_sdl.rs` `DirectiveLocation`. `FromStr` via strum. Parse: `require_token(Identifier, Keyword)`, `DirectiveLocation::from_str(text)`, wrap `DirectiveLocationWrapper`; mismatch is `lexer.expected(Expectation::DirectiveLocation)`.

## Resolution node

Enums that always delegate to a payload are not leaves (iso has no `IsoLiteralItem` variant). `TypeSystemDefinition`, `TypeSystemExtension`, `TypeSystemExtensionOrDefinition`, `TypeAnnotation`, `NonNullInner`, and `ConstantValue` always delegate.

```rust
// from crates/graphql_schema_parser/src/resolution_node.rs
#[derive(Debug, derive_more::From)]
#[non_exhaustive]
pub enum GraphQLResolutionNode<'a> {
    TypeSystemDocument(TypeSystemDocumentPath<'a>),
    TypeSystemExtensionDocument(TypeSystemExtensionDocumentPath<'a>),
    ObjectTypeDefinition(ObjectTypeDefinitionPath<'a>),
    ObjectTypeExtension(ObjectTypeExtensionPath<'a>),
    ScalarTypeDefinition(ScalarTypeDefinitionPath<'a>),
    InterfaceTypeDefinition(InterfaceTypeDefinitionPath<'a>),
    InputObjectTypeDefinition(InputObjectTypeDefinitionPath<'a>),
    EnumTypeDefinition(EnumTypeDefinitionPath<'a>),
    UnionTypeDefinition(UnionTypeDefinitionPath<'a>),
    DirectiveDefinition(DirectiveDefinitionPath<'a>),
    SchemaDefinition(SchemaDefinitionPath<'a>),
    FieldDefinition(FieldDefinitionPath<'a>),
    InputValueDefinition(InputValueDefinitionPath<'a>),
    EnumValueDefinition(EnumValueDefinitionPath<'a>),
    Directive(DirectivePath<'a>),
    Argument(ArgumentPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    NonNullTypeAnnotation(NonNullTypeAnnotationPath<'a>),
    ConstantList(ConstantListPath<'a>),
    ConstantObject(ConstantObjectPath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    FloatValue(FloatValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
    StringLiteralValueWrapper(StringLiteralValueWrapperPath<'a>),
    EnumLiteralValueWrapper(EnumLiteralValueWrapperPath<'a>),
    EntityNameWrapper(EntityNameWrapperPath<'a>),
    SelectableNameWrapper(SelectableNameWrapperPath<'a>),
    ArgumentNameWrapper(ArgumentNameWrapperPath<'a>),
    DirectiveNameWrapper(DirectiveNameWrapperPath<'a>),
    Description(DescriptionPath<'a>),
    DirectiveLocationWrapper(DirectiveLocationWrapperPath<'a>),
}
```

A position on a keyword or `{` / `}` / `(` / `)` / `[` / `]` / `:` / `=` / `|` / `&` / `@` / `!` that is not a stored node resolves to the nearest parent whose spanning span covers it. `WithSpan<T>` delegates to `T` (iso `WithGenericLocation`). Tests call `document.resolve((), span)` on the `WithSpan<TypeSystemDocument>` `parse_schema` returns.

## Parse-only type

`RootOperationKind` is not stored. `parse_root_operation_type` uses it to fill `SchemaDefinition`'s three options.

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RootOperationKind {
    Query,
    Subscription,
    Mutation,
}
```
