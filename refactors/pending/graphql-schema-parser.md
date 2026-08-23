# GraphQL schema parser

Requires the iso parser (tokenize, match_brackets, chunk, consume). Independent of the Zed extension, iso-format.md, lsp-open-files.md.

`crates/graphql_schema_parser` turns GraphQL SDL into a tree. The pipeline is the iso parser's four stages: tokenize, match brackets, chunk, grammar consume. Functions are plain functions over `&str`. pico sits above (which schema files exist). Malformed input is a representable state. Semantic tokens record at consume. `ResolvePosition` on the tree.

Origin of the language: isograph `crates/graphql_schema_parser` and `crates/graphql_lang_types`. Origin of the stages: `crates/isograph_parser`. Delta: `WithSpan` (no `TextSource` / `WithEmbeddedLocation`); `report_error` instead of `?` that aborts the document; `expect` / `unwrap` / `assert` on `extend` gone; chunked lists for fields, input values, enum values, union members, directive locations, implements; `#` comments, `|`, and `&` are token kinds.

The type system document: `schema`, `scalar`, `type`, `interface`, `union`, `enum`, `input`, `directive`, and `extend type`. Executable documents (query / mutation / fragment) are a later parser.

## What the user does

A schema file:

```
type Pet {
  id: ID!
  name: String
}
```

`parse_schema` returns an object type `Pet` with fields `id: ID!` and `name: String`. A second definition after a junk token is still parsed. Hover/highlighting of `.graphql` files is a later LSP arm on this tree.

## Entry

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
pub fn parse_schema(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<GraphQLSemanticToken>>,
) -> WithSpan<SchemaDocument>
```

Same composition as `parse_iso_literal`: caller runs `tokenize`, `match_brackets`, `chunk`, then this. `text` is the file. The wrapper span is the document.

```rust
// from crates/graphql_schema_parser/src/lib.rs
pub fn parse_schema_file(text: &str) -> ParsedSchema {
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let root = chunk(match_brackets(tokenize(text)));
    let tree = parse_schema(text, root, &mut errors, &mut tokens);
    ParsedSchema { tree, errors, tokens }
}
```

`parse_schema_file` is the tests-and-compiler entry. The four pieces stay public so a later memo can intern parse of the text the way `parsed_iso_literal` does.

## Types

Most important first.

```rust
// from crates/graphql_schema_parser/src/schema_document.rs
pub struct SchemaDocument {
    pub definitions: Vec<WithSpan<Slot<TypeSystemDefinitionOrExtension, UnparsedChunkItems>>>,
}

pub enum TypeSystemDefinitionOrExtension {
    Definition(TypeSystemDefinition),
    Extension(TypeSystemExtension),
}

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

pub enum TypeSystemExtension {
    Object(ObjectTypeExtension),
}

pub struct ObjectTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub implements: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub fields: Option<WithSpan<FieldDefinitionList>>,
}

pub struct ObjectTypeExtension {
    pub name: WithSpan<EntityNameWrapper>,
    pub implements: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub fields: Option<WithSpan<FieldDefinitionList>>,
}

pub struct ScalarTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Option<WithSpan<DirectiveList>>,
}

pub struct InterfaceTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub implements: Vec<WithSpan<EntityNameWrapper>>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub fields: Option<WithSpan<FieldDefinitionList>>,
}

pub struct InputObjectTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub fields: Option<WithSpan<InputValueDefinitionList>>,
}

pub struct EnumTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub values: Option<WithSpan<EnumValueDefinitionList>>,
}

pub struct UnionTypeDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<EntityNameWrapper>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub members: Vec<WithSpan<EntityNameWrapper>>,
}

pub struct DirectiveDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<DirectiveNameWrapper>,
    pub arguments: Option<WithSpan<InputValueDefinitionList>>,
    pub repeatable: Option<Span>,
    pub locations: Vec<WithSpan<DirectiveLocation>>,
}

pub struct SchemaDefinition {
    pub description: Option<WithSpan<Description>>,
    pub directives: Option<WithSpan<DirectiveList>>,
    pub operations: Option<WithSpan<RootOperationList>>,
}

pub struct FieldDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<SelectableNameWrapper>,
    pub arguments: Option<WithSpan<InputValueDefinitionList>>,
    pub type_: WithSpan<TypeAnnotation>,
    pub directives: Option<WithSpan<DirectiveList>>,
}

