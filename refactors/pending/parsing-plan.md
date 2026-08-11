# Parsing plan: the grammar stage

The grammar stage turns a chunked literal into a declaration. It is the last parser pass: the caller composes `tokenize`, `match_brackets`, `chunk`, and then this stage's entry point, which consumes the chunk tree by value and produces the tree that positions resolve against from here on.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn parse_iso_literal(text: &str, root: WithSpan<ChunkedLevel>) -> WithSpan<IsoLiteralParse>
```

`text` is the literal itself, the same string the earlier passes ran over. The stage reads it only to recognize keyword identifiers (`entrypoint`, `field`, `pointer`, `to`, `true`, `false`, `null`); every name in the output is a span into it, never a copied or interned string. The one parsed scalar is the integer value of an integer literal, because deferring the `i64` conversion forces a later stage to reparse text and handle overflow far from the source (upstream panics on overflow; we produce an error).

The stage parses the same language as upstream isograph's `parse_iso_literal`, with the deliberate changes listed below. Where upstream and this stage disagree on an input's validity, the difference must appear in that list; anything else is a bug.

## The golden rule: one chunk, one item

A chunk parses to exactly one grammar item, and an item never continues past a separator into the next chunk:

- the declaration is one root-level chunk;
- a selection is one chunk of its brace group's interior level;
- an argument, a variable definition, and an object-literal entry are one chunk of their group's interior level;
- the element type of a `[...]` type annotation is the bracket level's one chunk.

Composite items own groups within their chunk: `foo(arg: 1) { bar }` is one selection chunk whose paren and brace groups are the selection's arguments and selection set.

## Boundary rules

A boundary is a chunk's trailing separator run. The chunking pass already guarantees that consecutive separators collapse into one boundary and that only a level's first chunk can be empty of contents.

- A boundary between two items of a delimited list (a selection set, a paren list, an object literal) carries at most one comma. A second comma in one boundary is an error.
- Every other boundary is line-break-only: every boundary at the root level, the leading boundary held by a level's empty first chunk, and any boundary inside a `[...]` type. A comma there is an error.
- A selection chunk requires a trailing boundary, the last selection before the `}` included. So `{ bar }` on one line is an error, as it is upstream, and the error message can suggest the fix. Chunks in paren lists and object literals do not require one: trailing delimiters there are optional, as upstream.

These rules reproduce upstream's accepted set exactly for every input in which no item spans a line break: upstream demanded one comma or line break after each list item and rejected doubled commas, and the rules above demand the same of each boundary.

## Language changes relative to upstream

1. Separators are structure. A comma or line break inside what upstream read as one item now ends the item, and the remainder is its own chunk, which then fails to parse as an item. Inputs upstream accepted that are now errors, each an opportunity for a message like "perhaps you meant to remove this line break":

   ```
   foo
   { bar }          <- upstream: one object selection; now: a scalar selection, then an error on the orphaned group

   field Query.Foo
   { bar }          <- upstream: one declaration; now: an error after the header chunk

   ($x:
   String)          <- upstream: one variable definition; now: an error

   [String
   !]               <- upstream: one list type; now: an error
   ```

2. There are no directives. Upstream parsed `@name(args)` on declarations and on selections; the language no longer contains them. An `@` where an item would continue produces a dedicated error, since `@component` in existing code is the likeliest way to hit it.

3. An integer literal whose value does not fit in `i64` is a typed parse error. Upstream panics.

## The error model

Every literal yields a tree; the parse never fails to return one. Malformed regions degrade to unparsed nodes that hold the reason and the chunk-stage data they cover, so every error is representable in the tree and every position inside a degraded region still resolves. `errors()` on the result collects the reasons in source order; there is no error list beside the tree.

There is one error enum, `ParseError`, whose variants grow doc by doc. A reason is a `WithSpan<ParseError>`; the span points at the offending tokens, or is an empty span at the position where a missing item was expected. Rendering messages (interpolating source text, printing carats) is a later stage's concern and not part of this series; each doc states its variants' intended messages in their doc comments.

Failure granularity starts coarse and refines:

- parse-entrypoint.md degrades the whole literal: any failure produces `UnparsedLiteral`, holding the reason and the entire root `ChunkedLevel`.
- parse-fields.md introduces per-item degradation: a selection chunk that fails to parse becomes an unparsed item holding its chunk, and its sibling selections parse normally.

## The resolution surface

`IsographResolutionNode` keeps its role as the leaves of the newest tree. Each doc swaps or adds variants in place: parsed regions resolve to grammar-stage leaves (a declaration, a name, a selection), and the chunk-stage variants remain because unparsed nodes hold chunk-stage data, which resolves through the existing chunk paths. The chunk-stage parent enums gain variants pointing back into the grammar tree (`ChunkedLevelParent` gains one for `UnparsedLiteral` in parse-entrypoint.md; `Chunk`'s parent becomes an enum when unparsed selection items arrive in parse-fields.md), so an ancestry walk from a token inside a degraded region reaches the grammar tree that holds it.

## What later stages own

- Semantic tokens. The finished tree plus spans determines them, so a separate walk derives them when the LSP needs them; upstream interleaved them with parsing.
- Extraction context. `const_export_name`, the definition file path, and the "must be exported" check belong to the stage that extracts literals from files. This stage sees only the text between the backticks, and a missing export is not a malformed literal.
- Diagnostics rendering: turning `WithSpan<ParseError>` plus the literal text into printed messages.
- Smarter recovery, for example treating a top-level `{ ... }` after a failed header as a selection set. The series builds the minimal correct version first.
- Span-slot genericity: this series builds `Span`-only trees. refactors/pending/spanless-parsing.md said to decide the `TSpan` parameter together with this stage; the decision here is to not adopt it now, and adopting it later is the mechanical change that doc describes.

## The docs, in order

Each doc is independently shippable and lands with its tests before the next begins. Later docs may go stale while earlier ones are iterated on; each is refreshed when it becomes active.

1. `parse-entrypoint.md`. The skeleton everything else extends: `parse_iso_literal`, the root-level rules (one contentful chunk, line-break-only boundaries), keyword dispatch, the `ParseError` enum, `UnparsedLiteral` with its resolution fallback, and the complete `entrypoint Type.field` declaration. `field` and `pointer` dispatch to a temporary `UnsupportedDeclarationType` error that the following docs remove.
2. `parse-fields.md`. `field Type.name { ... }` with selection sets: scalar selections, `alias: name`, object selections with nested selection sets, the per-selection unparsed item, the trailing-boundary requirement, the delimited-list comma rules, and the dedicated errors for an orphaned `{ ... }` chunk and for a leading `...` (upstream's fragment-spread message). Converts `EntityName`'s parent and `Chunk`'s parent to enums as second parents appear. Arguments are not yet parsed: a paren group inside a selection is that selection's parse error until the next doc.
3. `parse-arguments.md`. Argument lists on selections, `name: value` pairs, and values: variable, string, integer (with the `i64` conversion and its overflow error), boolean, null, and object literals with their entry chunks.
4. `parse-variables.md`. Variable-definition lists on field declarations, `$name: Type = default`, type annotations (named, `!`, and `[...]` with its one-chunk interior), and the constant-value restriction on defaults.
5. `parse-descriptions.md`. The optional description (string or block string) a declaration carries before its selection set.
6. `parse-pointers.md`. `pointer Type.name to Type { ... }`, reusing type annotations, descriptions, and selection sets, and removing `UnsupportedDeclarationType`.
