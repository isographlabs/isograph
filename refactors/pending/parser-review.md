# Parser review

The parser is a four-stage pipeline (tokenize, match brackets, chunk, parse grammar) with real recovery and a large test suite. The grammar layer is in decent shape. The lexer and the type AST are not. `cargo test -p isograph_parser --lib` is green.

## Bugs

### Lexer: `1.5` is an integer, then rejected as overflow (fixed)

The float regex is live. `1.5`, `12.34`, `0.0`, `-1.5`, `1e2`, and `1.5e2` are `FloatLiteral`. `parse_non_constant_value` does not match that kind, so `a: 1.5` is `Expected a value, found floating point value`. `IntegerDoesNotFitI64` is overflow of an integer token only. `1.` is still `ErrorNumberLiteralTrailingInvalid`. `.5` is still `ErrorFloatLiteralMissingZero`.

### Lexer: string failures do not produce the error kinds that exist, and they do not consume the body (fixed)

`lex_string` / `lex_block_string` bump through the body and set `lexer.extras.error_token`. `tokenize` takes that extras onto the `Error` logos emits.

- `"unterminated"` → one `ErrorUnterminatedString`
- `"\"\\x\""` → one `ErrorUnsupportedStringCharacter`
- unterminated `"""` → one `ErrorUnterminatedBlockString`

`number_and_string_errors_are_their_kinds` passes. `an_unterminated_string_is_not_a_description` consumes the whole token.

### Lexer: a control character inside a block string panics (fixed)

`BlockStringToken::Error` is consumed like `Other`. A terminated block string containing U+0000 is `BlockStringLiteral`. An unterminated one is `ErrorUnterminatedBlockString`.

### Block strings are values in tests and in descriptions, not in `parse_non_constant_value` (fixed)

`parse_non_constant_value` matches `StringLiteral` or `BlockStringLiteral`. `a: """hi"""` is a string value. `a_block_string_is_a_value` passes.

### `!` is consumed and dropped. Nullability is not in the tree

Mental model `Wrapper` is `Entity` / `List` / `Null`. GraphQL `T` is `T | null`. GraphQL `T!` is `T`. The bang is not a node; it is the absence of `Null`.

```
Foo!      ->  Foo
Foo       ->  Foo | null
[Foo!]!   ->  [Foo]
[Foo]     ->  [Foo | null] | null
[Foo!]    ->  [Foo] | null
[Foo]!    ->  [Foo | null]
```

After:

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    Null(Box<NullTypeAnnotation>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullTypeAnnotation(
    #[resolve_field]
    #[parent_variant(Null)]
    pub WithSpan<TypeAnnotation>,
);

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Null(Box<NullTypeAnnotationPath<'a>>),
}

pub type NullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NullTypeAnnotation, TypeAnnotationParent<'a>>;
```

`NamedTypeAnnotation` and `ListTypeAnnotation` are unchanged. `!` is still recorded as `SemanticToken::GraphQLTypeName`.

```rust
// from crates/isograph_parser/src/variables.rs
pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    let core = parse_named_or_list(cursor)?;
    match cursor.consume_token_if(
        NonBracketTokenKind::Exclamation,
        SemanticToken::GraphQLTypeName,
    ) {
        Some(bang) => core
            .item
            .with_span(Span::new(core.location.start, bang.location.end))
            .wrap_ok(),
        None => {
            let location = core.location;
            TypeAnnotation::Null(NullTypeAnnotation(core).boxed())
                .with_span(location)
                .wrap_ok()
        }
    }
}

