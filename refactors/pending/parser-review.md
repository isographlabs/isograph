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

### `!` is consumed and dropped. Nullability is not in the tree (moved)

Moved to type-annotation-null.md.

### `parse_iso_literal` has no single entry point and three error channels (moved)

Moved to parse-iso-literal-entry.md.

### Test suite is red on purpose in one case (fixed)

`tokenize::tests::observe_kinds` is not in the tree.

## Invariants not encoded in types

`Slot` and `parse_singleton` (deferred): parser-minor-improvements.md.

### `VariableDeclarationOrUsage` is only a declaration (specified)

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

-----

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

### `lib.rs` glob-exports every module (moved)

Moved to parse-iso-literal-entry.md Change 3.

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

The next work is making `VariableDeclarationOrUsage` the shared `$name` node whose parent is `Declaration | Usage`. Combined parse is parse-iso-literal-entry.md. Nullability is type-annotation-null.md. `Slot` and `parse_singleton` wait in parser-minor-improvements.md.
