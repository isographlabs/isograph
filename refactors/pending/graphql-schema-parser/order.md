# GraphQL schema parser: order

Requires parsing-standards.md and ast.md. This file is the work list. Each item is independently shippable. An agent implements the next pending item, with the tests named here, then sets that item's Status to done. When every item is done, `parse_schema` / `parse_schema_extensions` cover ast.md and parsing-standards.md.

Origin of each parse function: isograph `crates/graphql_schema_parser/src/parse_schema.rs` and `description.rs`, control flow copied. Delta: primitives in parsing-standards.md; types in ast.md.

Crate `crates/graphql_schema_parser`, workspace member via `./crates/*`. No other crate's `Cargo.toml` lists it. Tests are `#[cfg(test)]` in this crate, calling `parse_schema` / `parse_schema_extensions` on string fixtures. That is the only use.

```toml
# from crates/graphql_schema_parser/Cargo.toml
[package]
name = "graphql_schema_parser"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]
common_lang_types = { path = "../common_lang_types" }
derive_more = { workspace = true }
intern = { path = "../../relay-crates/intern" }
logos = { workspace = true }
prelude = { path = "../prelude" }
resolve_position = { path = "../resolve_position" }
resolve_position_macros = { path = "../resolve_position_macros" }
span = { path = "../span" }
strum = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

`lib.rs` modules: `token_kind`, `peekable_lexer`, `parse_error`, `semantic_token`, `ast`, `type_annotation`, `value`, `description`, `parse_schema`, `resolution_node`. Public: `parse_schema`, `parse_schema_extensions`, the AST types, `ParseError` / `Expectation` / `ExpectedFound` / `Found`, `GraphQLSemanticToken`, `GraphQLResolutionNode`. `#[cfg(test)]`: `parse_schema_with_tokens`, `parse_schema_extensions_with_tokens`.

## 1. TokenKind

Status: pending

`token_kind.rs` as parsing-standards.md: `TokenKind`, `TokenKindExtras`, `StringToken`, `BlockStringToken`, `lex_string`, `lex_block_string`. No `tokenize() -> Vec` used by the grammar.

```rust
// from crates/graphql_schema_parser/src/token_kind.rs
#[cfg(test)]
fn lexed_kinds(text: &str) -> Vec<TokenKind>
```

`lexed_kinds` iterates `TokenKind::lexer(text)`, taking `extras.error_token` the same way `parse_token` does. It does not emit `EndOfFile`.

Tests:

- `type Pet { id: ID! }` kinds: Identifier, Identifier, OpenBrace, Identifier, Colon, Identifier, Exclamation, CloseBrace.
- `# hi\ntype` is Identifier `type` (comment skipped). Confirm by lexing and taking the first kind, and by `source` of that token being `type`.
- `a, b` is Identifier, Identifier.
- `"hi"` is StringLiteral. `"""hi"""` is BlockStringLiteral.
- empty string is empty vec.
- `"unterminated` is ErrorUnterminatedString.
- `"""unterminated` is ErrorUnterminatedBlockString.
- `"\x"` (unsupported string character) is ErrorUnsupportedStringCharacter.
- `01` is ErrorNumberLiteralLeadingZero.
- `.5` is ErrorFloatLiteralMissingZero.
- `1.` and `1a` are ErrorNumberLiteralTrailingInvalid.

## 2. PeekableLexer

Status: pending

`TokenText`, `PeekableLexer`, `ParseError` / `Expectation` / `ExpectedFound` / `Found`, `GraphQLSemanticToken` as parsing-standards.md. `parse_token` / `peek` / `reached_eof` / `source` / `spanning` / `expected` / `consume_token_if` / `require_token` / `consume_matching_identifier` / `require_matching_identifier`.

Tests (construct `PeekableLexer::new` with a local tokens vec):