fn parse_named_or_list(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
            return TypeAnnotation::Named(NamedTypeAnnotation {
                name: name.interned().map(EntityNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(parsed) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let parsed = parsed.item?;
            return TypeAnnotation::List(
                ListTypeAnnotation {
                    inner: parsed.item,
                    extra_tokens: parsed.extra_tokens,
                }
                .boxed(),
            )
            .wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}
```

Before: `parse_type_annotation` is one `spanning` that consumes a trailing bang and drops it. After: `parse_named_or_list` is that body without the bang consume. `parse_type_annotation` wraps `Null` when there is no bang, and extends the core span over the bang when there is one.

`parse_bracket_interior_type` still calls `parse_type_annotation`, so `[Pet]`'s element is already `Null(Named(Pet))`.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    NullTypeAnnotation(NullTypeAnnotationPath<'a>),
```

```rust
// from crates/isograph_parser/src/variables.rs
impl<'a> From<NullTypeAnnotationPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: NullTypeAnnotationPath<'a>) -> Self {
        IsographResolutionNode::NullTypeAnnotation(path)
    }
}
```

Who calls: `consume_to_target` and `parse_variable_declaration` already call `parse_type_annotation`. No other call sites.

Tests in `parse_iso_literal.rs`. Existing span tests stay (`a_to_target_accepts_every_type_annotation_form`, `a_full_field` `Person!`, `list_types_nest_with_non_null_markers` `[Pet!]!` is still `List` of `Named` whose inner span is `Pet!`). Shape tests:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_named_target_without_bang_is_null_wrapped() {
        let text = "field Query.Foo to Pet { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet");
        assert_eq!(target.location, span_of(text, "Pet"));
        match target.item.reference() {
            TypeAnnotation::Null(null) => {
                assert_eq!(null.0.location, span_of(text, "Pet"));
                match null.0.item.reference() {
                    TypeAnnotation::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Pet"));
                    }
                    annotation => panic!("expected Named inside Null, got {annotation:?}"),
                }
            }
            annotation => panic!("expected Null, got {annotation:?}"),
        }
    }

    #[test]
    fn a_named_target_with_bang_is_not_null_wrapped() {
        let text = "field Query.Foo to Pet! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet!");
        assert_eq!(target.location, span_of(text, "Pet!"));
        match target.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected Named, got {annotation:?}"),
        }
    }

    #[test]
    fn a_list_target_maps_graphql_nullability() {
        let text = "field Query.Foo to [Pet] { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet]");
        match target.item.reference() {
            TypeAnnotation::Null(outer) => match outer.0.item.reference() {
                TypeAnnotation::List(list) => {
                    let inner = list.inner.as_ref().expect("the list holds a type");
                    match inner.item.reference() {
                        TypeAnnotation::Null(elem) => {
                            assert!(matches!(elem.0.item, TypeAnnotation::Named(_)));
                        }
                        annotation => panic!("expected Null element, got {annotation:?}"),
                    }
                }
                annotation => panic!("expected List, got {annotation:?}"),
            },
            annotation => panic!("expected Null list, got {annotation:?}"),
        }
    }

    #[test]
    fn a_non_null_list_of_non_null_named_is_list_of_named() {
        let text = "field Query.Foo to [Pet!]! { id }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet!]!");
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                assert_eq!(inner.location, span_of(text, "Pet!"));
            }
            annotation => panic!("expected List, got {annotation:?}"),
        }
    }

    #[test]
    fn a_second_bang_is_leftover() {
        let text = "field Query.Foo to Pet!! { id }";
        let (parse, errors) = parsed(text);
        as_selectable(parse.reference());
        let second_bang = Span::new(span_of(text, "Pet!!").start + 4, span_of(text, "Pet!!").start + 5);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(NonBracketTokenKind::Exclamation))
                .with_span(second_bang)
                .wrap_vec(),
        );
    }
