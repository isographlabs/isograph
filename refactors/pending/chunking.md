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
- A separator only separates at its own level. A nested bracket group is a single opaque item at the level it appears in, so the commas and line breaks inside it never split the enclosing level; they separate that group's own chunks when the walk descends into it.
- A chunk is a maximal separator-free sequence of a level's items. Run tokens split at separator tokens; a group glues onto whichever chunk is open where it appears, whatever its bracket kind. So `foo { ... }` is one chunk (name tokens plus the adjacent brace group), and the brace group's interior chunks independently by the same rules. A group appearing right after a boundary opens a chunk of its own.
- Chunking is infallible. It validates nothing and emits no errors: every token that survived the bracket pass lands in some chunk. `baz watttt` is one chunk — a chunk that will produce an error when it is parsed, not two chunks and not a chunking error. Arguments, variable lists, and values are the same: a paren or square group just rides along in its chunk, and whether it belongs there is the chunk parser's question later. Boundaries come from separators and bracket structure only, never from token-shape heuristics.
- Each chunk is parsed independently. Token-level garbage inside a chunk is that chunk's error and nothing else's. A bracket-level malformation keeps the containment the bracket pass already gave it: an unclosed `(` still forces its group shut where the bracket rules say, and material swallowed into it stays chunked inside it — chunking never widens or narrows that boundary.

One consequence to name: `foo\n{ bar }` is two chunks, because the line break separates the name from the brace group. A selection set's brace has to open on its field's line.

## Data structures and paths

The chunk tree reuses the bracket tree's types wherever the shapes agree, and the resolve_position machinery decides what reuse means: a node type has exactly one `Parent` and one `ResolvedNode` associated type, so a type reused across stages shares its path types across stages rather than getting a parallel family.

- Reused as-is: `Inner` (a chunk's runs are separator-free token runs), `OpenBracket`, `UnmatchedClose`, `Closing` (over a new `Chunked` stage marker with the same slots as `BracketsMatched`), and `WithSpan` everywhere.
- New types: `Chunks(Vec<WithSpan<Chunk>>)` for one level (the whole literal at the root, a group's interior below), `Chunk(Vec<WithSpan<ChunkItem>>)`, `ChunkItem { Inner, Group(ChunkGroup), StrayClose }`, and `ChunkGroup` — `Bracketed` one stage later: the same opening and closing, children chunked. All derive `ResolvePosition` with `#[resolve_field]`; the macro needs no extension for this.
- How the new paths relate to the old: they are one family. `BracketItemParent` grows a `Chunk` variant (a run's parent is a bracket-tree item slot or a chunk) and gets a name that stops lying, `ItemParent`; `OpenBracketParent` grows a `ChunkGroup` variant; `ResolvedBracketNode` becomes `ResolvedNode` and grows the `Chunks`, `Chunk`, and `ChunkGroup` variants — the one enum that keeps growing toward the full isograph path enum. The cost of sharing is that a parent enum can name a variant no tree of the current stage produces; the alternative is a duplicated type family per stage.
- A stray close rides in its chunk as an item, and an unclosed group keeps its synthetic closing, so the bracket errors stay derivable from the chunked tree the same way `errors()` derives them from the matched-brackets tree.

The pass itself is `chunk(MatchedBrackets<BracketsMatched>) -> Chunks`: one `chunk_items` function per level, applied recursively to group interiors, splitting runs at separator tokens and closing the open chunk at each boundary. The implementation lands with a thorough unit-test suite: the four-chunk example above, separator equivalence and collapsing, `baz watttt` as one chunk, group attachment on both sides of a boundary, nested separators not splitting the outer level, empty groups and empty literals, chunk spans excluding separators, path resolution through the chunk tree (run → chunk → group → chunk → root, opening brackets, separators falling through to the level, stray closes), and error derivation.