pub struct InputValueDefinition {
    pub description: Option<WithSpan<Description>>,
    pub name: WithSpan<InputValueNameWrapper>,
    pub type_: WithSpan<TypeAnnotation>,
    pub default: Option<WithSpan<ConstantValue>>,
    pub directives: Option<WithSpan<DirectiveList>>,
}

pub struct EnumValueDefinition {
    pub description: Option<WithSpan<Description>>,
    pub value: WithSpan<EnumLiteralValue>,
    pub directives: Option<WithSpan<DirectiveList>>,
}

pub struct RootOperation {
    pub kind: WithSpan<RootOperationKind>,
    pub type_: WithSpan<EntityNameWrapper>,
}

pub enum RootOperationKind {
    Query,
    Mutation,
    Subscription,
}

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

pub struct FieldDefinitionList(pub Vec<WithSpan<Slot<FieldDefinition, UnparsedChunkItems>>>);
pub struct InputValueDefinitionList(pub Vec<WithSpan<Slot<InputValueDefinition, UnparsedChunkItems>>>);
pub struct EnumValueDefinitionList(pub Vec<WithSpan<Slot<EnumValueDefinition, UnparsedChunkItems>>>);
pub struct RootOperationList(pub Vec<WithSpan<Slot<RootOperation, UnparsedChunkItems>>>);

pub struct DirectiveList(pub Vec<WithSpan<Directive>>);

pub struct Directive {
    pub name: WithSpan<DirectiveNameWrapper>,
    pub arguments: Option<WithSpan<ArgumentList>>,
}

pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    NonNull(Box<NonNullTypeAnnotation>),
}

pub struct NamedTypeAnnotation {
    pub name: WithSpan<EntityNameWrapper>,
}

pub struct ListTypeAnnotation {
    pub inner: WithSpan<TypeAnnotation>,
}

pub enum NonNullTypeAnnotation {
    Named(NamedTypeAnnotation),
    List(ListTypeAnnotation),
}