```

`a_field_with_to_parses_the_target_type` (`to Owner`) matches `Null` then `Named`, not `Named` at the top. `a_line_break_inside_a_list_type_does_not_attach_bang` (`[Pet\n!]`) inner is `Null(Named(Pet))`, not `Named`. `type_names_resolve_through_their_annotation_ancestry` (`$pets: [Pet]`): `Pet` is `EntityNameWrapper` -> `NamedTypeAnnotation` -> `TypeAnnotationParent::Null` -> `List` -> `Null` -> `Variable`. `a_bang_resolves_to_the_annotation` (`ID!`) is still `NamedTypeAnnotation` covering `!`.

Degenerate: `$x: ID` is `Null(Named)`. `$x: ID!` is `Named`. `to [[Pet]]` is `Null(List(Null(List(Null(Named)))))`.

### `parse_iso_literal` has no single entry point and three error channels

The crate entry is one function over `&str`. It runs tokenize, match brackets, chunk, grammar. The three error lists live on the return value, so a caller cannot drop a channel without ignoring a named field. Grammar-stage tests call this function. Stage unit tests (tokenize, brackets, chunk, subparsers) keep `pub(crate)` internals.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub bracket_errors: Vec<BracketError>,
    pub comma_errors: Vec<CommaWithoutItem>,
    pub tokens: Vec<WithSpan<SemanticToken>>,
}

pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut errors, &mut tokens);
    ParsedIsoLiteral {
        item,
        errors,
        bracket_errors,
        comma_errors,
        tokens,
    }
}

pub(crate) fn parse_chunked_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

Before: `pub fn parse_iso_literal(text, root, errors, tokens) -> Option<WithSpan<IsoLiteralParse>>`. After: that body is `parse_chunked_iso_literal`. `item: None` is still only the empty-literal case.

`BracketError` and `CommaWithoutItem` gain `Display` and `std::error::Error`, same impls as lsp-parse-diagnostics.md Change 1 (`Unclosed '{'` / `Unexpected '('` / `A comma with no item before it.`). Tests of those Displays live in the modules that own the types.

```rust
// from crates/isograph_parser/src/lib.rs
mod arguments;
mod chunk;
mod chunk_stream;
mod directives;
mod isograph_resolution_node;
mod matched_brackets;
mod non_bracket_token;
mod parse_error;
mod parse_iso_literal;
mod selections;
mod semantic_token;
mod token_kind;
mod tokenize;
mod variables;

