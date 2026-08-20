# Parsing plan: the grammar stage

The grammar stage turns a chunked literal into a declaration. The caller composes `tokenize`, `match_brackets`, `chunk`, and then this stage's entry point.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>>
```

`text` is the literal itself. The stage reads it to recognize keyword identifiers (`entrypoint`, `field`, `to`, `true`, `false`, `null`), to intern names (`token.interned()`), and to convert integer literals to `i64`. The wrapper span is location only.

The stage parses the same language as upstream isograph's `parse_iso_literal`, with the deliberate changes listed below. Where upstream and this stage disagree on an input's validity, the difference must appear in that list; anything else is a bug.

## The golden rule: one chunk, one item

A chunk parses to exactly one grammar item, in its entirety and always independently. `parse_one_chunk` returns `Slot<P, UnparsedChunkItems>`: `item: Some` when the form parsed, `extra_tokens: Some` when unread or failed items remain. An item never continues past a separator into the next chunk:

- the declaration is one root-level chunk;
- a selection is one chunk of its brace group's interior level;
- an argument, a variable declaration, and an object-literal entry are one chunk of their group's interior level;
- the element type of a `[...]` type annotation is the bracket level's one chunk.

Composite items own groups within their chunk: `foo(arg: 1) { bar }` is one selection chunk whose paren and brace groups are the selection's arguments and selection set.

## Boundary rules

A boundary is a chunk's trailing separator run. Line breaks are swallowed by whatever precedes them. A comma is meaningful only inside a list: between two items, or after the last one.

- In a list (a selection set, a paren list, an object literal), a boundary carries at most one comma, sitting anywhere among the boundary's line breaks, and a trailing comma after the last item is fine: `{ bar, }`, `{ bar,\n }`, and `{ bar\n, }` all parse.
- In a one-item context (the root level, the interior of a `[...]` type), no comma is valid: `entrypoint Query.foo,`, `field Query.Foo { },`, and `[Pet,]` are `Expected(end, Token(Comma))` at the comma, via `parse_singleton`.
- No chunk requires a trailing boundary: `{ bar }` on one line parses. `{}` and `{\n}` are empty selection sets.

## Language changes relative to upstream

1. Separators are structure. A comma or line break inside what upstream read as one item now ends the item, and the remainder is its own chunk, which then fails to parse as an item:

   ```
   foo
   { bar }          <- a selection, then a failed slot on the orphaned group

   field Query.Foo
   { bar }          <- an error at the end of the header chunk

   ($x:
   String)          <- an error

   [String
   !]               <- an error
   ```

2. No trailing separator is ever required: `{ bar }` on one line parses. Trailing commas in lists parse. A comma at the root, inside `[...]`, before a list's first item, or doubled is an error.

3. Directives land in parse-directives.md. Until that doc lands, `@` is leftover.

4. An integer literal whose value does not fit in `i64` is `IntegerDoesNotFitI64`.

5. A selection that starts with `.` is `Expected(Selection, Token(Period))`. Upstream emits a fragment-spread diagnostic for `...`.

6. Variable defaults accept `$`. One value type (`NonConstantValue`). Upstream parses a `ConstantValue` and rejects `$` at the `$`.

7. There is no `pointer` keyword. `field Type.name to Type { ... }` is a field with `target_type: Some`. Upstream's `pointer Type.name to Type { ... }` is `DECLARATION_KEYWORD` at `pointer`.

## The error model

`parse_iso_literal` returns `None` on an empty chunked literal and a tree otherwise. Diagnostics go through `report_error` on a cursor, or `errors.push` at the root where there is no cursor. They are not stored on the tree. A failed chunk is `Slot { item: None, extra_tokens: Some(the chunk's items) }`. Leftover after a successful item is `item: Some` plus leftover items. Extra root chunks sit on `IsoLiteralParse.extra_chunks`.

```rust
// from crates/isograph_parser/src/parse_error.rs
use thiserror::Error;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(NonBracketTokenKind),
    Keyword(&'static str),
    Description,
    OneOf(&'static [Expectation]),
    EndOfDeclaration,
    SelectionSet,
    Selection,
    Separator(BracketKind),
    Argument,
    Value,
    ObjectEntry,
    VariableDeclarationOrUsage,
    TypeAnnotation,
    EndOfType,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Found {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("a group opened by {0}")]
    Group(BracketKind),
    #[error("nothing more")]
    EndOfChunk,
}

// from crates/isograph_parser/src/non_bracket_token.rs
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, strum::Display)]
pub enum BracketKind {
    #[strum(to_string = "'('")]
    Parenthesis,
    #[strum(to_string = "'{'")]
    Brace,
    #[strum(to_string = "'['")]
    Bracket,
}

impl BracketKind {
    pub fn closing(self) -> &'static str {
        match self {
            BracketKind::Parenthesis => "')'",
            BracketKind::Brace => "'}'",
            BracketKind::Bracket => "']'",
        }
    }
}
```

`Expectation` names what the grammar wanted. `Found` names what sat there (a token kind, a group, or `EndOfChunk`). Bracket errors never appear here. Suggestions belong to the rendering stage, keyed off `(expected, found)`.

## The resolution surface

`IsographResolutionNode` is the leaves of the newest tree. Parsed regions resolve to grammar-stage leaves. Chunk-stage variants remain because leftover and failed items hold chunk-stage data. A name is a newtype over an interned key. A position on `.`, `$`, `!`, `:`, or `to` answers the containing node. There is no keyword-marker type.

## Names relative to isograph

Where a type or function exists in both, i2 uses the isograph name. Wrappers that exist only so a lang type can carry `ResolvePosition` take the wrappee's name plus `Wrapper` (`EntityNameWrapper`, `VariableNameWrapper`, `ArgumentNameWrapper`, `ValueKeyNameWrapper`, `SelectionNameWrapper`, `SelectableNameWrapper`, `StringLiteralValueWrapper`, `IsographDirectiveNameWrapper`). `SelectionNameWrapper` is a selection name and a `reader_alias` (AST). `SelectableNameWrapper` is an entrypoint name and a field name (definition). The left-hand side of `Type.name` is `EntityNameWrapper`.

Justified differences:

- Slots, `UnparsedChunkItems`, `Singleton`, `ArgumentList`, `VariableDeclarationOrUsageList`, `IsographFieldDirectiveList`, `ListLiteralValue`: no isograph equivalent.
- `VariableDeclarationOrUsage` (not `VariableDeclaration`).
- `IsographResolutionNode` (not `IsographResolvedNode`): isograph-resolution-node.md; the enum does not rename per stage.
- `consume_*` / `require_*` (not `parse_optional_*`): parsing-standards.md.
- `ObjectEntry` (not `NameValuePair`): two slot `T`s, one per list parent.
- `VariableUse`, `IntegerValue`, `BooleanValue`, `NullValue`: resolve-position leaves; isograph inlines `i64` / `bool` / unit.
- `TypeAnnotation` as `Named` / `List` with `!` on the span (not `TypeAnnotationDeclaration` as `Scalar` / `Union` / `Plural`): i2 stores the written form; isograph converts from `GraphQLTypeAnnotation`.
- `Selection` is one struct with optional `selection_set` (not `SelectionType<ScalarSelection, ObjectSelection>`).
- Parent enums drop the `Type` suffix (`SelectionSetParent`, not `SelectionSetParentType`).
- `IsoLiteralItem` (not `IsoLiteralExtractionResult`): extraction is a different stage.
- `SelectableDeclaration` (isograph `ClientFieldDeclaration`).
- `ArgumentName` / `ArgumentNameWrapper` / `SelectionArgument` (isograph `FieldArgumentName` / `SelectionFieldArgument`).
- `SelectableNameWrapper` for entrypoint and field names (isograph `ClientScalarSelectableNameWrapper` / `ClientObjectSelectableName`). `SelectionNameWrapper` wraps `SelectionName` (isograph uses `SelectableName` as the interned key of a selection name).
- `SelectableDeclaration.target_type: Option<WithSpan<TypeAnnotation>>` (isograph has a separate `ClientPointerDeclaration` and a `pointer` keyword).
- `name` on entrypoint and field declarations (isograph `client_field_name`).
- Raw `IsographFieldDirectiveList` (not immediate serde into typed `*DirectiveSet`).
- `Description` stores quotes included (upstream unquotes and dedents).
- Empty optional lists are `None` (upstream empty `Vec` with a generated span).
- `parse_nested_singleton` (isograph has no chunk singleton).
- Defaults are `NonConstantValue` (isograph `ConstantValue`). `$` in a default is a variable use.

## What later stages own

- Typed directive sets (`from_isograph_field_directives`, `EntrypointDirectiveSet`, …).
- Description unquote and block-string dedent.
- Semantic tokens: semantic-tokens.md.
- Extraction context (`const_export_name`, definition path, export check).
- Diagnostics rendering.
- Synthetic closing of unclosed groups: unclosed-group-recovery.md.
- Span-slot genericity: spanless-parsing.md.
- Storing leftover as a range into the original chunk instead of a clone.
- Reachable variables as `HashSet<VariableName>` from a walk of `NonConstantValue`. A later pass panics if a context that forbids variables contains any.
- Checking that a `SelectionName` refers to a `SelectableName` that exists.

## The docs, in order

parsing-standards.md governs how every implementation below is written. Each doc is independently shippable and lands with its tests before the next begins.

1. `parse-variables.md`. Variable-declaration lists, `$name: Type = default` with `NonConstantValue` defaults, type annotations (named, `!`, and `[...]` via `parse_nested_singleton`), and the `Box` delegation impl.
2. `parse-type-dot-name.md`. Extract `Type.name` from entrypoint and field. No AST change.
3. `selectable-name-wrapper.md`. `SelectableNameWrapper` for entrypoint and field names. `FieldDeclaration`. `name` not `client_field_name`. `SelectionNameWrapper` stays.
4. `selection-name.md`. `SelectionNameWrapper` wraps `SelectionName`.
5. `expectation-one-of.md`. `Expectation::OneOf` and `Keyword`. `DeclarationKeyword` and `ToOrDescriptionOrSelectionSet` become `OneOf`.
6. `selectable-declaration.md`. `FieldDeclaration` is `SelectableDeclaration`. The keyword `field` stays.
7. `optional-field-selection-set.md`. `field Type.name` with no `{ }`. `selection_set` is `Option`.
8. `parse-directives.md`. `@name` and `@name(args)` on entrypoints, fields, and selections. Raw `IsographFieldDirectiveList`; typed sets are a later stage.

Later: `parse-arrays.md`. `[ ... ]` list values.

Deferred: `token-kind-zst.md`. `NonBracketTokenKind` variants carry a ZST; matching yields proof passed into `parse_*`.
