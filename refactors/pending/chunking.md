# Chunking: the pass after bracket matching

The next pass. The literal is assumed to be a selection set, and chunking regroups each level of the matched-brackets tree into chunks so that a malformed chunk cannot affect another chunk. The goal at this stage is to chunk by selection; the same pass later chunks per argument, per array item, and per key-value pair, because the separator rule is universal and the chunking code is one function that runs at every level. Each chunk is parsed independently, and materializing the chunk lists per level is a price we pay knowingly: resilience is the priority, not allocation counts.

```
foo { bar, baz
qux }
```

is four chunks: `foo { ... }` at the top level, containing the three chunks `bar`, `baz`, and `qux`.

The containment always goes through a level: `foo { bar }` is one chunk, which contains a selection set containing a single chunk. The chunk holds the run `foo` and the brace group; the group's interior is a selection set of its own, with `bar` as its one chunk.

These rules are authoritative as written; old isograph is no longer the source of truth for them.

## The rules

- Line breaks and commas are the separators, and they are equivalent, at every level. One field per line, or fields delimited by commas. Any nonempty mix of consecutive separators is one boundary; separators at the start or end of a level produce no empty chunks; a blank line is a boundary, not two.
- Separators are required between fields. This is a language decision, not just a chunking convenience: iso literals delimit like Go, Swift, or Python (statements end at line breaks), not like GraphQL, which demotes commas and newlines to insignificant whitespace and lets selections self-delimit. Requiring the separator is what makes `foo baz` one chunk with a helpful error ("expected a comma or line break before `baz`") instead of two silently accepted fields whose wrongness surfaces later as confusing schema-validation errors.
- A separator only separates at its own level. A nested bracket group is a single opaque item at the level it appears in, so the commas and line breaks inside it never split the enclosing level; they separate that group's own chunks when the walk descends into it. So

  ```
  a(
  ) {
  }
  ```

  is one chunk: both line breaks sit inside groups, and the top level sees `a`, the paren group, and the brace group with no comma or line break anywhere between them. Multi-line argument formatting can never split a selection.
- A chunk is a maximal separator-free sequence of a level's items. Run tokens split at separator tokens; a group glues onto whichever chunk is open where it appears, whatever its bracket kind. A group appearing right after a boundary opens a chunk of its own.
- Chunking is infallible. It validates nothing and emits no errors: every token that survived the bracket pass lands in some chunk. `baz watttt` is one chunk, a chunk that will produce an error when it is parsed, not two chunks and not a chunking error. Arguments, variable lists, and values are the same: a paren or square group just rides along in its chunk, and whether it belongs there is the chunk parser's question later. Boundaries come from separators and bracket structure only, never from token-shape heuristics.
- Each chunk is parsed independently. Token-level garbage inside a chunk is that chunk's error and nothing else's. A bracket-level malformation keeps the containment the bracket pass already gave it: an unclosed `(` still forces its group shut where the bracket rules say, and material swallowed into it stays chunked inside it. Chunking never widens or narrows that boundary.

One consequence to name: `foo\n{ bar }` is two chunks, because the line break separates the name from the brace group. A selection set's brace has to open on its field's line.

## Why a separate pass

Chunking after bracket matching is not an arbitrary factoring; the two passes have different context requirements.

- Bracket matching is inherently global. A close bracket's owner can be arbitrarily far up the enclosing stack, and a synthetic close is decided by a token that lexically belongs to a different level. It is a stack machine over the whole token stream.
- Chunking, under the universal separator rule, is perfectly level-local: given one level's items, split runs at separators and let groups ride along as opaque items, with zero cross-level state. It is a recursive per-level map over the bracket tree; `chunk(level)` splits the level's items and replaces each group's children with `chunk(children)`. The universal rule is what bought this: when per-level grammar differed, chunking needed the enclosing bracket kind as context.

A tandem pass with the same rules would produce the same output, so the question is only which factoring is cheaper to own. Separated, the chunker is about fifty lines, infallible, with no stack, no lookahead, and no error paths, because group extents are already settled; the bracket-recovery cases (crossing pairs, stray closes, a `{` swallowing everything to EOF) do not appear in it at all. Merged, every bracket-recovery path would also have to decide what happens to the chunk in progress, and every bracket edge case would multiply against every chunk edge case in the code and the tests. Old isograph intertwines them, and that intertwining is part of why it bails on the first error: a bracket problem and a grammar problem land in the same control flow.

The intermediate `MatchedBrackets` tree also has consumers of its own: it retains the separator tokens that the chunk tree discards, which anything wanting the lossless token sequence (the formatter, eventually comments and whitespace) reads.