pub use arguments::{
    Argument, ArgumentList, ArgumentListParent, ArgumentNameWrapper, Boolean, BooleanValue,
    IntegerValue, ListLiteral, ListLiteralValue, NonConstantValue, NullValue, ObjectEntry,
    ObjectLiteral, StringLiteralValueWrapper, VariableDeclarationOrUsage, VariableUse,
    ValueKeyNameWrapper,
};
pub use chunk::{
    Chunk, ChunkContentItem, ChunkSeparator, ChunkedGroup, ChunkedLevel, CommaWithoutItem,
    ExtraChunks, Singleton, Slot, UnparsedChunkItems,
};
pub use directives::{
    IsographDirectiveNameWrapper, IsographFieldDirective, IsographFieldDirectiveList,
};
pub use isograph_resolution_node::IsographResolutionNode;
pub use matched_brackets::{BracketError, CloseBracket, OpenBracket};
pub use non_bracket_token::{BracketKind, NonBracketToken, NonBracketTokenKind};
pub use parse_error::{Expectation, Found, ParseError};
pub use parse_iso_literal::{
    Description, EntityNameWrapper, EntrypointDeclaration, IsoLiteralItem, IsoLiteralParse,
    ParsedIsoLiteral, SelectableDeclaration, SelectableNameWrapper, parse_iso_literal,
};
pub use selections::{Selection, SelectionNameWrapper, SelectionSet};
pub use semantic_token::SemanticToken;
pub use variables::{
    ListTypeAnnotation, NamedTypeAnnotation, NullTypeAnnotation, TypeAnnotation,
    VariableDeclaration, VariableDeclarationList,
};
```

Path aliases and `*Parent` enums that `IsographResolutionNode` names stay `pub` via the modules that define them: add those `pub use` lines for every path type the resolve node lists. Do not `pub use` `tokenize`, `match_brackets`, `chunk`, `ItemCursor`, `ChunkStream`, `IsographLangTokenKind`, `TokenKindExtras`, `parse_type_annotation`, `consume_description`, or other parse helpers. `mod tokenize` and friends stay private; `tokenize` / `match_brackets` / `chunk` become `pub(crate)`.

Grammar tests: `parsed` / `parsed_with_errors` / `parsed_with_tokens` call `parse_iso_literal(text)` and read the struct fields. They do not call `tokenize` / `match_brackets` / `chunk`. `parsed` still asserts `bracket_errors` empty and `comma_errors` empty. `chunked` / `stream_of` used by `consume_description` unit tests stay on `pub(crate)` internals.

`arguments.rs` and `selections.rs` subparser tests stay on internals; they are not whole-literal parses.

Who else calls the pipeline: extract-iso-literals.md and lsp-semantic-tokens.md `file_literals` call `parse_iso_literal(text)`.

### Test suite is red on purpose in one case (fixed)

`tokenize::tests::observe_kinds` is not in the tree.

## Invariants not encoded in types

`Slot` and `parse_singleton` are in parser-minor-improvements.md.

### `VariableDeclarationOrUsage` is only a declaration

The `$name` node is the same whether it is a declaration or a use. Distinction is the resolve parent, not the payload.

Before: `VariableDeclarationOrUsage` is the declaration (`name`, `type_`, `default_value`). A use is `VariableUse(VariableNameWrapper)`. `VariableNameWrapperParent` is already `Use | Declaration`, but the `$name` tree is two different shapes, and `$` on a declaration is not a `VariableDeclarationOrUsage`.

After, most important first:

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsageParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsage(
    #[resolve_field]
    pub WithSpan<VariableNameWrapper>,
);

#[derive(Debug)]
pub enum VariableDeclarationOrUsageParent<'a> {
    Declaration(VariableDeclarationPath<'a>),
    Usage(VariableUsePath<'a>),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsagePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(pub common_lang_types::VariableName);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(
    #[resolve_field]
    #[parent_variant(Usage)]
    pub WithSpan<VariableDeclarationOrUsage>,
);

pub type VariableDeclarationOrUsagePath<'a> = PositionResolutionPath<
    &'a VariableDeclarationOrUsage,
    VariableDeclarationOrUsageParent<'a>,
>;

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableDeclarationOrUsagePath<'a>>;
```

```rust
// from crates/isograph_parser/src/variables.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclaration {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableDeclarationOrUsage>,
    #[resolve_field]
    #[parent_variant(Variable)]
    pub type_: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(VariableDefault)]
    pub default_value: Option<WithSpan<NonConstantValue>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclaration, UnparsedChunkItems>>>,
);
```

`VariableUse` stays a newtype so `#[parent_variant(Usage)]` has a pin. It does not add fields. `VariableNameWrapperParent` is deleted. `NonConstantValue::Variable(VariableUse)` is unchanged at the enum. `NonConstantValueParent::VariableDefault` and `TypeAnnotationParent::Variable` take `VariableDeclarationPath`.

```rust
// from crates/isograph_parser/src/arguments.rs
pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    missing_dollar: Expectation,
) -> Result<WithSpan<VariableDeclarationOrUsage>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        cursor
            .require_token(NonBracketTokenKind::Dollar, SemanticToken::Variable)
            .map_err(|()| cursor.expected(missing_dollar))?;
        let name = cursor
            .require_token(NonBracketTokenKind::Identifier, SemanticToken::Variable)
            .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
        VariableDeclarationOrUsage(name.interned().map(VariableNameWrapper)).wrap_ok()
    })
}
```

Before: returns `WithSpan<VariableNameWrapper>` whose span is the identifier only; `$` is consumed and not on that node. After: span is `$` plus the identifier. Call sites still wrap a use in `VariableUse(...)` and still pass the result to `parse_name_colon` as the declaration lhs.

`Expectation::VariableDeclarationOrUsage` becomes `Expectation::VariableDeclaration`. Display stays `a variable declaration, like '$id: ID!'`.

