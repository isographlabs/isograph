# Root things in isograph

- entrypoint
- field + optional "to X", i.e. pointers are not separate keyword. optional-to.md is the parser change: `target_type: Option<WithSpan<TypeAnnotation>>` on `ClientFieldDeclaration`
- type X, which lets you define a type of data that must be provided by pointers to client types

A field declaration (with or without `to`) may carry a description immediately before its selection set:

```
field <Identifier> . <Identifier> [<paren group>] [to <type>] [<description>] <brace group>
```

`<type>` is a type annotation: `Pet`, `Pet!`, `[Pet]`, `[Pet!]!`, `[[Pet]]`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
```

Origin: `consume_description` in `crates/isograph_parser/src/parse_iso_literal.rs`. Delta: none.

The declaration stores `description: Option<WithSpan<Description>>`. `Description` gains `ResolvePosition` with the declaration as parent, and `IsographResolutionNode` gains `Description`.
