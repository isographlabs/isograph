# Parsing plan: the grammar stage

The grammar stage turns a chunked literal into a declaration. It is the last parser pass: the caller composes `tokenize`, `match_brackets`, `chunk`, and then this stage's entry point, which consumes the chunk tree by value and produces the tree that positions resolve against from here on.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`text` is the literal itself, the same string the earlier passes ran over. The stage reads it only to recognize keyword identifiers (`entrypoint`, `field`, `pointer`, `to`, `true`, `false`, `null`) and to convert integer literals; every name in the output is a span into it, never a copied or interned string. The one parsed scalar is the integer value of an integer literal, because deferring the `i64` conversion forces a later stage to reparse text and handle overflow far from the source (upstream panics on overflow; we produce an error).

The stage parses the same language as upstream isograph's `parse_iso_literal`, with the deliberate changes listed below. Where upstream and this stage disagree on an input's validity, the difference must appear in that list; anything else is a bug.

## The golden rule: one chunk, one item

A chunk parses to exactly one grammar item, in its entirety and always independently, and the item is a result: the parsed item, or an unparsed item holding the reason and the chunk. An item never continues past a separator into the next chunk:

- the declaration is one root-level chunk;
- a selection is one chunk of its brace group's interior level;
- an argument, a variable declaration, and an object-literal entry are one chunk of their group's interior level;
- the element type of a `[...]` type annotation is the bracket level's one chunk.

Composite items own groups within their chunk: `foo(arg: 1) { bar }` is one selection chunk whose paren and brace groups are the selection's arguments and selection set.

## Boundary rules

A boundary is a chunk's trailing separator run. The model: line breaks are swallowed by whatever precedes them, and commas are swallowed by nothing. An opening bracket, or the literal's start, swallows the line breaks at its level's start as it is parsed; an item swallows the line breaks after it. A comma is meaningful only inside a list: between two items, or after the last one.

- In a list (a selection set, a paren list, an object literal), a boundary carries at most one comma, sitting anywhere among the boundary's line breaks, and a trailing comma after the last item is fine: `{ bar, }`, `{ bar,\n }`, and pathologically `{ bar\n, }` all parse.
- The trailing comma is legal because ease of generation is a goal of the language: a generator emits `item,` uniformly, with no special case for the last item. The rule stops there as a balance against aesthetics: commas anywhere would be easier still to generate, but one comma per boundary, in lists only, keeps doubled and stray commas caught as the mistakes they are.
- In a one-item context (the root level, the interior of a `[...]` type), no comma is valid at all: `entrypoint Query.foo,`, `field Query.Foo { },`, and `[Pet,]` are errors at the comma. The leading and doubled cases are empty chunks and error from the start; the final-comma case is its own doc, no-final-comma.md, landing after the feature docs.
- No chunk requires a trailing boundary: `{ bar }` on one line parses, where upstream demanded a comma or line break after every selection, the last included. `{}` and `{\n}` are empty selection sets.
- The earlier passes make the comma rule structural: the bracket matcher swallows the line breaks after an opening as it parses it (landed; refactors/past/one-comma-per-boundary.md), the chunking pass's boundary phase stops before a second comma, and chunking emits `CommaWithoutItem` for a comma no item precedes, dropping the empty chunk it would have opened (landed; refactors/past/no-empty-chunks.md). No empty chunk exists at the grammar stage, and the only boundary it inspects is a one-item context's, for the final comma no list gives meaning there.

## Language changes relative to upstream

1. Separators are structure. A comma or line break inside what upstream read as one item now ends the item, and the remainder is its own chunk, which then fails to parse as an item:

   ```
   foo
   { bar }          <- upstream: one object selection; now: a scalar selection, then an unparsed item on the orphaned group

   field Query.Foo
   { bar }          <- upstream: one declaration; now: an error at the end of the header chunk

   ($x:
   String)          <- upstream: one variable declaration; now: an error

   [String
   !]               <- upstream: one list type; now: an error
   ```

2. No trailing separator is ever required: `{ bar }` on one line parses, where upstream demanded a comma or line break after every selection, the last included. Comma placement otherwise matches upstream: trailing commas in lists parse, and a comma at the root, inside `[...]`, before a list's first item, or doubled is an error.

3. Directives are deferred. Upstream parsed `@name(args)` on declarations and on selections; this series does not, and a later series adds them back. Until then an `@` is an ordinary unexpected token: `field Query.Foo @component { ... }` reports `Expected(<a selection set>, found '@')`.

4. An integer literal whose value does not fit in `i64` is a typed parse error. Upstream panics.

## The error model

Every literal yields a tree; the parse never fails to return one. Malformed regions degrade to unparsed nodes that hold the reason and the chunk-stage data they cover, so every error is representable in the tree and every position inside a degraded region still resolves. Leftover after a successful list item is `ParsedSlot::trailing`. `errors()` on the result collects the reasons in source order; there is no error list beside the tree.

There is one error enum, `ParseError`, and its workhorse variant is generic, in the shape of upstream's token errors:

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum ParseError {
    /// "Expected {expected}, found {found}."
    Expected(ExpectedFound),
    EmptyLiteral,
    MultipleDeclarations,
    UnsupportedDeclarationType,
    IntegerOutOfRange,
}

pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}
```

`Expectation` names what the grammar wanted (a specific token, a selection, a value, a type, the end of the declaration, ...) and grows a few variants per doc; `Found` names what sat there (a token kind, a group, or `EndOfChunk` when the chunk ran out, so a missing trailing piece is `Expected(<x>, found nothing more)`). Bracket errors never appear here: the matcher returned them beside its tree (landed; refactors/past/cut-at-unmatched.md), and no `ParseError` variant names a bracket. All messages live in `Display` impls in the parser crate. Contextual suggestions (a fragment-spread hint on `Expected(<a selection>, found '.')`, a directive-migration hint on a found `@`, a "perhaps you meant to remove this line break" hint on an orphaned group) belong to the rendering stage, keyed off the `(expected, found)` pair; the parser never carries prose.

A reason is a `WithSpan<ParseError>`; the span points at the offending item, or is an empty span at the position where a missing item was expected.

Failure granularity starts coarse and refines:

- parse-entrypoint.md: any failure produces `UnparsedLiteral`, which stores the reason and the entire root `ChunkedLevel`. `parse_singleton` returns those errors: empty literal, extra chunk, leftover, trailing comma.
- parse-fields.md introduces per-item degradation with `LevelSlot`: a list chunk that fails to parse becomes `LevelSlot::Unparsed` holding its cloned chunk; leftover after a successful item is `ParsedSlot::trailing`. Siblings parse normally. Declaration-header errors keep degrading the whole literal.

## The resolution surface

`IsographResolutionNode` keeps its role as the leaves of the newest tree. Each doc swaps or adds variants in place: parsed regions resolve to grammar-stage leaves (a declaration, a name, a selection, a value), and the chunk-stage variants remain because unparsed nodes hold chunk-stage data, which resolves through the existing chunk paths. The chunk-stage parent enums gain variants pointing back into the grammar tree (`ChunkedLevelParent` gains `UnparsedLiteral` in parse-entrypoint.md; `Chunk`'s parent becomes an enum with an `UnparsedItem` variant in parse-fields.md), so an ancestry walk from a token inside a degraded region reaches the grammar tree that holds it.

Name leaves are fieldless marker structs (`EntityName`, `SelectionName`, `VariableName`, ...) whose text is their span; distinct roles are distinct types even when the shape is identical, so the resolution surface distinguishes an alias from a name and an argument name from an object key.

## What later stages own

- Directives, when they return.
- Semantic tokens. The finished tree plus spans determines them, so a separate walk derives them when the LSP needs them; upstream interleaved them with parsing. The layering rule (semantic where parsed, lexical where not, errors as diagnostics) is semantic-tokens.md.
- Extraction context. `const_export_name`, the definition file path, and the "must be exported" check belong to the stage that extracts literals from files. This stage sees only the text between the backticks, and a missing export is not a malformed literal.
- Diagnostics rendering: turning `WithSpan<ParseError>` plus the literal text into printed messages, including the contextual suggestions keyed off `(expected, found)` pairs.
- Smarter recovery, for example treating a top-level `{ ... }` after a failed header as a selection set, and synthetic closing of unclosed groups (unclosed-group-recovery.md), a future optimization over the cut. The series builds the minimal correct version first.
- Span-slot genericity: this series builds `Span`-only trees. refactors/pending/spanless-parsing.md said to decide the `TSpan` parameter together with this stage; the decision here is to not adopt it now, and adopting it later is the mechanical change that doc describes.

## The docs, in order

parsing-standards.md governs how every implementation below is written. Each doc is independently shippable and lands with its tests before the next begins:

0. `chunk-contents-nonempty.md`. `Chunk::contents` and `ChunkSeparator` become `nonempty::NonEmpty`. A comma no item precedes is already a `CommaWithoutItem`; this doc makes the empty-contents state unrepresentable.
1. `parse-entrypoint.md`. The skeleton: `LiteralText`, `ItemCursor` / `ChunkStream`, `parse_singleton`, `parse_iso_literal`, keyword dispatch, `ParseError`, `UnparsedLiteral`, and `entrypoint Type.field`. `field` and `pointer` dispatch to a temporary `UnsupportedDeclarationType` error that parse-fields.md and parse-pointers.md remove.
2. `parse-fields.md`. `field Type.name { ... }` with selection sets: scalar selections, `alias: name`, object selections, `LevelSlot` / `parse_items`, and the parent-enum conversions second parents force. Adds `Clone` to the chunk tree so unparsed items can own their chunks. Arguments are not yet parsed: a paren group after a selection name is that selection's trailing leftover until the next doc.
3. `parse-arguments.md`. Argument lists on selections, `name: value` pairs, and values: variable, string, integer (with the `i64` conversion and `IntegerOutOfRange`), `BooleanValue(Boolean::{True, False})`, null, and object literals.
4. `parse-variables.md`. Variable-declaration lists, `$name: Type = default` with `ConstantValue` defaults, type annotations (named, `!`, and `[...]` via `parse_singleton`), and the `Box` delegation impl the recursion needs in `resolve_position`.
5. `parse-descriptions.md`. The optional description (string or block string) a field declaration carries before its selection set, via `consume_token_if_any`.
6. `parse-pointers.md`. `pointer Type.name to Type { ... }` via `require_keyword("to")`, reusing type annotations, descriptions, and selection sets, and removing `UnsupportedDeclarationType`.
7. `no-final-comma.md`. No new code: the test matrix for `parse_singleton`'s comma check, already written into the feature docs above.