`IsographResolutionNode`: keep `VariableDeclarationOrUsage`. Add `VariableDeclaration`, `VariableDeclarationList`, `VariableDeclarationSlot`. Drop `VariableNameWrapper` parent enum variants. `UnparsedChunkItemsParent` leftover pin is `VariableDeclarationSlot`.

Who calls: `parse_variable_declaration`, `parse_non_constant_value`'s `$` arm, `parse_variable_name` callers. `chunk.rs` pin list.

Tests:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    #[test]
    fn a_dollar_in_a_use_resolves_to_declaration_or_usage_with_usage_parent() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Usage(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "x")) {
            IsographResolutionNode::VariableNameWrapper(name) => {
                assert_eq!(name.inner.0, "x".intern().to());
            }
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_dollar_in_a_declaration_resolves_to_declaration_or_usage_with_declaration_parent() {
        let text = "field Query.Foo($id: ID) { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Declaration(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::VariableNameWrapper(_) => {}
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }
```

`argument_names_resolve_through_the_selection` and `a_default_variable_resolves_through_variable_default` currently expect `VariableUse` on `$`. They expect `VariableDeclarationOrUsage` with `Usage` parent. `type_names_resolve_through_their_annotation_ancestry` currently matches `VariableNameWrapperParent::Declaration`; the name's parent is `VariableDeclarationOrUsagePath`, whose parent is `Declaration`.

### `TypeAnnotation::List(Box<ListTypeAnnotation>)` vs named-struct enums

The crate standard is `enum Foo { NamedStruct(Struct) }` or unit variants, not mixed payload shapes. `Expectation::Keyword(&'static str)`, `Expectation::OneOf(&'static [Expectation])`, `Found::Token(NonBracketTokenKind)`, `Found::Group(BracketKind)` are tuple variants. `OneOf(&[])` displays as `"one of"`.

### `BooleanValue(Boolean)` is two layers

Mental model is `BooleanValue { True, False }`. The parser has `enum Boolean { True, False }` plus `struct BooleanValue(pub Boolean)` so resolve-position has a node. `NullValue` is a unit struct. `BooleanValue` can be the enum.

### Dead token kinds sit on every match

`EndOfFile` is never emitted (`tokenize` stops at the last real token). `NonBracketTokenKind` still carries it, so every `From` / `Display` / `SplitToken` match pretends it exists. `SemanticToken::Content` is never recorded. `Expectation::Description` and `Expectation::SelectionSet` exist only for Display tests.

### Wrapper interned keys have inconsistent visibility

`EntityNameWrapper(pub ...)`, `SelectionNameWrapper(pub ...)`, `VariableNameWrapper(pub ...)` vs private `SelectableNameWrapper`, `ArgumentNameWrapper`, `StringLiteralValueWrapper`. No rule distinguishes them.

### String / description values are lexemes, not values

`Description` is documented as the source slice including quotes. `parse_string_literal` uses the same `interned()` path, so a value `"hi"` is interned as `"\"hi\""`, and `"\\n"` is not a newline. The lexer accepted escape sequences and then the parser discarded that work. `StringLiteralValue` is the wrong representation if later passes compare to GraphQL string values.

## Structure

### Four trees, then the first chunk tree is thrown away and cloned back

`MatchedBrackets` and `ChunkedLevel` are the same nesting with different item types (`Bracketed` vs `ChunkedGroup`). `chunk` walks the first to build the second, copying every opening and closing. `parse_iso_literal` then drops the root `ChunkedLevel` except extra chunks. Failed or leftover regions clone `ChunkContentItem` trees into `UnparsedChunkItems` so resolve-position still has somewhere to walk.

That is wasted allocation and a split source of truth. Either keep the chunk tree and have `Slot` point into it, or parse into the grammar tree during chunking and stop cloning.

### Trailing separators of a parsed chunk leave the tree

A successful `entrypoint Query.foo,` reports the comma as a `ParseError` and then the comma is not a node. Resolve on it hits the singleton / slot unmatched span. Cut unmatched brackets are also gone: `entrypoint Query.foo)` parses, and the `)` is only in `BracketError`, not in the tree. Hover and highlighting cannot see those characters as tokens.

### Semantic tokens stop at the first failure in a chunk

`an_unknown_keyword_records_keyword_at_that_identifier` records `fieldd` as `Keyword` and nothing after it. Leftover after a successful item is also unrecorded (`leftover_after_an_entrypoint_is_not_recorded`). `SemanticToken::Error` / `Content` look like they were meant to cover that and are unused.

`@` is recorded as `DirectiveName`, and the name is too. `!` is recorded as `GraphQLTypeName`. Roles are caller-supplied strings, not a function of the token, which is correct, but several of those roles are lies.

### Keyword-as-identifier is copy-pasted

`entrypoint` / `field`, `to`, and `true` / `false` / `null` are all "require Identifier, then match the source slice." `consume_to_target` peeks, compares to `"to"`, then `require_token` with `Keyword`. `parse_boolean_or_null` records `BooleanOrNull` before checking the word, so `a: yes` highlights `yes` as boolean/null and then errors. A `consume_keyword` that records only on match would remove the duplication and the bad highlight.

### `parse_each_chunk` is the one good shared seam; tests do not use a shared harness

`consume_selection_set`, `consume_argument_list`, `consume_variable_declaration_list`, object interiors, and list interiors all go through `parse_each_chunk`. That is the right extraction.

`span_of`, `parsed_items`, and the dummy parent-cursor setup are duplicated in `arguments.rs` and `selections.rs` tests. `crates/tests` is an empty crate.

### `lib.rs` glob-exports every module

Covered by the combined `parse_iso_literal` entry above. Explicit `pub use` of the AST, errors, tokens, and that function. Pipeline stages are `pub(crate)`.

### `impl std::error::Error for Expectation`

`Expectation` is a fragment of a diagnostic. `ParseError` is the error. The impl does not buy `thiserror` anything (`Expected(ExpectedFound)` already displays). Three number-error Displays are the identical string `"unsupported number (int or float) literal"`, so the three variants are indistinguishable in user text.

### Commented-out grammar in `token_kind.rs`

Spread, comments, `Pipe`, `PeriodPeriod` sit as comments, plus `TODO don't skip comments and spaces`. The crate rule is that a comment must not describe what was not done. `observe_kinds` is the same residue in test form.