- `require_token(Identifier, Type)` on `Pet` returns text `Pet`, location `0..3`. The tokens vec is one `Type` at that span.
- `require_token(Colon, Colon)` on `Pet` is `Err(())`; `expected(Expectation::Token(Colon))` displays `Expected colon (':'), found non-variable identifier (e.g. 'x' or 'Foo').`; cursor still on `Pet`.
- `consume_token_if(Colon, Colon)` on `Pet` is `None`, cursor still on `Pet`, tokens vec empty.
- `require_matching_identifier("type", Keyword)` on `type` succeeds; on `scalar` is `Err(())`.
- `reached_eof` after consuming the only token of `Pet`.
- empty string: `reached_eof` is true immediately.
- `spanning` of `require_token(Identifier, Type)` on `Pet` is `0..3`.
- `"hi` : peek is `ErrorUnterminatedString`.

## 3. Scalar, empty document, description, unknown keyword

Status: pending

```rust
// from crates/graphql_schema_parser/src/lib.rs
pub fn parse_schema(text: &str) -> Result<WithSpan<TypeSystemDocument>, WithSpan<ParseError>>
fn parse_schema_inner(
    text: &str,
    tokens: &mut Vec<WithSpan<GraphQLSemanticToken>>,
) -> Result<WithSpan<TypeSystemDocument>, WithSpan<ParseError>>
#[cfg(test)]
pub fn parse_schema_with_tokens(
    text: &str,
) -> Result<(WithSpan<TypeSystemDocument>, Vec<WithSpan<GraphQLSemanticToken>>), WithSpan<ParseError>>

// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_type_system_document(
    lexer: &mut PeekableLexer<'_>,
) -> Result<TypeSystemDocument, WithSpan<ParseError>>
fn parse_type_system_definition(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<TypeSystemDefinition>, WithSpan<ParseError>>
fn parse_scalar_type_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ScalarTypeDefinition, WithSpan<ParseError>>
fn parse_constant_directives(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<Directive>>, WithSpan<ParseError>>

// from crates/graphql_schema_parser/src/description.rs
pub(crate) fn parse_optional_description(
    lexer: &mut PeekableLexer<'_>,
) -> Option<WithSpan<Description>>
fn clean_block_string_literal(source: &str) -> String
```

`parse_type_system_document`: while not eof, push `parse_type_system_definition`. `parse_type_system_definition` is `spanning`: optional description, `require_token(Identifier, Keyword)`, match `scalar` (other keywords land in later items; for this item they are `expected(Definition)`), else `expected(Definition)`. `parse_scalar_type_definition`: name `require_token(Identifier, Type)` interned as `EntityNameWrapper`, directives (this item: `consume_token_if(At, DirectiveName)` is none, empty vec).

`clean_block_string_literal` copied from origin `description.rs`.

Tests:

- `scalar Date` is one `Scalar` named `Date`, no description, empty directives.
- `""` is empty `TypeSystemDocument`, spanning span zero-width at 0.
- `foo` is `Expected(Definition, Token(Identifier))`.
- `"a date" scalar Date` description lookup `a date`; description span includes the quotes (0..8).
- `"""a date""" scalar Date` description lookup `a date`.
- `"""\n  hello\n  world\n""" scalar Date` description lookup `hello\nworld` (origin indent rule).
- `{` is `Expected(Definition, Token(OpenBrace))`.
- `scalar` with no name is `Expected Token(Identifier), found end of file`.

## 4. Type annotation and object fields

