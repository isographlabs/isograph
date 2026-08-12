# Parsing plan: the grammar stage

The grammar stage turns a chunked literal into a declaration. It is the last parser pass: the caller composes `tokenize`, `match_brackets`, `chunk`, and then this stage's entry point, which consumes the chunk tree by value and produces the tree that positions resolve against from here on.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`text` is the literal itself, the same string the earlier passes ran over. The stage reads it only to recognize keyword identifiers (`entrypoint`, `field`, `pointer`, `to`, `true`, `false`, `null`) and to convert integer literals; every name in the output is a span into it, never a copied or interned string. The one parsed scalar is the integer value of an integer literal, because deferring the `i64` conversion forces a later stage to reparse text and handle overflow far from the source (upstream panics on overflow; we produce an error).

The stage parses the same language as upstream isograph's `parse_iso_literal`, with the deliberate changes listed below. Where upstream and this stage disagree on an input's validity, the difference must appear in that list; anything else is a bug.

## The golden rule: one chunk, one item

A chunk parses to exactly one grammar item, and an item never continues past a separator into the next chunk:

- the declaration is one root-level chunk;
- a selection is one chunk of its brace group's interior level;
- an argument, a variable declaration, and an object-literal entry are one chunk of their group's interior level;
- the element type of a `[...]` type annotation is the bracket level's one chunk.

Composite items own groups within their chunk: `foo(arg: 1) { bar }` is one selection chunk whose paren and brace groups are the selection's arguments and selection set.

## Boundary rules

A boundary is a chunk's trailing separator run: any number of line breaks and at most one comma. The rule of the language is that a comma must follow an item; line breaks are free.

