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

`text` is the literal itself. The stage reads it to recognize keyword identifiers (`entrypoint`, `field`, `pointer`, `to`, `true`, `false`, `null`), to intern names (`token.interned()`), and to convert integer literals to `i64`. The wrapper span is location only.

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
   { bar }          <- a scalar selection, then a failed slot on the orphaned group

   field Query.Foo
   { bar }          <- an error at the end of the header chunk

   ($x:
   String)          <- an error

   [String
   !]               <- an error
   ```

2. No trailing separator is ever required: `{ bar }` on one line parses. Trailing commas in lists parse. A comma at the root, inside `[...]`, before a list's first item, or doubled is an error.

3. Directives are deferred. An `@` is an ordinary unexpected token.

4. An integer literal whose value does not fit in `i64` is `IntegerDoesNotFitI64`.

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
    #[error("This declaration type is not supported yet.")]
    UnsupportedDeclarationType,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
    #[error("a comma, a line break, or {}", .0.closing())]
    Separator(BracketKind),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
    #[error("a variable declaration, like '$id: ID!'")]
    VariableDeclaration,
    #[error("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[error("a constant value; variables are not allowed here")]
    ConstantValue,
    #[error("the end of the type")]
    EndOfType,
    #[error("the keyword `to`")]
    ToKeyword,
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

## What later stages own

- Directives, when they return.
- Semantic tokens: semantic-tokens.md.
- Extraction context (`const_export_name`, definition path, export check).
- Diagnostics rendering.
- Synthetic closing of unclosed groups: unclosed-group-recovery.md.
- Span-slot genericity: spanless-parsing.md.
- Storing leftover as a range into the original chunk instead of a clone.

## The docs, in order

parsing-standards.md governs how every implementation below is written. Each doc is independently shippable and lands with its tests before the next begins.

1. `from-container-parent-field.md`. `#[from_container_parent]` on a struct field. `Slot.extra_tokens` stays bare until parse-arguments.md.
2. `parse-arguments.md`. `Separator(BracketKind)`. Argument lists and values: variable, string, integer (`i64` / `IntegerDoesNotFitI64`), `BooleanValue(Boolean::{True, False})`, null, and object literals. `NamedArgument` and `ObjectEntry` pins, `UnparsedChunkItemsParent`. Tests feed a list interior to `parse_each_chunk`.
3. `parse-selection-sets.md`. Scalar selections, `alias: name`, object selections, argument lists on those selections. Tests feed a list interior to `parse_each_chunk`.
4. `parse-fields.md`. `field Type.name { ... }` via `require_selection_set`. Resolve-from-the-declaration tests.
5. `parse-variables.md`. Variable-declaration lists, `$name: Type = default` with `ConstantValue` defaults, type annotations (named, `!`, and `[...]` via `parse_singleton`), and the `Box` delegation impl.
6. `parse-descriptions.md`. The optional description a field declaration carries before its selection set, via two `consume_token_if` calls.
7. `parse-pointers.md`. `pointer Type.name to Type { ... }` via `require_token(Identifier)` and `token_text == "to"`. Removes `UnsupportedDeclarationType`.

Later: `parse-arrays.md`. `[ ... ]` list values. `parse-variables.md` uses them for defaults. `constant-value.md`. One value type instead of `ConstantValue` beside `NonConstantValue`.
