# Chunking: the pass after bracket matching

The next pass. It regroups each level of the matched-brackets tree into chunks — the grammar-level units of isograph — so that a malformed chunk cannot affect another chunk. In

```
foo { bar(abc) baz(xyz) } qux {}
```

the chunks at the top level are the selection `foo { ... }` and the selection `qux {}`; inside the braces, the selections `bar(abc)` and `baz(xyz)`; inside `bar`'s parens, the argument `abc`. Garbage inside `bar(...)` is `bar`'s problem: `baz(xyz)`, `qux {}`, and everything else parse as if it weren't there.

Chunking is where runs and groups recombine. The bracket pass split each level into `Inner` runs and `Bracketed` groups; a chunk crosses that split — `bar(abc)` is name tokens from a run plus the adjacent paren group — but never crosses a bracket: chunking operates on one level's item sequence at a time, and the bracket pass already guaranteed levels nest correctly.

What a level's items chunk into depends on what the level is, which is the context the walk threads down (the enclosing bracket kind; this is why the stage 4 crossing is a context-carrying recursive walk and not a per-slot map):

- The top level and the inside of `{...}`: selection chunks. A selection is its leading tokens (alias, name, directives) plus an adjacent `(...)` group and an adjacent `{...}` group when present.
- Inside `(...)`: argument chunks — `name: value`, where the value may itself be a group.
- Inside `[...]`: list-element chunks.

Separators split runs into chunk boundaries: line breaks and commas, which is what line breaks became real tokens for. Which separators count at which level, and the exact leading-token grammar of each chunk kind, are the open decisions this doc gets before implementation.

Containment, stated precisely:

- A token-level malformation inside a chunk is that chunk's error and nothing else's. Sibling chunks, parent chunks, and following chunks are unaffected.
- A bracket-level malformation keeps the containment the bracket pass already gave it: an unclosed `(` in `foo { bar(abc baz(xyz) }` forces `bar`'s group shut at the `}` and the following material sits inside it. Those swallowed items are still chunked and analyzed — resilience continues inside an unbalanced group — but they live under it, and only the bracket rules decide that boundary. Chunking never widens or narrows it.

Stage shape: chunking is a `TreeContents` crossing in the established sense — a new stage whose `Inner` is a list of chunk nodes instead of a flat token run — but implemented as the context-threading walk, with its own error tokens for runs that fit no chunk shape (each pass owns its errors). The resolved-node enum grows the chunk leaves when this lands, on the way to the full isograph path enum.

Open, to be decided when this doc is worked:

- The exact chunk grammar per level: what token sequences open a selection chunk, how directives attach, whether a group with no preceding name tokens is its own (malformed) chunk.
- Separator rules per level: where line breaks separate and where they are noise; whether commas and line breaks are interchangeable everywhere.
- Whether chunking and parsing chunk interiors are one pass or two: chunk boundaries are decidable without parsing interiors, so splitting them keeps the containment decision as simple as the bracket pass's — the same argument that separated bracket matching from grammar.