pub enum ConstantValue {
    Int(IntegerValue),
    Float(FloatValue),
    String(StringLiteralValueWrapper),
    Boolean(BooleanValue),
    Null(NullValue),
    Enum(EnumLiteralValue),
    List(ConstantList),
    Object(ConstantObject),
}
```

`Description`, `EntityNameWrapper`, `SelectableNameWrapper` are the iso parser's wrappers. `DirectiveNameWrapper(pub DirectiveName)` and `InputValueNameWrapper(pub InputValueName)` are the same shape. `EnumLiteralValue` is `string_key_newtype!` in `common_lang_types` if it is not already there.

`ArgumentList` is iso's `ArgumentList` with `ConstantValue` in place of `NonConstantValue` (no `$`).

`FloatValue` is a newtype over a parsed float token's interned text (GraphQL floats are not `f64` identity). The printer and later passes read `.0`.

Lists of definitions at the document root are chunks of the root level. Field lists are chunks of a `{` group. Input-value lists are chunks of `{` (input object) or `(` (field arguments, directive arguments). Enum values are chunks of `{`. Union members and implements names and directive locations are `|` / `&`-separated; those separators are tokens in the chunk boundary, same as comma (see tokenize). Root operations are chunks of the `schema { }` group.

## Tokenize

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
pub enum GraphQLTokenKind {
    Error,
    LineBreak,
    Comment,
    Identifier,
    IntegerLiteral,
    FloatLiteral,
    StringLiteral,
    BlockStringLiteral,
    At,
    Colon,
    Equals,
    Exclamation,
    Pipe,
    Ampersand,
    OpenBrace,
    CloseBrace,
    OpenParenthesis,
    CloseParenthesis,
    OpenBracket,
    CloseBracket,
    Comma,
    EndOfFile,
    ErrorUnterminatedString,
    ErrorUnsupportedStringCharacter,
    ErrorUnterminatedBlockString,
}
```

`#` to end of line is `Comment`. Skip is `[ \t\f\ufeff]+`, same as iso. Newlines are `LineBreak` and chunk separators. `|` and `&` are separator tokens inside union members, directive locations, and implements lists, the way comma is a separator in field lists.

Brackets are `{` `}` `(` `)` `[` `]`. `match_brackets` and `chunk` are the iso algorithms over these kinds. Copy `matched_brackets.rs`, `chunk.rs`, `chunk_stream.rs` from `isograph_parser`, origin named, delta: `GraphQLTokenKind` in place of `NonBracketTokenKind` / `IsographLangTokenKind`; `|` and `&` join comma as `ChunkSeparator` in implements / union / directive-location levels.

`ItemCursor::require_token` / `require_group` take a `GraphQLSemanticToken`.

```rust
// from crates/graphql_schema_parser/src/semantic_token.rs
pub enum GraphQLSemanticToken {
    Keyword,
    Type,
    FieldName,
    Argument,
    DirectiveName,
    ScalarName,
    EnumValue,
    Description,
    Integer,
    Float,
    String,
    BooleanOrNull,
    Colon,
    Equals,
    Pipe,
    Ampersand,
    Bang,
    Brace,
    Parenthesis,
    Bracket,
    Content,
    Comment,
    Error,
}
```

`!` records `Bang`. Descriptions record `Description`. Keywords: `type`, `scalar`, `interface`, `union`, `enum`, `input`, `directive`, `schema`, `extend`, `implements`, `on`, `repeatable`, `query`, `mutation`, `subscription`.

## Grammar

Root level: `parse_each_chunk` of a definition or extension.

A definition chunk:

1. Optional description (string or block string).
2. Keyword.
3. Body for that keyword.

`extend` then `type` then the object-extension body (name, implements, directives, fields).

`implements` is `implements` then optional `&` then names separated by `&`.

Union members: `=` then optional `|` then names separated by `|`.

Directive definition: `@` name, optional `(...)` input values, optional `repeatable`, `on`, optional `|`, locations separated by `|`.

Schema: optional directives, optional `{` root operations `}`. A root operation is `query` / `mutation` / `subscription` `:` Type.

Field: optional description, name, optional `(...)` input values, `:`, type annotation, optional directives.

Input value: optional description, name, `:`, type annotation, optional `=` constant value, optional directives.

Type annotation: `Name`, `Name!`, `[T]`, `[T]!`, `[T!]`, `[T!]!`, nested. `!` is consumed and recorded (`Bang`). The tree is `Named` / `List` / `NonNull` as above.

Constant values: int, float, string, block string, `true` / `false` / `null`, enum name, `[...]`, `{ key: value }`.

One chunk, one item. A field is one chunk of its brace group. Junk after a field is `Slot.extra`. A failed field is `item: None`.

## Changes

Four independently shippable changes.

### Change 1: crate, tokenize

`crates/graphql_schema_parser` workspace member. `GraphQLTokenKind`, `tokenize`, tests:

- `type Pet { id: ID! }` is identifier, identifier, `{`, identifier, `:`, identifier, `!`, `}`
- `# comment\n` is `Comment` then `LineBreak`
- `|` and `&` are those kinds
- skip spaces, keep newlines

### Change 2: match_brackets and chunk

Copy from iso, GraphQL kinds. Tests: `{ id: ID! }` is one brace group whose interior chunks as one item; `type Pet { id: ID! } type User { id: ID! }` is two root chunks.

`|` / `&` / comma are list separators at the levels that use them.

### Change 3: `parse_schema`

The types above, consume catalog, `parse_schema` / `parse_schema_file`. Tests (tree facts, no snapshots):

- `type Pet { id: ID! }` is an object `Pet` with one field `id` whose type is non-null named `ID`
- `scalar Date` is a scalar
- `interface Node { id: ID! }` is an interface with one field
- `enum Status { AVAILABLE, SOLD }` is an enum with two values
- `union Search = Pet | User` is a union with two members
- `input PetInput { name: String }` is an input object with one input value
- `directive @oneOf on INPUT_OBJECT` is a directive definition with one location
- `schema { query: Query }` has `query: Query`
- `extend type Pet { nickname: String }` is an object extension
- `"a pet" type Pet { id: ID! }` has description `a pet`
- `type Pet { id: ID!, name: String }` two fields
- junk between two `type` definitions: two definition slots, errors nonempty, both types present
- empty file: empty `definitions`

Semantic tokens at consume. A `type` keyword is `Keyword`. `Pet` is `Type`. `id` is `FieldName`. `ID` is `Type`. `!` is `Bang`.

### Change 4: `ResolvePosition`

`#[derive(ResolvePosition)]` on the document types. Resolution node enum in this crate. Tests: a position on `id` in `type Pet { id: ID! }` is the field name; on `Pet` is the type name.

## Call sites

- compiler memo over a schema `DiskFile` (later, schema ingest)
- LSP semantic tokens / hover for `.graphql` (later)
- tests call `parse_schema_file`

## Docs this change amends

pluggable-compiler.md `NetworkProtocol::parse_schema`: this crate is the GraphQL implementor's parse.

pico.md: schema file text is a source; `parse_schema` is a function over `&str`.