## The formatter

Formatter policy follows from the error classes, per literal (literals are independent; one broken literal must not stop formatting of its neighbors):

- Bracket errors: bail on the literal. Formatting rewrites whitespace from the tree's nesting, and under a bracket error the nesting is a recovery bet; reformatting would physically commit the bet into the author's file. The gate is not a runtime check but the existing refinement boundary: the formatter takes the tree refined via `try_map` into a stage with `Infallible` error slots, so "the formatter never sees a stray close" is a property of its input type, and the bail is the `Err` arm carrying the same `BracketError`s the LSP reports.
- Missing-separator errors: repair. `foo bar` formats to `foo\nbar`. In this token language that inserts a separator token and turns one dirty chunk into two clean selections, so it is the canonical mechanical fix for the exact error the chunk parser reports, not whitespace normalization. Accepted consequence: the `first Name` typo gets split onto two lines and its wrongness moves to schema validation; the dose is small because autoformat is optional and the split is visible in the diff. This also makes the newline the canonical separator: formatted code converges on one field per line.
- Other chunk dirt: pass the chunk through verbatim and format its clean siblings. The structure around a dirty chunk is settled, so this is safe in a way it is not for bracket errors.

## Data structures and paths

The chunk tree reuses the bracket tree's types wherever the shapes agree, and the resolve_position machinery decides what reuse means: a node type has exactly one `Parent` and one `ResolvedNode` associated type, so a type reused across stages shares its path types across stages rather than getting a parallel family.

- Reused as-is: `Inner` (a chunk's runs are separator-free token runs), `OpenBracket`, `UnmatchedClose`, `Closing` (over a new `Chunked` stage marker with the same slots as `BracketsMatched`), and `WithSpan` everywhere.
- New types: `Chunks(Vec<WithSpan<Chunk>>)` for one level (the whole literal at the root, a group's interior below), `Chunk(Vec<WithSpan<ChunkItem>>)`, `ChunkItem { Inner, Group(ChunkGroup), StrayClose }`, and `ChunkGroup`, which is `Bracketed` one stage later: the same opening and closing, children chunked. All derive `ResolvePosition` with `#[resolve_field]`; the macro needs no extension for this.
- How the new paths relate to the old: they are one family. `BracketItemParent` grows a `Chunk` variant (a run's parent is a bracket-tree item slot or a chunk) and gets a name that stops lying, `ItemParent`; `OpenBracketParent` grows a `ChunkGroup` variant; `ResolvedBracketNode` becomes `ResolvedNode` and grows the `Chunks`, `Chunk`, and `ChunkGroup` variants, the one enum that keeps growing toward the full isograph path enum. The cost of sharing is that a parent enum can name a variant no tree of the current stage produces; the alternative is a duplicated type family per stage.
- A stray close rides in its chunk as an item, and an unclosed group keeps its synthetic closing, so the bracket errors stay derivable from the chunked tree the same way `errors()` derives them from the matched-brackets tree.

## Implementation plan

1. Add the `Chunked` stage marker implementing `TreeContents` (same slots as `BracketsMatched`), so `Closing<Chunked>` works unchanged.
2. Define `Chunks`, `Chunk`, `ChunkItem`, and `ChunkGroup` with their `ResolvePosition` derives.
3. Grow the shared path family in matched_brackets.rs, the only change to existing code: the `ItemParent` and `ResolvedNode` renames, the new variants, and the three new path aliases. Known blast radius: the old bracket tests destructure `OpenBracketParent` with an irrefutable `let` that must become `let ... else`.
4. Write the pass: `chunk(MatchedBrackets<BracketsMatched>) -> Chunks`, one recursive `chunk_items` function per level, splitting runs at separator tokens, collapsing consecutive separators, gluing groups and strays onto the open chunk, recursing into group interiors, joining spans so chunks exclude separators.
5. Add `errors()` on `Chunks`, so the pipeline tip still answers the errors query.
6. Land the unit tests: the four-chunk example, separator equivalence and collapsing, `baz watttt` as one chunk, group attachment on both sides of a boundary, the multi-line `a(\n) {\n}` selection as one chunk, nested separators not splitting the outer level, empty group and empty literal, chunk spans excluding separators, path-resolution walks (run to chunk to group to chunk to root, open brackets, separators falling through to the level, stray closes), and error derivation for strays and unclosed groups.
7. Register the module in lib.rs, confirm `cargo test` is green in CI, and move this doc to refactors/past.