- A trailing comma is fine everywhere a boundary sits, the root level and the interior of a `[...]` type included. A comma before a level's first item is an error, as is a second comma in one boundary.
- No chunk requires a trailing boundary: `{ bar }` on one line parses, where upstream demanded a comma or line break after every selection, the last included. `{}` and `{\n}` are empty selection sets.
- The earlier passes make the rule structural (one-comma-per-boundary.md, a prefactor to this series): the bracket matcher consumes the line breaks directly after an opening bracket as it parses the opening (and at the literal's start for the root level), and the chunking pass's boundary phase stops before a second comma, so an empty chunk exists exactly when a comma has no item before it, holding that comma as its boundary's first token. The grammar stage reports every empty chunk as a missing item, `Expected(<the level's item>, found ',')` at the comma, and never inspects a contentful chunk's boundary. An error-free parse contains no empty chunk.

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

2. Separator placement loosens, on the trailing side only. A single trailing comma is valid everywhere a boundary sits, the root level and `[...]` interiors included, and no trailing separator is ever required, so `{ bar }` on one line parses. Upstream rejected those. Leading and doubled commas stay errors, as upstream.

3. Directives are deferred. Upstream parsed `@name(args)` on declarations and on selections; this series does not, and a later series adds them back. Until then an `@` is an ordinary unexpected token: `field Query.Foo @component { ... }` reports `Expected(<a selection set>, found '@')`.

4. An integer literal whose value does not fit in `i64` is a typed parse error. Upstream panics.

## The error model

Every literal yields a tree; the parse never fails to return one. Malformed regions degrade to unparsed nodes that hold the reason and the chunk-stage data they cover, so every error is representable in the tree and every position inside a degraded region still resolves. `errors()` on the result collects the reasons in source order; there is no error list beside the tree.

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

`Expectation` names what the grammar wanted (a specific token, a selection, a value, a type, the end of the declaration, ...) and grows a few variants per doc; `Found` names what sat there (a token kind, a group, an unmatched bracket, or `EndOfChunk` when the chunk ran out, so a missing trailing piece is `Expected(<x>, found nothing more)`). All messages live in `Display` impls in the parser crate. Contextual suggestions (a fragment-spread hint on `Expected(<a selection>, found '.')`, a directive-migration hint on a found `@`, a "perhaps you meant to remove this line break" hint on an orphaned group) belong to the rendering stage, keyed off the `(expected, found)` pair; the parser never carries prose.

A reason is a `WithSpan<ParseError>`; the span points at the offending item, or is an empty span at the position where a missing item was expected.

Failure granularity starts coarse and refines:

- parse-entrypoint.md degrades the whole literal: any failure produces `UnparsedLiteral`, holding the reason and the entire root `ChunkedLevel`.
- parse-fields.md introduces per-item degradation with `UnparsedItem`: a list chunk that fails to parse becomes an unparsed item holding its cloned chunk, and its siblings parse normally. Declaration-header errors keep degrading the whole literal.

## The resolution surface

`IsographResolutionNode` keeps its role as the leaves of the newest tree. Each doc swaps or adds variants in place: parsed regions resolve to grammar-stage leaves (a declaration, a name, a selection, a value), and the chunk-stage variants remain because unparsed nodes hold chunk-stage data, which resolves through the existing chunk paths. The chunk-stage parent enums gain variants pointing back into the grammar tree (`ChunkedLevelParent` gains `UnparsedLiteral` in parse-entrypoint.md; `Chunk`'s parent becomes an enum with an `UnparsedItem` variant in parse-fields.md), so an ancestry walk from a token inside a degraded region reaches the grammar tree that holds it.

Name leaves are fieldless marker structs (`EntityName`, `SelectionName`, `VariableName`, ...) whose text is their span; distinct roles are distinct types even when the shape is identical, so the resolution surface distinguishes an alias from a name and an argument name from an object key.

## What later stages own

- Directives, when they return.
- Semantic tokens. The finished tree plus spans determines them, so a separate walk derives them when the LSP needs them; upstream interleaved them with parsing.
- Extraction context. `const_export_name`, the definition file path, and the "must be exported" check belong to the stage that extracts literals from files. This stage sees only the text between the backticks, and a missing export is not a malformed literal.
- Diagnostics rendering: turning `WithSpan<ParseError>` plus the literal text into printed messages, including the contextual suggestions keyed off `(expected, found)` pairs.
- Smarter recovery, for example treating a top-level `{ ... }` after a failed header as a selection set. The series builds the minimal correct version first.
- Span-slot genericity: this series builds `Span`-only trees. refactors/pending/spanless-parsing.md said to decide the `TSpan` parameter together with this stage; the decision here is to not adopt it now, and adopting it later is the mechanical change that doc describes.

## The docs, in order

Each doc is independently shippable and lands with its tests before the next begins.

1. `one-comma-per-boundary.md`. The chunking prefactor: an opening bracket captures the line breaks directly after it, the boundary phase stops before a second comma, and an empty chunk therefore exists exactly when a comma has no item before it. `ChunkSeparator` invariantly holds at most one comma.
2. `parse-entrypoint.md`. The skeleton everything else extends: `parse_iso_literal`, the root-level rules (one contentful chunk; every empty chunk an error at its comma), keyword dispatch, `ParseError` with its `Display` impls, `UnparsedLiteral` with its resolution fallback, and the complete `entrypoint Type.field` declaration. `field` and `pointer` dispatch to a temporary `UnsupportedDeclarationType` error that parse-fields.md and parse-pointers.md remove.
3. `parse-fields.md`. `field Type.name { ... }` with selection sets: scalar selections, `alias: name`, object selections with nested selection sets, `UnparsedItem` and the shared level-walking helper, and the parent-enum conversions second parents force (`EntityName`, `ClientFieldName`, `Chunk`). Adds `Clone` to the chunk tree so unparsed items can own their chunks. Arguments are not yet parsed: a paren group inside a selection is that selection's unparsed reason until the next doc.
4. `parse-arguments.md`. Argument lists on selections, `name: value` pairs, and values: variable, string, integer (with the `i64` conversion and `IntegerOutOfRange`), boolean, null, and object literals with their entry chunks.
5. `parse-variables.md`. Variable-declaration lists on field declarations, `$name: Type = default`, type annotations (named, `!`, and `[...]` with its one-chunk interior), the constant-value restriction on defaults, and the `Box` delegation impl the recursion needs in `resolve_position`.
6. `parse-descriptions.md`. The optional description (string or block string) a field declaration carries before its selection set, and that pointers reuse.
7. `parse-pointers.md`. `pointer Type.name to Type { ... }`, reusing type annotations, descriptions, and selection sets, and removing `UnsupportedDeclarationType`.