Status: pending

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_object_type_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ObjectTypeDefinition, WithSpan<ParseError>>
fn parse_implements_interfaces_if_present(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>>
fn parse_optional_fields(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<FieldDefinition>>, WithSpan<ParseError>>
fn parse_field(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<FieldDefinition>, WithSpan<ParseError>>
fn parse_type_annotation(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>>
fn parse_optional_enclosed_items<T>(
    lexer: &mut PeekableLexer<'_>,
    open: TokenKind,
    close: TokenKind,
    open_token: GraphQLSemanticToken,
    close_token: GraphQLSemanticToken,
    parse: impl FnMut(&mut PeekableLexer<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>>
fn parse_argument_definition(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<InputValueDefinition>, WithSpan<ParseError>>
```

`parse_type_system_definition` match arm `"type"` → `parse_object_type_definition`. After the `type` keyword: name, `parse_implements_interfaces_if_present` (this item: no `implements`, empty vec), directives empty, `parse_optional_fields`.

`parse_type_annotation` is `spanning` (this outer span includes `!` when present). Peek `Identifier`: intern as `EntityNameWrapper` (`Type` token) with that identifier's span; optional `!` (`Bang`). If bang, `TypeAnnotation::NonNull(NonNullTypeAnnotation { inner: NonNullInner::Named(...).with_span(identifier_span) })`. Else `TypeAnnotation::Named`. Peek `OpenBracket`: parse the inner annotation, `require_token(CloseBracket, Bracket)`; the list span is the spanning span of `[` through `]`; optional `!`. If bang, `NonNull` whose `inner` is `NonNullInner::List` at the list span. Else `TypeAnnotation::List`. Else `expected(TypeAnnotation)`. After a present `[`, the close bracket is required (origin: do not fall through if `[` was consumed).

`parse_optional_fields` is `parse_optional_enclosed_items` with braces and `parse_field`. `parse_field` is `spanning`: optional description, name (`FieldName`), `parse_optional_enclosed_items` with parens and `parse_argument_definition` (arguments), `require_token(Colon, Colon)`, type annotation, directives empty for now.

`parse_argument_definition` is `spanning`: optional description, name (`Argument`), colon, type annotation, `default_value: None` (item 5 consumes `=` and parses the constant), directives empty.

Tests:

- `type Pet { id: ID! }` object `Pet`, one field `id`, type `NonNull(Named(ID))`. Field arguments empty.
- `type Pet { name: String }` named `String`.
- `type Pet { tags: [String!]! }` non-null list of non-null named `String`.
- `type Pet` fields empty vec.
- `type Pet { }` errors (`Expected Token(Identifier), found closing brace ('}')`).
- `type Pet { id: ID!, name: String }` two fields.
- `type Pet { name(lang: String): String }` one argument `lang`, type named `String`, no default.
- `type Pet { name(): String }` errors (present `(` requires a first argument).
- `type Pet { "a field" id: ID! }` field description lookup `a field`.
- `type Pet {` errors (first field parse at EOF).
- `type Pet { id: ID!` errors (next field parse at EOF).
- `type` with no name errors.
- `[` as a field type with no close: `type Pet { x: [String }` errors on `require_token(CloseBracket)`.

## 5. Directives on types and fields, constant values

Status: pending

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_constant_directives(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<Directive>>, WithSpan<ParseError>>
fn parse_optional_constant_arguments(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<Argument>>, WithSpan<ParseError>>
fn parse_constant_name_value_pair(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<Argument>, WithSpan<ParseError>>
fn parse_constant_value(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>>
fn parse_optional_constant_default_value(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Option<WithSpan<ConstantValue>>, WithSpan<ParseError>>
```

`parse_constant_directives`: while peek is `At`, `spanning` around: `consume_token_if(At, DirectiveName)`, name `require_token(Identifier, DirectiveName)` interned as `DirectiveNameWrapper`, `arguments: parse_optional_constant_arguments`. The `Directive` span includes `@`. `parse_optional_constant_arguments` is `parse_optional_enclosed_items` with parens (required first argument if `(` present).

`parse_constant_name_value_pair` is `spanning`: name (`Argument`) interned as `ArgumentNameWrapper`, colon, `parse_constant_value`.

`parse_constant_value` is `spanning` and the peek dispatch in parsing-standards.md. Integer: `require_token(IntegerLiteral, Integer)`, `text().parse::<i64>()`, `IntegerDoesNotFitI64` at the token span on failure. Float: `require_token(FloatLiteral, Float)`, `text().parse::<f64>()` then `FloatValue::new`, `InvalidFloat` at the token span on `Err`. String: `StringLiteral`, interior interned, `String` token; span includes quotes. `true` / `false` / `null`: `require_matching_identifier` with `BooleanOrNull`. Other identifier: `Enum` / `EnumValue`. List: `OpenBracket` then while not close, push `parse_constant_value` (empty allowed). Object: `OpenBrace` then while not close, push `parse_constant_name_value_pair` (empty allowed).

`parse_optional_constant_default_value`: if no `=`, `None`; if `=`, `parse_constant_value` wrapped `Some`. `parse_argument_definition` calls it after the type annotation.

Tests:

- `type Pet @foo { id: ID! }` one directive `foo`, no args.
- `type Pet @foo(x: 1) { id: ID! }` argument `x` int 1.
- `type Pet { id: ID! @bar }` field directive `bar`.
- `type Pet @foo(x: 1) @bar { id: ID! }` two directives.
- `type Pet @foo(xs: [1, 2]) { id: ID! }` list of two ints.
- `type Pet @foo(xs: []) { id: ID! }` empty list.
- `type Pet @foo(o: { a: true }) { id: ID! }` object field `a` boolean true.
- `type Pet @foo(o: {}) { id: ID! }` empty object.
- `type Pet @foo(x: false, y: null, z: AVAILABLE) { id: ID! }` boolean false, null, enum `AVAILABLE`.
- `type Pet @foo(x: "hi") { id: ID! }` string lookup `hi`.
- `type Pet @foo(x: 1.5) { id: ID! }` float `as_float() == 1.5`.
- `type Pet { name(lang: String = "en"): String }` default string `en`.
- `type Pet @foo(x: """no""") { id: ID! }` is `Expected(ConstantValue, Token(BlockStringLiteral))`.
- `type Pet @foo() { id: ID! }` errors (required first argument).
- integer `9223372036854775808` is `IntegerDoesNotFitI64`.
- `type Pet @foo` (at EOF after the directive, no fields) is a valid object with empty fields.

## 6. Interface, input, enum, union

Status: pending

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_interface_type_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InterfaceTypeDefinition, WithSpan<ParseError>>
fn parse_input_object_type_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InputObjectTypeDefinition, WithSpan<ParseError>>
fn parse_enum_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<EnumTypeDefinition, WithSpan<ParseError>>
fn parse_enum_value_definitions(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<EnumValueDefinition>>, WithSpan<ParseError>>
fn parse_enum_value_definition(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<EnumValueDefinition>, WithSpan<ParseError>>
fn parse_union_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<UnionTypeDefinition, WithSpan<ParseError>>
fn parse_union_member_types(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>>
fn parse_implements_interfaces_if_present(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>>
fn parse_interfaces(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>>
```

Keyword arms: `interface`, `input`, `enum`, `union`. `parse_implements_interfaces_if_present`: `consume_matching_identifier("implements", Keyword)` then `parse_interfaces`, else empty. `parse_interfaces`: optional leading `&` (`Ampersand`), required first identifier (`Type`), while `consume_token_if(Ampersand, Ampersand)` push another identifier.

`parse_enum_value_definitions` is `parse_optional_enclosed_items` with braces. `parse_enum_value_definition` is `spanning`: optional description, identifier (`EnumValue`); if text is `true` / `false` / `null`, `EnumValueCannotBeTrueFalseOrNull` at that span; else intern `EnumLiteralValueWrapper`; directives.

`parse_union_definition`: name, directives, `require_token(Equals, Equals)`, `parse_union_member_types`. `parse_union_member_types`: optional leading `|` (`Pipe`), required first identifier (`Type`), while `consume_token_if(Pipe, Pipe)` push another.

`parse_input_object_type_definition`: name, directives, `parse_optional_enclosed_items` with braces and `parse_argument_definition`.

Tests:

- `interface Node { id: ID! }` interface `Node` one field `id`.
- `type Pet implements Node { id: ID! }` implements `[Node]`.
- `type Pet implements Foo & Bar { id: ID! }` two implements.
- `type Pet implements & Node { id: ID! }` one implement `Node`.
- `type Pet implements { id: ID! }` errors (required first interface name).
- `input PetInput { name: String }` one input value `name`.
- `input PetInput { name: String = "x" }` default string `x`.
- `input PetInput` fields empty.
- `input PetInput { }` errors.
- `enum Status { AVAILABLE SOLD }` two values.
- `enum Status` values empty.
- `enum Status { true }` is `EnumValueCannotBeTrueFalseOrNull`.
- `enum Status { false }` and `enum Status { null }` same error.
- `union Search = Pet | User` members `Pet`, `User`.
- `union Search = | Pet | User` same (leading pipe).
- `union Search = Pet` one member.
- `union Search =` errors (required first member).
- `interface Node implements Foo { id: ID! }` interface implements `[Foo]`.

## 7. Directive definition and schema definition

Status: pending

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_directive_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<DirectiveDefinition, WithSpan<ParseError>>
fn parse_directive_locations(
    lexer: &mut PeekableLexer<'_>,
) -> Result<Vec<WithSpan<DirectiveLocationWrapper>>, WithSpan<ParseError>>
fn parse_directive_location(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<DirectiveLocationWrapper>, WithSpan<ParseError>>
fn parse_schema_definition(
    lexer: &mut PeekableLexer<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<SchemaDefinition, WithSpan<ParseError>>
fn parse_root_operation_type(
    lexer: &mut PeekableLexer<'_>,
) -> Result<(WithSpan<RootOperationKind>, WithSpan<EntityNameWrapper>), WithSpan<ParseError>>
fn reassign_or_error(
    root_type: &mut Option<WithSpan<EntityNameWrapper>>,
    kind: WithSpan<RootOperationKind>,
    object_name: WithSpan<EntityNameWrapper>,
) -> Result<(), WithSpan<ParseError>>
```

Keyword arms: `directive`, `schema`.

`parse_directive_definition`: `consume_token_if(At, DirectiveName)` (optional; origin ignores a missing `@`), name (`DirectiveName`), `parse_optional_enclosed_items` with parens for arguments, `consume_matching_identifier("repeatable", Keyword)` mapped to `Some(span)` / `None`, `require_matching_identifier("on", Keyword)`, `parse_directive_locations`.

`parse_directive_locations`: optional leading `|`, required first `parse_directive_location`, while `consume_token_if(Pipe, Pipe)` push another. `parse_directive_location`: `require_token(Identifier, Keyword)` then `DirectiveLocation::from_str(text)` wrapped `DirectiveLocationWrapper`; mismatch `expected(DirectiveLocation)`.

`parse_schema_definition`: directives, `require_token(OpenBrace, Brace)`, required first `parse_root_operation_type` assigned into the matching option, while `consume_token_if(CloseBrace, Brace).is_none()` parse another and `reassign_or_error`. `parse_root_operation_type`: identifier (`Keyword`) match `query` / `mutation` / `subscription` else `expected(RootOperation)`, colon, type name (`Type`). `reassign_or_error`: if the option is `Some`, `DuplicateRootOperation` at the second keyword span; else assign.

Tests:

- `directive @oneOf on INPUT_OBJECT` name `oneOf`, one location `InputObject`, `repeatable` is `None`, arguments empty.
- `directive oneOf on INPUT_OBJECT` same (optional `@`).
- `directive @foo on FIELD | OBJECT` two locations.
- `directive @foo on | FIELD | OBJECT` same (leading pipe).
- `directive @foo repeatable on FIELD` `repeatable` is `Some` covering the keyword `repeatable`.
- `directive @foo(x: Int) on FIELD` one argument `x`.
- `directive @foo(x: Int = 1) on FIELD` argument default int 1.
- `directive @foo() on FIELD` errors (present `(` requires a first argument).
- `directive @foo on FOO` is `Expected(DirectiveLocation, Token(Identifier))`.
- `directive @foo on {` is `Expected Token(Identifier), found open brace ('{')` (require Identifier first). Origin's non-identifier message is `Expected directive location, found …`; delta: Identifier is required, then `from_str`.
- `directive @foo on` and `directive @foo on |` error (required first location).
- `schema { query: Query }` `query` is `Query`, mutation and subscription `None`.
- `schema { query: Query mutation: Mutation }` both set.
- `schema { query: Query query: Other }` `DuplicateRootOperation` at the second `query`.
- `schema { }` errors (required first root operation).
- `schema` with no braces errors (`Expected Token(OpenBrace)`).
- `schema { foo: Query }` is `Expected(RootOperation, Token(Identifier))`.
- `schema @foo { query: Query }` one directive on the schema.

## 8. Extend and `parse_schema_extensions`

Status: pending

```rust
// from crates/graphql_schema_parser/src/lib.rs
pub fn parse_schema_extensions(
    text: &str,
) -> Result<WithSpan<TypeSystemExtensionDocument>, WithSpan<ParseError>>
fn parse_schema_extensions_inner(
    text: &str,
    tokens: &mut Vec<WithSpan<GraphQLSemanticToken>>,
) -> Result<WithSpan<TypeSystemExtensionDocument>, WithSpan<ParseError>>
#[cfg(test)]
pub fn parse_schema_extensions_with_tokens(
    text: &str,
) -> Result<
    (
        WithSpan<TypeSystemExtensionDocument>,
        Vec<WithSpan<GraphQLSemanticToken>>,
    ),
    WithSpan<ParseError>,
>

// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_type_system_extension_document(
    lexer: &mut PeekableLexer<'_>,
) -> Result<TypeSystemExtensionDocument, WithSpan<ParseError>>
fn parse_type_system_extension(
    lexer: &mut PeekableLexer<'_>,
) -> Result<WithSpan<TypeSystemExtension>, WithSpan<ParseError>>
fn parse_object_type_extension(
    lexer: &mut PeekableLexer<'_>,
) -> Result<ObjectTypeExtension, WithSpan<ParseError>>
fn peek_type_system_doc_type(
    lexer: &PeekableLexer<'_>,
) -> Result<TypeSystemDocType, WithSpan<ParseError>>

enum TypeSystemDocType {
    Definition,
    Extension,
}
```

`peek_type_system_doc_type` does not consume. Peek `StringLiteral` or `BlockStringLiteral` → `Definition`. Peek `Identifier`: source `extend` → `Extension`, anything else → `Definition`. Else `expected(Definition)`. Origin's peek-error message lists `extend` among the definition keywords. `Definition` does not; `extend` is classified before that arm.

`parse_type_system_extension_document`: while not eof, peek, then `Definition` → `parse_type_system_definition` wrapped `TypeSystemExtensionOrDefinition::Definition`, or `Extension` → `parse_type_system_extension` wrapped `Extension`.

`parse_type_system_extension` is `spanning`: `require_matching_identifier("extend", Keyword)`, then `require_token(Identifier, Keyword)`, match `"type"` → `parse_object_type_extension` else `expected(ExtensionBody)`. Origin uses `expect` / `assert` that the first identifier is `extend`; we `require_matching_identifier`.

`parse_object_type_extension`: name, implements, directives, fields. No description (origin).

`parse_schema("extend type Pet { id: ID! }")` goes through `parse_type_system_definition`, identifier `extend`, `expected(Definition)`.

Tests:

- `extend type Pet { nickname: String }` object extension `Pet` one field `nickname`.
- `type Pet { id: ID! } extend type Pet { nickname: String }` via `parse_schema_extensions`: one `Definition` (object `Pet`) and one `Extension`.
- `parse_schema` of `extend type Pet { id: ID! }` is `Expected(Definition, Token(Identifier))`.
- `extend foo` is `Expected(ExtensionBody, Token(Identifier))`.
- `extend` at EOF is `Expected Token(Identifier), found end of file` (the body keyword).
- `"desc" extend type Pet { id: ID! }` via `parse_schema_extensions` is `Expected(Definition, Token(Identifier))` (peek string → definition, then identifier `extend`).
- `extend type Pet` fields empty.
- `extend type` errors (required name).
- `parse_schema_extensions("")` is empty `TypeSystemExtensionDocument`.
- `scalar Date extend type Pet { id: ID! }` via `parse_schema_extensions`: definition then extension.

## 9. Semantic tokens

Status: pending

Recording is already at every consume from item 2. This item adds tests that the recorded vec matches the call-site list in parsing-standards.md. Copy iso `assert_semantic_tokens` into this crate (`#[cfg(test)]`), same sequential-search behavior.

Tests via `parse_schema_with_tokens`:

- `type Pet { id: ID! }` tokens: `Keyword` `type`, `Type` `Pet`, `Brace` `{`, `FieldName` `id`, `Colon` `:`, `Type` `ID`, `Bang` `!`, `Brace` `}`.
- `@foo` on `type Pet @foo { id: ID! }`: `DirectiveName` `@`, `DirectiveName` `foo`.
- `"desc"` before `type` is `Description` covering `"desc"` including quotes.
- `"""desc"""` before `type` is `Description` covering the block string.
- `implements` / `&` / implement name: `Keyword`, `Ampersand`, `Type`.
- `directive @foo repeatable on FIELD`: `Keyword` `directive`, `DirectiveName` `@`, `DirectiveName` `foo`, `Keyword` `repeatable`, `Keyword` `on`, `Keyword` `FIELD`.
- `schema { query: Query }`: `Keyword` `schema`, `Brace` `{`, `Keyword` `query`, `Colon` `:`, `Type` `Query`, `Brace` `}`.
- enum value `AVAILABLE` is `EnumValue`.
- constant `1` / `1.5` / `"hi"` / `true` are `Integer` / `Float` / `String` / `BooleanOrNull`.
- argument name `x` in `@foo(x: 1)` is `Argument`.
- failed parse records tokens consumed before the error and nothing for the failed require (`type` then EOF: one `Keyword`).

## 10. ResolvePosition

Status: pending

`#[derive(ResolvePosition)]` on every AST type as ast.md. `GraphQLResolutionNode` as ast.md. Tests call `parse_schema` then `document.resolve((), span)` on the `WithSpan<TypeSystemDocument>` (`WithSpan` delegates to the item; iso tests call `parse.resolve((), span_of(text, "id"))`). Copy `span_of` from `crates/isograph_parser/src/parsed_items.rs` (the pattern occurs exactly once; `expect` is the fixture invariant that test established). Assert the node's variant and `.0.lookup()` where it is a wrapper:

- `type Pet { id: ID! }`, position on `id`: `SelectableNameWrapper`, lookup `id`.
- on `Pet`: `EntityNameWrapper`, lookup `Pet`.
- on `ID`: `EntityNameWrapper`, lookup `ID`.
- on `!`: `NonNullTypeAnnotation`.
- on `[` in `type Pet { tags: [String]! }`: `ListTypeAnnotation`.
- on `type`: `ObjectTypeDefinition` (keyword is not a leaf).
- on `{`: `ObjectTypeDefinition`.
- `"a date" scalar Date`, position on `a date` (interior of the description): `Description`.
- `type Pet @foo(x: 1) { id: ID! }`, on `foo`: `DirectiveNameWrapper`; on `x`: `ArgumentNameWrapper`; on `1`: `IntegerValue`.
- `enum Status { AVAILABLE }`, on `AVAILABLE`: `EnumLiteralValueWrapper`.
- `directive @foo on FIELD`, on `FIELD`: `DirectiveLocationWrapper` holding `DirectiveLocation::Field`.
- `schema { query: Query }`, on `Query`: `EntityNameWrapper` with parent variant `SchemaQuery`.
- `type Pet implements Node { id: ID! }`, on `Node`: `EntityNameWrapper` with parent variant `ObjectImplements`.
- `union Search = Pet | User`, on `User`: `EntityNameWrapper` with parent variant `UnionMember`.
- empty document, position 0: `TypeSystemDocument`.

When 1–10 are done, the spec in ast.md and parsing-standards.md is implemented.
