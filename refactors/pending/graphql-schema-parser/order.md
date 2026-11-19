# GraphQL schema parser: order

Requires parsing-standards.md and ast.md. This file is the work list for `crates/graphql_schema_parser`. Each change is independently shippable, lands with the tests named here, and is one commit. When every change is done, `parse_schema` covers spec 3 (Type System) and 3.1 (Type System Extensions) with the lexical grammar of spec 2.1 and 2.10. integration.md is the work in `graphql_network_protocol` that reads the result; it lands after change 11.

## End-user experience

A schema file is any file the config's `schema` globs match (integration.md). The user writes definitions and `extend` blocks in any of those files, in any order. A file with a syntax error in one definition still contributes every other definition: a lookup that reaches the broken definition's name fails with that definition's error, and every other lookup is unaffected. Errors are not shown to the user yet: there are no diagnostics, semantic tokens, or other LSP features on `.graphql` files (parsing-standards.md, Later). Every keystroke in an open schema file reparses that whole file.

## Crate

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
nonempty = { workspace = true }
prelude = { path = "../prelude" }
span = { path = "../span" }
strum = { workspace = true }
thiserror = { workspace = true }

[lints]
workspace = true
```

Workspace member through `./crates/*`. `lib.rs` modules: `token_kind`, `tokenize`, `string_value`, `token_cursor`, `keyword`, `parse_error`, `ast`, `type_annotation`, `value`, `parse_schema`, `recovery`; `#[cfg(test)]` modules `test_cursor`, `span_of`, and `demo_schemas`. `lib.rs` re-exports `ast` with `pub use ast::*`. Public: `parse_schema`, `ParsedSchema`, `DefinitionError`, `DefinitionName`, every type in ast.md, `ParseError`, `ExpectedFound`, `Expectation`, `Found`, `GraphQLTokenKind` (it is the payload of `Expectation::Token` and `Found::Token`).

Each change lists the functions it adds. A function's span comes from `spanning` when it returns `WithSpan`. Every `require_*` maps `Err(())` with `expected`; the expectation is `Expectation::Token(kind)` unless the change names another.

## 1. Tokenizer

Status: landed

The crate, `GraphQLTokenKind`, `TokenKindExtras`, `StringToken`, `VariableWidthEscape`, `BlockStringToken`, `lex_string`, `lex_block_string`, `fail_with`, `fixed_width_code_unit`, `variable_width_escape`, and `tokenize`, as parsing-standards.md (Tokenizer).

Tests (`tokenize` kinds and spans):

- `type Pet { id: ID! }`: Identifier, Identifier, OpenBrace, Identifier, Colon, Identifier, Exclamation, CloseBrace.
- `a, b`: Identifier, Identifier (comma ignored, spec 2.1.4).
- `# comment\ntype`: one Identifier at the span of `type` (spec 2.1.3).
- `\u{feff}type`: one Identifier (BOM ignored, spec 2.1.6).
- `a\rb` and `a\r\nb`: two Identifiers each.
- `\u{000C}`: one Error (form feed is not whitespace, spec 2.1.1).
- empty text and `  \t\n`: no tokens.
- `& | = @ : ! ( ) [ ] { }`: their twelve kinds in order.
- `$`, `.`, `...`: Error tokens.
- `0`, `-0`, `42`, `9223372036854775807`: IntegerLiteral each. `1.5`, `-1.5`, `1e2`, `1.5E-2`: FloatLiteral each.
- `01`: ErrorNumberLiteralLeadingZero. `.5`: ErrorFloatLiteralMissingZero. `1.`, `1a`, `0x`: ErrorNumberLiteralTrailingInvalid.
- `"hi"`, `""`: StringLiteral. `"""hi"""`: BlockStringLiteral. `""""""`: one BlockStringLiteral spanning all six quotes.
- `"\u{1F4A9}"`, `"\uD83D\uDCA9"`, `"💩"`, `"a\u{0001}b"` (a raw control character): StringLiteral each.
- `"\x"`: ErrorUnsupportedStringCharacter spanning `"\x"`. `"unterminated`: ErrorUnterminatedString spanning the rest. `"hi\n`: ErrorUnterminatedString spanning `"hi`. `"""unterminated`: ErrorUnterminatedBlockString.
- `"""\u{0000}💩"""`: BlockStringLiteral.

## 2. String values

Status: landed

`decode_string_value`, `decode_code_units`, `TokenText` (copied, with `sub_span`, in `token_cursor.rs`), `intern_block_string_value`, `clean_block_string`, `get_common_indent`, `block_string_lines`, `line_is_whitespace`, `is_not_whitespace`, as parsing-standards.md (String values). `ParseError`, `ExpectedFound`, `Expectation`, `Found`, and `ParseError::expected` (parsing-standards.md, Errors) land here because `decode_string_value` returns `WithSpan<ParseError>`. `consume_string_value_if` lands in change 3 with the cursor.

Tests call `decode_string_value` on the interior of a tokenized `StringLiteral` (`TokenText` built from the token's span, `exclude_ends(1)`) and `clean_block_string` on literal interiors:

- `"hi"` is `hi`; `""` is the empty string.
- `"a\"b\\c\/d\be\ff\ng\rh\ti"` decodes each of the eight escapes.
- `"\u0041"` is `A`; `"\u{41}"` is `A`; `"\u{1F4A9}"` and `"\uD83D\uDCA9"` are both U+1F4A9.
- `"\uDEAD"` (lone trailing surrogate), `"\uD83D"` (lone leading surrogate), `"\uD83Dx"`, `"\u{D800}"`, `"\u{110000}"`, `"\u{FFFFFFFFF}"` are `InvalidUnicodeEscape`. For `"x\uDEADy"` the error span is the span of `\uDEAD` in the text.
- `"\u0041\uD83D\uDCA9"` is `A` then U+1F4A9 (one fixed-width run with a scalar and a pair).
- `"💩"` is U+1F4A9 (unescaped supplementary character).
- `clean_block_string("hi")` is `hi`; `"\n  hello\n  world\n"` is `hello\nworld`; `"   hi"` is `   hi` (first line keeps its indent); `"\r  a\r  b"` is `a\nb` (lone CR splits); `"\r\n  a\r\n  b\r\n"` is `a\nb`; `""` is empty; `"  \n  \n"` is empty.
- `intern_block_string_value::<StringLiteralValue>` of `a\"""b` (backslash then three quotes) looks up `a"""b`.

## 3. Cursor and errors

Status: landed, except the two `'{'` display assertions below. `strum::Display` writes a unit variant's `to_string` verbatim and rejects a lone `{` or `}` in it, so `'{{'` displays as `'{{'` (iso's `OpenBrace` and `CloseBrace` have the same defect). Open question: which representation displays one brace.

`TokenIndex`, `TokenCursor`, `consume_string_value_if`, `DefinitionKeyword`, `ExtensionKeyword`, `BooleanOrNullKeyword` (in `keyword.rs`), the `with_cursor` test helper, and `span_of` (copied), as parsing-standards.md (`TokenCursor`, Keyword dispatch, Errors, String values, Tests).

Tests (through `with_cursor`):

- `require_token(Identifier)` on `Pet` returns text `Pet` at `0..3`; `at_end` afterwards.
- `require_token(Colon)` on `Pet` is `Err(())`; the cursor is still on `Pet`; `expected(Expectation::Token(Colon))` displays `Expected colon (':'), found non-variable identifier (e.g. 'x' or 'Foo').` with span `0..3`.
- `expected` on an empty text is `Found::EndOfFile` at `0..0`; after consuming `Pet ` (trailing space), at `3..3`.
- `consume_identifier_of::<DefinitionKeyword>()` on `type` is `Type` at `0..4`; on `typo` is `None` and the cursor is still on `typo`.
- `consume_keyword_if("extend")` on `extended` is `None`.
- `spanning` around two `require_token(Identifier)` on `a b` is `0..3`; `spanning` around a closure that consumes nothing, on `a`, is `0..0`; after consuming `a` in `a b`, a no-consume `spanning` is `1..1` (the zero-width rule).
- `consumed_since(start)` after two advances on `a b c` is the first two tokens; `span_since(start)` is `0..3`.
- `consume_string_value_if::<StringLiteralValue>` on `"a\nb"` looks up `a`, U+000A, `b` and spans the quotes; on `"""  x"""` looks up `  x`; on `x` is `Ok(None)`; on `"\uDEAD"` is `Err(InvalidUnicodeEscape)`.
- `Expectation` displays: `DefinitionOrExtensionKeyword`, `ImplementsDirectiveOrOpenBrace` (`` `implements`, a directive, or '{' ``), `DirectiveOrOpenBrace` (`a directive or '{'`), `Keyword("on")` (`` the keyword `on` ``). `Found::EndOfFile` displays `end of file`. `ParseError::InvalidUnicodeEscape` displays its message.

## 4. Constant values and directives

Status: landed

```rust
// from crates/graphql_schema_parser/src/value.rs
pub(crate) fn parse_constant_value(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(value) = consume_string_value_if::<StringLiteralValue>(cursor)? {
            return ConstantValue::String(StringLiteralValueWrapper(value.item)).wrap_ok();
        }
        match cursor.peek().map(|token| token.item) {
            Some(GraphQLTokenKind::IntegerLiteral) => {
                ConstantValue::Int(parse_integer_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::FloatLiteral) => {
                ConstantValue::Float(parse_float_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::Identifier) => parse_name_value(cursor),
            Some(GraphQLTokenKind::OpenBracket) => {
                ConstantValue::List(parse_list_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::OpenBrace) => {
                ConstantValue::Object(parse_object_value(cursor)?).wrap_ok()
            }
            _ => cursor.expected(Expectation::ConstantValue).wrap_err(),
        }
    })
}

fn parse_integer_value(cursor: &mut TokenCursor<'_>) -> Result<IntegerValue, WithSpan<ParseError>>
fn parse_float_value(cursor: &mut TokenCursor<'_>) -> Result<FloatValue, WithSpan<ParseError>>
fn parse_name_value(cursor: &mut TokenCursor<'_>) -> Result<ConstantValue, WithSpan<ParseError>>
fn parse_list_value(cursor: &mut TokenCursor<'_>) -> Result<ListValue, WithSpan<ParseError>>
fn parse_object_value(cursor: &mut TokenCursor<'_>) -> Result<ObjectValue, WithSpan<ParseError>>
fn parse_object_field(cursor: &mut TokenCursor<'_>) -> Result<WithSpan<ObjectField>, WithSpan<ParseError>>

// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_directives(cursor: &mut TokenCursor<'_>) -> Result<Vec<WithSpan<Directive>>, WithSpan<ParseError>> {
    std::iter::from_fn(|| {
        cursor
            .peek()
            .is_some_and(|token| token.item == GraphQLTokenKind::At)
            .then(|| cursor.spanning(parse_directive))
    })
    .collect()
}

fn parse_directive(cursor: &mut TokenCursor<'_>) -> Result<Directive, WithSpan<ParseError>>
fn parse_argument(cursor: &mut TokenCursor<'_>) -> Result<WithSpan<Argument>, WithSpan<ParseError>>
fn parse_optional_nonempty_group<T>(...)    // parsing-standards.md, Groups
fn parse_nonempty_group_after_open<T>(...)
fn parse_group_after_open<T>(...)
```

Strings go first through `consume_string_value_if`, the two `consume_token_if` calls of iso `parse_string_literal`; the other forms dispatch on the peeked kind and then call a function that requires its first token (iso `parse_non_constant_value`, `crates/isograph_parser/src/arguments.rs`).

- `parse_integer_value`: `require_token(IntegerLiteral)`, `text().parse::<i64>()`, `IntegerDoesNotFitI64` at the token on `Err` (iso `parse_integer_value`, `crates/isograph_parser/src/arguments.rs`).
- `parse_float_value`: `require_token(FloatLiteral)`, `text().parse::<f64>()` then `FloatValue::new`, `InvalidFloat` at the token on `Err`.
- `parse_name_value`: `consume_identifier_of::<BooleanOrNullKeyword>()` maps `True` / `False` to `Boolean(BooleanValue(..))` and `Null` to `Null(NullValue)`; otherwise `require_token(Identifier)` interned as `EnumLiteralValueWrapper` is `Enum`.
- `parse_list_value`: `require_token(OpenBracket)`, then `parse_group_after_open(cursor, CloseBracket, parse_constant_value)` (empty allowed, spec 2.10.7).
- `parse_object_value`: `require_token(OpenBrace)`, then `parse_group_after_open(cursor, CloseBrace, parse_object_field)` (empty allowed, spec 2.10.8).
- `parse_object_field`: spanning; `require_token(Identifier)` interned as `ValueKeyNameWrapper`, `require_token(Colon)`, `parse_constant_value`.
- `parse_directive`: `require_token(At)`, `require_token(Identifier)` interned as `DirectiveNameWrapper`, `parse_optional_nonempty_group(cursor, OpenParenthesis, CloseParenthesis, parse_argument)`.
- `parse_argument`: spanning; `require_token(Identifier)` interned as `ArgumentNameWrapper`, `require_token(Colon)`, `parse_constant_value`.

Tests (through `with_cursor`):

- `1` is `Int(1)`; `-7` is `Int(-7)`; `9223372036854775808` is `IntegerDoesNotFitI64` at its span.
- `1.5` is `Float` with `as_float() == 1.5`; `1e400` is `Float` with `as_float()` infinite.
- `"hi"` is `String` looking up `hi`; `"""hi"""` is `String` looking up `hi` (block strings are spec `StringValue`); `"\n"`-escaped strings decode.
- `true`, `false`, `null` are `Boolean(True)`, `Boolean(False)`, `Null`; `AVAILABLE` is `Enum` looking up `AVAILABLE`; `nullable` is `Enum`.
- `[]` is an empty `List`; `[1, "a", [true]]` is a three-item list whose third item is a one-item list; `[1` is `Expected(Token(CloseBracket), EndOfFile)`.
- `{}` is an empty `Object`; `{ a: 1, b: { c: null } }` has fields `a` and `b`, `b` holding an object with `c`; `{ a 1 }` is `Expected(Token(Colon), Token(IntegerLiteral))`.
- `}` is `Expected(ConstantValue, Token(CloseBrace))`; empty text is `Expected(ConstantValue, EndOfFile)`; `$x` is `Expected(ConstantValue, Token(Error))`.
- Value spans: `[1, 2]` spans the brackets; its items span `1` and `2`; `"hi"` spans the quotes.
- `parse_directives` on `@a @b(x: 1) rest` is two directives (`a` with no arguments, `b` with argument `x` = `Int(1)`), and the cursor is on `rest`; the second directive's span runs from `@` to `)`.
- `parse_directives` on `rest` is empty and consumes nothing; on `@a()` is `Expected(Token(Identifier), Token(CloseParenthesis))`; on `@` is `Expected(Token(Identifier), EndOfFile)`; on `@a(x: 1` is `Expected(Token(CloseParenthesis), EndOfFile)`.

## 5. Type annotations

Status: landed

```rust
// from crates/graphql_schema_parser/src/type_annotation.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        let nullable = parse_nullable_type_annotation(cursor)?;
        match cursor.consume_token_if(GraphQLTokenKind::Exclamation) {
            Some(_) => TypeAnnotation::NonNull(NonNullTypeAnnotation(nullable)).wrap_ok(),
            None => nullable.item.into_type_annotation().wrap_ok(),
        }
    })
}

fn parse_nullable_type_annotation(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<NullableTypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(GraphQLTokenKind::Identifier) {
            return NullableTypeAnnotation::Named(NamedTypeAnnotation(
                name.interned().map(EntityNameWrapper),
            ))
            .wrap_ok();
        }
        if cursor.consume_token_if(GraphQLTokenKind::OpenBracket).is_some() {
            let inner = parse_type_annotation(cursor)?;
            cursor
                .require_token(GraphQLTokenKind::CloseBracket)
                .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::CloseBracket)))?;
            return NullableTypeAnnotation::List(ListTypeAnnotation(inner).boxed()).wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}
```

Origin: iso `parse_type_annotation` and `parse_named_or_list` (`crates/isograph_parser/src/variables.rs`). Delta: `!` builds `NonNull` instead of iso's union-with-null; the list interior is `[` type `]` tokens, not a bracket group and its chunk; no semantic tokens.

Tests (through `with_cursor`):

- `String` is `Named` looking up `String`, span `0..6`.
- `String!` is `NonNull` whose inner is `Named` at `0..6`; the outer span is `0..7`.
- `[String]` is `List` spanning `0..8` whose inner is `Named`.
- `[String!]!` is `NonNull` of `List` of `NonNull` of `Named`.
- `[[Int]]` nests two lists.
- `String!!` parses `String!` and leaves the cursor on the second `!`.
- `!`, `]`, empty text are `Expected(TypeAnnotation, ..)` with found `Token(Exclamation)`, `Token(CloseBracket)`, `EndOfFile`.
- `[String` is `Expected(Token(CloseBracket), EndOfFile)`; `[]` is `Expected(TypeAnnotation, Token(CloseBracket))`.

## 6. Definition errors, descriptions, scalars

Status: landed

`DefinitionError`, `DefinitionName`, `DefinitionError::unnamed`, `parse_after_entity_name` (parsing-standards.md, Recovery), and:

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn consume_description(
    cursor: &mut TokenCursor<'_>,
) -> Result<Option<WithSpan<Description>>, WithSpan<ParseError>> {
    consume_string_value_if::<DescriptionValue>(cursor)
        .map(|description| description.map(|description| description.map(Description)))
}

fn parse_entity_name(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<EntityNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map(|name| name.interned().map(EntityNameWrapper))
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))
}

fn parse_scalar_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ScalarTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        ScalarTypeDefinition {
            description,
            name,
            directives: parse_directives(cursor)?,
        }
        .wrap_ok()
    })
}
```

Origin: upstream isograph `parse_scalar_type_definition` and `parse_optional_description` (`../isograph/crates/graphql_schema_parser/src/parse_schema.rs`, `description.rs`). Delta: `DefinitionError` carries the name; a description is any `StringValue` and decodes.

Changes 6 to 10 add the definition and extension parse functions; change 11 adds `parse_schema`, which dispatches on the keyword. Their tests run `with_cursor` on the whole fixture, consume the description when the fixture has one, advance past the keyword (past `extend` only for change 10, which calls `parse_type_system_extension`), and call the function directly with that description (`None` when the fixture has none). Spans are offsets in the whole fixture. "`Err` named `X`" means `DefinitionError { name: Some(DefinitionName::Entity(X)), .. }` (or `Directive` for directive definitions).

Tests:

- `scalar Date` is named `Date` with no directives; the `spanning` span around the call is `7..11`.
- `scalar Date @specifiedBy(url: "https://example.com")` has one directive `specifiedBy` with argument `url`.
- `scalar` (nothing after the keyword) is `Err` with `name: None` and `Expected(Token(Identifier), EndOfFile)`.
- `scalar 1` is `Err` with `name: None` and found `Token(IntegerLiteral)`.
- `scalar Date @a(` is `Err` named `Date` (the name is kept when the failure comes after it).
- `consume_description` on `"a date"` looks up `a date` at `0..8`; on `"""\n  a\n  date\n"""` looks up `a\ndate`; on `scalar` is `Ok(None)` and consumes nothing; on `"\uDEAD"` is `Err(InvalidUnicodeEscape)`.

## 7. Objects, interfaces, fields, input values

Status: landed

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_object_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ObjectTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        ObjectTypeDefinition {
            description,
            name,
            interfaces: parse_implements_interfaces(cursor)?,
            directives: parse_directives(cursor)?,
            fields: parse_fields_definition(cursor)?,
        }
        .wrap_ok()
    })
}

fn parse_interface_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InterfaceTypeDefinition, DefinitionError>

fn parse_implements_interfaces(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>> {
    match cursor.consume_keyword_if("implements") {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_separated_nonempty(cursor, GraphQLTokenKind::Ampersand, parse_entity_name),
    }
}

fn parse_fields_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<FieldDefinition>>, WithSpan<ParseError>> {
    parse_optional_nonempty_group(
        cursor,
        GraphQLTokenKind::OpenBrace,
        GraphQLTokenKind::CloseBrace,
        |cursor| cursor.spanning(parse_field_definition),
    )
}

fn parse_field_definition(cursor: &mut TokenCursor<'_>) -> Result<FieldDefinition, WithSpan<ParseError>>

fn parse_arguments_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<InputValueDefinition>>, WithSpan<ParseError>>

fn parse_input_value_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<InputValueDefinition, WithSpan<ParseError>>

fn parse_separated_nonempty<T>(...)    // parsing-standards.md, Groups
```

- `parse_interface_type_definition`: the same body as objects building `InterfaceTypeDefinition` (spec 3.7 allows `implements` on interfaces).
- `parse_field_definition`: `consume_description`, `require_token(Identifier)` interned as `SelectableNameWrapper`, `parse_arguments_definition`, `require_token(Colon)`, `parse_type_annotation`, `parse_directives`.
- `parse_arguments_definition`: `parse_optional_nonempty_group(cursor, OpenParenthesis, CloseParenthesis, |cursor| cursor.spanning(parse_input_value_definition))`.
- `parse_input_value_definition`: `consume_description`, `require_token(Identifier)` interned as `InputValueNameWrapper`, `require_token(Colon)`, `parse_type_annotation`, `default_value`: `consume_token_if(Equals)` then `parse_constant_value` wrapped `Some`, else `None`; `parse_directives`.

In change 11 the `DefinitionKeyword::Type` and `Interface` arms call these.

Tests (`with_cursor`, the change 6 convention):

- `type Pet { id: ID! name: String }` is `Object` `Pet` with fields `id` (`NonNull(Named(ID))`) and `name` (`Named(String)`); commas absent (spec 2.1.4).
- `type Pet { id: ID!, name: String, }` has the same two fields.
- `type Pet` has no fields; `type Pet {}` is `Err` named `Pet` with `Expected(Token(Identifier), Token(CloseBrace))`.
- `type Pet {` is `Err` named `Pet` with `Expected(Token(Identifier), EndOfFile)` (the first field is required); `type Pet { id: ID!` is `Err` named `Pet` with `Expected(Token(CloseBrace), EndOfFile)`.
- `type Pet implements Node { id: ID! }`, `type Pet implements Node & Named { id: ID! }`, `type Pet implements & Node { id: ID! }` have interfaces `[Node]`, `[Node, Named]`, `[Node]`.
- `type Pet implements { id: ID! }` is `Err` named `Pet` with `Expected(Token(Identifier), Token(OpenBrace))`.
- `type Pet @key(fields: "id") { id: ID! }` has one directive on the type.
- `type Pet { "the id" id: ID! @deprecated(reason: "no") }`: field description `the id`, one field directive.
- `type Pet { name(lang: String = "en", upper: Boolean): String }`: two arguments; `lang` default `String("en")`, `upper` no default.
- `type Pet { name(): String }` is `Err` named `Pet` with `Expected(Token(Identifier), Token(CloseParenthesis))`.
- `type Pet { name(lang: String = ): String }` is `Err` with `Expected(ConstantValue, Token(CloseParenthesis))`.
- `type Pet { type: String input: Int }` has fields named `type` and `input` (keywords are names in field position).
- `interface Node { id: ID! }` is `Interface` `Node`; `interface Named implements Node { id: ID! }` has interfaces `[Node]`.
- `type type { id: ID! }` is `Object` named `type`.

## 8. Unions, enums, input objects

Status: landed

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_union_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<UnionTypeDefinition, DefinitionError>

fn parse_union_member_types(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>> {
    match cursor.consume_token_if(GraphQLTokenKind::Equals) {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_separated_nonempty(cursor, GraphQLTokenKind::Pipe, parse_entity_name),
    }
}

fn parse_enum_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<EnumTypeDefinition, DefinitionError>

fn parse_enum_values_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EnumValueDefinition>>, WithSpan<ParseError>>

fn parse_enum_value_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<EnumValueDefinition, WithSpan<ParseError>> {
    let description = consume_description(cursor)?;
    if let Some(word) = cursor.peek_identifier_of::<BooleanOrNullKeyword>() {
        return ParseError::EnumValueCannotBeTrueFalseOrNull
            .with_span(word.location)
            .wrap_err();
    }
    let value = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?;
    EnumValueDefinition {
        description,
        value: value.interned().map(EnumLiteralValueWrapper),
        directives: parse_directives(cursor)?,
    }
    .wrap_ok()
}

fn parse_input_object_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InputObjectTypeDefinition, DefinitionError>

fn parse_input_fields_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<InputValueDefinition>>, WithSpan<ParseError>>
```

- `parse_union_type_definition`: after the name, `parse_directives`, `parse_union_member_types`.
- `parse_enum_type_definition`: after the name, `parse_directives`, `parse_enum_values_definition` (`parse_optional_nonempty_group` with braces and `|cursor| cursor.spanning(parse_enum_value_definition)`).
- `parse_input_object_type_definition`: after the name, `parse_directives`, `parse_input_fields_definition` (`parse_optional_nonempty_group` with braces and `|cursor| cursor.spanning(parse_input_value_definition)`).

Tests (`with_cursor`, the change 6 convention):

- `union Search = Pet | User`, `union Search = | Pet | User` have members `[Pet, User]`; `union Search = Pet` has `[Pet]`; `union Search` has none; `union Search @a` has one directive and no members.
- `union Search =` and `union Search = |` are `Err` named `Search` with found `EndOfFile`; `union Search = Pet |` likewise.
- `enum Status { AVAILABLE SOLD }` has values `AVAILABLE`, `SOLD`; `enum Status { "on sale" AVAILABLE @deprecated }` has a described value with a directive; `enum Status` has no values; `enum Status {}` is `Err`.
- `enum Status { true }`, `{ false }`, `{ null }` are `Err` named `Status` with `EnumValueCannotBeTrueFalseOrNull` at the word; `enum Status { nullable }` is `Ok`.
- `input PetInput { name: String = "x" tags: [String!] @a }` has two fields: `name` with default `String("x")`, `tags` with one directive; `input PetInput` has none; `input PetInput {}` is `Err`.
- `input I { a: I = { b: [1, 2], c: ENUM } }` has a default `Object` with fields `b` (`List`) and `c` (`Enum`).

## 9. Schema and directive definitions

Status: landed

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_schema_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<SchemaDefinition, DefinitionError>

fn parse_root_operation_types_after_open(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<RootOperationTypeDefinition>>, WithSpan<ParseError>>

fn parse_root_operation_type_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<RootOperationTypeDefinition, WithSpan<ParseError>>

fn parse_directive_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<DirectiveDefinition, DefinitionError>

fn parse_directive_definition_name(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<DirectiveNameWrapper>, WithSpan<ParseError>>

fn parse_directive_location(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<DirectiveLocationWrapper>, WithSpan<ParseError>>
```

- `parse_schema_definition`: `parse_directives`, `require_token(OpenBrace)`, `parse_root_operation_types_after_open`; every error mapped with `DefinitionError::unnamed`.
- `parse_root_operation_types_after_open`: `parse_nonempty_group_after_open(cursor, CloseBrace, |cursor| cursor.spanning(parse_root_operation_type_definition))`.
- `parse_root_operation_type_definition`: `consume_identifier_of::<OperationType>()` mapped to `OperationTypeWrapper`, `Expectation::OperationType` when absent; `require_token(Colon)`; `parse_entity_name`.
- `parse_after_directive_name` lands here (parsing-standards.md, Recovery).
- `parse_directive_definition`: `parse_after_directive_name`, then `parse_arguments_definition`, `repeatable`: `consume_keyword_if("repeatable")` is `Repeatable` when `Some`, `NotRepeatable` when `None`; `require_keyword("on")` mapped to `Expectation::Keyword("on")`; `locations`: `parse_separated_nonempty(cursor, Pipe, parse_directive_location)`.
- `parse_directive_definition_name`: `require_token(At)`, then `require_token(Identifier)` interned as `DirectiveNameWrapper`; the result's span is the identifier's (the `@` is not part of the name), so there is no `spanning`.
- `parse_directive_location`: `consume_identifier_of::<DirectiveLocation>()` mapped to `DirectiveLocationWrapper`, `Expectation::DirectiveLocation` when absent.

Spec 3.13 requires the `@` in a directive definition. Upstream isograph accepts a definition without it; this parser does not.

Tests (`with_cursor`, the change 6 convention):

- `schema { query: Query mutation: Mutation subscription: Subscription }`: three root operation types in order with their operation types and names.
- `"the schema" schema @a { query: Query }`: description and one directive.
- `schema { query: Query query: Other }`: two entries, both `Query` operation type (no duplicate error in the parser).
- `schema {}` is `Err` with `name: None` and `Expected(OperationType, Token(CloseBrace))`; `schema` is `Err` with `Expected(Token(OpenBrace), EndOfFile)`; `schema { foo: Query }` is `Expected(OperationType, Token(Identifier))` at `foo`.
- `directive @oneOf on INPUT_OBJECT`: name `oneOf`, no arguments, `NotRepeatable`, locations `[InputObject]`.
- `directive @a(x: Int = 1) repeatable on FIELD | OBJECT`, `directive @a on | FIELD | OBJECT`: arguments, `Repeatable`, locations `[Field, Object]` (the second with a leading `|`).
- Every one of the 19 locations parses: `directive @a on QUERY | MUTATION | SUBSCRIPTION | FIELD | FRAGMENT_DEFINITION | FRAGMENT_SPREAD | INLINE_FRAGMENT | VARIABLE_DEFINITION | SCHEMA | SCALAR | OBJECT | FIELD_DEFINITION | ARGUMENT_DEFINITION | INTERFACE | UNION | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION`.
- `directive @a on FOO` is `Err` with `name: Some(Directive(a))` and `Expected(DirectiveLocation, Token(Identifier))` at `FOO`; `directive @a on field` likewise (locations are case-sensitive).
- `directive @a FIELD` is `Expected(Keyword("on"), Token(Identifier))`; `directive @a on` is found `EndOfFile`; `directive @a() on FIELD` is `Expected(Token(Identifier), Token(CloseParenthesis))`.
- `directive a on FIELD` is `Err` with `name: None` and `Expected(Token(At), Token(Identifier))`.
- `directive @type on FIELD` is `Ok` named `type`.

## 10. Extensions

Status: landed

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_type_system_extension(
    cursor: &mut TokenCursor<'_>,
) -> Result<TypeSystemExtension, DefinitionError> {
    let keyword = cursor
        .consume_identifier_of::<ExtensionKeyword>()
        .ok_or_else(|| DefinitionError::unnamed(cursor.expected(Expectation::ExtensionKeyword)))?;
    match keyword.item {
        ExtensionKeyword::Schema => TypeSystemExtension::Schema(parse_schema_extension(cursor)?),
        ExtensionKeyword::Scalar => {
            TypeSystemExtension::Type(TypeExtension::Scalar(parse_scalar_type_extension(cursor)?))
        },
        ExtensionKeyword::Type => {
            TypeSystemExtension::Type(TypeExtension::Object(parse_object_type_extension(cursor)?))
        },
        ExtensionKeyword::Interface => {
            TypeSystemExtension::Type(TypeExtension::Interface(
                parse_interface_type_extension(cursor)?,
            ))
        }
        ExtensionKeyword::Union => {
            TypeSystemExtension::Type(TypeExtension::Union(parse_union_type_extension(cursor)?))
        },
        ExtensionKeyword::Enum => {
            TypeSystemExtension::Type(TypeExtension::Enum(parse_enum_type_extension(cursor)?))
        },
        ExtensionKeyword::Input => {
            TypeSystemExtension::Type(TypeExtension::InputObject(
                parse_input_object_type_extension(cursor)?,
            ))
        }
    }
    .wrap_ok()
}

fn parse_object_type_extension(
    cursor: &mut TokenCursor<'_>,
) -> Result<ObjectTypeExtension, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        let interfaces = parse_implements_interfaces(cursor)?;
        let directives = parse_directives(cursor)?;
        let fields = parse_fields_definition(cursor)?;
        if interfaces.is_empty() && directives.is_empty() && fields.is_empty() {
            return cursor
                .expected(Expectation::ImplementsDirectiveOrOpenBrace)
                .wrap_err();
        }
        ObjectTypeExtension {
            name,
            interfaces,
            directives,
            fields,
        }
        .wrap_ok()
    })
}

fn parse_schema_extension(cursor: &mut TokenCursor<'_>) -> Result<SchemaExtension, DefinitionError>
fn parse_scalar_type_extension(cursor: &mut TokenCursor<'_>) -> Result<ScalarTypeExtension, DefinitionError>
fn parse_interface_type_extension(cursor: &mut TokenCursor<'_>) -> Result<InterfaceTypeExtension, DefinitionError>
fn parse_union_type_extension(cursor: &mut TokenCursor<'_>) -> Result<UnionTypeExtension, DefinitionError>
fn parse_enum_type_extension(cursor: &mut TokenCursor<'_>) -> Result<EnumTypeExtension, DefinitionError>
fn parse_input_object_type_extension(cursor: &mut TokenCursor<'_>) -> Result<InputObjectTypeExtension, DefinitionError>
```

Each extension has no description (spec 3.1) and checks its spec alternatives after its parts:

- `parse_schema_extension` (spec 3.3.2): `parse_directives`, then `root_operation_types`: `consume_token_if(OpenBrace)` then `parse_root_operation_types_after_open`, else empty; both empty is `Expectation::DirectiveOrOpenBrace`; errors mapped with `DefinitionError::unnamed`.
- `parse_scalar_type_extension` (spec 3.5.6): after the name, `NonEmpty::from_vec(parse_directives(cursor)?)`, `None` is `Expectation::Directive`.
- `parse_interface_type_extension` (spec 3.7.1): the object extension body building `InterfaceTypeExtension`.
- `parse_union_type_extension` (spec 3.8.1): after the name, `parse_directives`, `parse_union_member_types`; both empty is `Expectation::DirectiveOrEquals`.
- `parse_enum_type_extension` (spec 3.9.1): after the name, `parse_directives`, `parse_enum_values_definition`; both empty is `Expectation::DirectiveOrOpenBrace`.
- `parse_input_object_type_extension` (spec 3.10.2): after the name, `parse_directives`, `parse_input_fields_definition`; both empty is `Expectation::DirectiveOrOpenBrace`.

Origin: upstream isograph `parse_type_system_extension` and `parse_object_type_extension`. Delta: all seven kinds; `require` of the extension keyword through keyword dispatch instead of upstream's `expect` / `assert` on `extend`; the emptiness checks.

Tests (`with_cursor`, the change 6 convention):

- `extend type Pet { nickname: String }` is `Extension(Type(Object))` `Pet` with one field; `extend type Pet implements Node`, `extend type Pet @a` are `Ok`.
- `extend type Pet` is `Err` named `Pet` with `Expected(ImplementsDirectiveOrOpenBrace, EndOfFile)`.
- `extend interface Node { id: ID! }` is `Ok`; `extend interface Node` is `Err` with `ImplementsDirectiveOrOpenBrace`.
- `extend scalar Date @a` is `Ok` with one directive; `extend scalar Date` is `Err` named `Date` with `Expected(Directive, EndOfFile)`.
- `extend union Search = Photo`, `extend union Search @a` are `Ok`; `extend union Search` is `Err` with `DirectiveOrEquals`.
- `extend enum Status { LOST }`, `extend enum Status @a` are `Ok`; `extend enum Status` is `Err` with `DirectiveOrOpenBrace`.
- `extend input PetInput { age: Int }`, `extend input PetInput @a` are `Ok`; `extend input PetInput` is `Err` with `DirectiveOrOpenBrace`.
- `extend schema { mutation: Mutation }`, `extend schema @a` are `Ok`; `extend schema` is `Err` with `name: None` and `DirectiveOrOpenBrace`.
- `extend directive @a on FIELD` is `Err` with `name: None` and `Expected(ExtensionKeyword, Token(Identifier))` at `directive`; `extend` alone is found `EndOfFile`.

## 11. `parse_schema` and recovery

Status: landed

`parse_schema`, `ParsedSchema`, `parse_definition_or_extension_or_skip`, `BracketDepth`, `skip_failed_definition`, `starts_definition`, `is_top_level_keyword` (parsing-standards.md, Pipeline and Recovery), and:

```rust
// from crates/graphql_schema_parser/src/parse_schema.rs
fn parse_type_system_definition_or_extension(
    cursor: &mut TokenCursor<'_>,
) -> Result<TypeSystemDefinitionOrExtension, DefinitionError> {
    if cursor.consume_keyword_if("extend").is_some() {
        return TypeSystemDefinitionOrExtension::Extension(parse_type_system_extension(cursor)?)
            .wrap_ok();
    }
    TypeSystemDefinitionOrExtension::Definition(parse_type_system_definition(cursor)?).wrap_ok()
}

fn parse_type_system_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<TypeSystemDefinition, DefinitionError> {
    let description = consume_description(cursor).map_err(DefinitionError::unnamed)?;
    let expectation = match description {
        Some(_) => Expectation::DefinitionKeyword,
        None => Expectation::DefinitionOrExtensionKeyword,
    };
    let keyword = cursor
        .consume_identifier_of::<DefinitionKeyword>()
        .ok_or_else(|| DefinitionError::unnamed(cursor.expected(expectation)))?;
    match keyword.item {
        DefinitionKeyword::Schema => {
            TypeSystemDefinition::Schema(parse_schema_definition(cursor, description)?)
        }
        DefinitionKeyword::Scalar => {
            TypeSystemDefinition::Type(TypeDefinition::Scalar(
                parse_scalar_type_definition(cursor, description)?,
            ))
        }
        DefinitionKeyword::Type => {
            TypeSystemDefinition::Type(TypeDefinition::Object(
                parse_object_type_definition(cursor, description)?,
            ))
        }
        DefinitionKeyword::Interface => {
            TypeSystemDefinition::Type(TypeDefinition::Interface(
                parse_interface_type_definition(cursor, description)?,
            ))
        }
        DefinitionKeyword::Union => {
            TypeSystemDefinition::Type(TypeDefinition::Union(
                parse_union_type_definition(cursor, description)?,
            ))
        }
        DefinitionKeyword::Enum => {
            TypeSystemDefinition::Type(TypeDefinition::Enum(
                parse_enum_type_definition(cursor, description)?,
            ))
        }
        DefinitionKeyword::Input => TypeSystemDefinition::Type(TypeDefinition::InputObject(
                parse_input_object_type_definition(cursor, description)?,
            )),
        DefinitionKeyword::Directive => {
            TypeSystemDefinition::Directive(parse_directive_definition(cursor, description)?)
        }
    }
    .wrap_ok()
}
```

Origin: upstream isograph `parse_schema`, `parse_schema_extensions`, `parse_type_system_document`, `parse_type_system_extension_document`, `peek_type_system_doc_type`, and `parse_type_system_definition`. Delta: one entry point for definitions and extensions; per-definition results with recovery; keyword dispatch (the unknown word is the error's span and is not consumed); a description before `extend` is an error at `extend`.

Tests (`parse_schema`):

- Empty text, `   `, and `# only a comment\n` have no entries.
- `scalar Date` is one `Ok(Definition(Type(Scalar)))` with span `0..11`.
- `"a date" scalar Date` is one `Ok` whose span starts at 0 and whose description looks up `a date`.
- `scalar Date scalar Time` is two `Ok`s (no separator needed).
- `type Pet { id: ID! } extend type Pet { name: String } scalar Date` is `Ok(Definition)`, `Ok(Extension)`, `Ok(Definition)` in order.
- One fixture per definition kind and one per extension kind (the first passing fixture of changes 6 to 10 for that kind) is one `Ok` entry of that variant.
- `foo` is one `Err` with `name: None`, error `Expected(DefinitionOrExtensionKeyword, Token(Identifier))` at `0..3`, entry span `0..3`.
- `"desc" foo` is one `Err` with `Expected(DefinitionKeyword, Token(Identifier))` at the span of `foo`.
- `"desc" extend type Pet { a: A }` is `Err` with `Expected(DefinitionKeyword, Token(Identifier))` at `extend` and entry span covering `"desc"`, then `Ok(Extension(Type(Object)))`.

Recovery tests (`parse_schema`):

- `type A { a: ! } type B { b: B }`: `Err` named `A`, then `Ok(B)` (two definitions, error in the first).
- `type A { a: B } type B { b: ! }`: `Ok(A)`, then `Err` named `B` (error in the last).
- `type A { a(x: { y: ! }): B type: C } type D { d: D }`: `Err` named `A` (the nested braces and the field named `type` inside are skipped), then `Ok(D)`.
- `type A { a: B` then end of file: one `Err` named `A` with found `EndOfFile`; the entry span runs to the end of the file (unterminated `{`).
- `scalar A @x( scalar B`: one `Err` named `A`; the open `(` keeps `scalar B` inside the skipped region.
- `} scalar A`: `Err` with `name: None` for the stray `}` (depth never below zero), then `Ok(A)`.
- `) ) ] scalar A`: one `Err` for the run of closers, then `Ok(A)`.
- `"described" type A { a: ! } type B { b: B }`: `Err` named `A` whose entry span starts at the description, then `Ok(B)`.
- `foo "desc" scalar A`: `Err` covering `foo`, then `Ok(A)` with description `desc` (a description followed by a keyword starts a definition).
- `directive @a on scalar Date`: `Err` named `a` (`scalar` is not a directive location), then `Ok(Scalar)` named `Date` (the failing token is a top-level keyword, so nothing is skipped).
- `type A { a: ! } type B { b: ! } type C { c: C }`: `Err` named `A`, `Err` named `B`, then `Ok(C)` (consecutive broken definitions).
- `foo bar scalar A`: one `Err` covering `foo bar`, then `Ok(A)`.
- `1 2 3`: one `Err` (a file of only errors).
- `extend type A { a: ! } extend type B { b: B }`: `Err` named `A`, then `Ok(Extension)`.

Fixture tests (`demo_schemas`, `parse_schema`): `tests/fixtures/github-demo-subset.graphql` (every directive and scalar of upstream isograph `demos/github-demo/schema.graphql`, then its first 12 types, interfaces, unions, enums, and input objects; the full file is over the 500 KB `check-added-large-files` limit), `tests/fixtures/pet-demo-schema.graphql`, and `tests/fixtures/pet-demo-schema-extension.graphql` (copies of `demos/pet-demo/backend/`). Each file has no `Err` entry, and its entry count equals the number of unindented lines whose first word is a definition keyword or `extend`. Prettier skips `**/tests/fixtures/**`.