## Grammar sharp edges (tested, still wrong for a GraphQL-shaped language)

Line break and comma are the same chunk separator. These are tests, not accidents:

- `field Query.Foo\n{ bar }` is a field with no selection set plus `MultipleDeclarations` on the brace.
- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`.
- `[Pet\n!]` does not attach the bang to `Pet`.

Spaces do not split. Newlines do. Anyone who formats a selection set or a `to` clause onto the next line gets a second declaration. If that is the language, the diagnostic should say so (`expected the selection set on the same line`). It currently says `Expected nothing after the declaration`.

`#` comments are `Error` plus identifiers. There is no comment token. The skip regex skips only `[ \t\f\ufeff]+`.

## What is in good shape

Bracket matching with cut-and-diagnose is consistent and well tested. Crossing `foo { (} )` and unclosed interiors behave as documented. Chunking's `CommaWithoutItem` vs trailing comma is the right split. Per-chunk recovery (`each_malformed_variable_declaration_degrades_alone`, leftover keeps the item) is the right parser architecture. `SafePeekable` / `ItemCursor` make "peek without consume" a lifetime, not a boolean. `parse_name_colon` is the right helper for `name: value`. Resolve-position coverage on the grammar tree is thorough.

The next work is `Null` on `TypeAnnotation` (with the tests above), the combined `parse_iso_literal(text)` entry, and making `VariableDeclarationOrUsage` the shared `$name` node whose parent is `Declaration | Usage`. `Slot` and `parse_singleton` wait in parser-minor-improvements.md.
