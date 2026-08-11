# Parent-variant attribute

`parent_variant` moves out of `#[resolve_field]` and becomes an attribute of its own. Today one spelling carries two meanings: on a struct field, `#[resolve_field]`'s presence opts the field into resolution, while on an enum payload every variant resolves regardless and the attribute exists only to wrap the parent — so marking presence means "resolution descends here" on structs and something else on enums, and the shared spelling invites the wrong reading (it produced one within a day of one-variant-parent-enums.md landing). Splitting the concerns gives each its own name instead of requiring the marker everywhere:

- `#[resolve_field]`, bare, on struct fields only: resolution descends into this field, and the child's parent is the container's own path. An unmarked struct field is not descended into; a position on it answers the container.
- `#[parent_variant = V]`, on a struct field (alongside `#[resolve_field]`) or on an enum payload: the parent is wrapped in variant `V` of the child's parent enum on the way in.

The full grammar after the split, with every invalid combination a compile error:

- struct field, unmarked: skipped.
- struct field, `#[resolve_field]`: descend, container's path as the parent.
- struct field, `#[resolve_field]` + `#[parent_variant = V]`: descend, container's path wrapped in `V`.
- struct field, `#[parent_variant = V]` alone: error — it wraps a parent for a descent that never happens.
- enum payload, unmarked: delegate, parent passed through unchanged (the enum is exactly its payload and adds no path segment).
- enum payload, `#[parent_variant = V]`: delegate, parent wrapped in `V` as the delegation narrows.
- enum payload, `#[resolve_field]` (with or without `parent_variant` inside or beside it): error — an enum payload always resolves; there is no opt-in, and no container path to grant.
- `#[resolve_field(parent_variant = V)]` no longer parses anywhere; the error message points at the two-attribute spelling.

Behavior is unchanged at every existing site: the generated code is identical, and the test suite passes with no assertion edits. Two files change: `crates/resolve_position_macros/src/lib.rs` (the helper-attribute registration) and `crates/resolve_position_macros/src/resolve_position_macro.rs`, plus the mechanical respelling at whatever derive sites exist when this lands — matched_brackets.rs today, chunk.rs too once chunking.md has landed. This refactor is independent of chunking.md and can be done at any time; chunking.md does not assume it.

## Derive sites

The five `parent_variant` uses in matched_brackets.rs respell; the bare `#[resolve_field]` on `MatchedBrackets` and the unmarked arms of `BracketItem` and `RawToken::NonBracket` are untouched.

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub struct Bracketed {
    #[resolve_field(parent_variant = Matched)]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<MatchedBrackets>,
    #[resolve_field(parent_variant = Matched)]
    pub closing: WithSpan<CloseBracket>,
}

pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(#[resolve_field(parent_variant = Unmatched)] OpenBracket),
    Close(#[resolve_field(parent_variant = Unmatched)] CloseBracket),
}
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub struct Bracketed {
    #[resolve_field]
    #[parent_variant = Matched]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field]
    #[parent_variant = Interior]
    pub children: WithSpan<MatchedBrackets>,
    #[resolve_field]
    #[parent_variant = Matched]
    pub closing: WithSpan<CloseBracket>,
}

pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(#[parent_variant = Unmatched] OpenBracket),
    Close(#[parent_variant = Unmatched] CloseBracket),
}
```

## The macro

The derive registers the new helper attribute:

```rust
// from crates/resolve_position_macros/src/lib.rs
#[proc_macro_derive(
    ResolvePosition,
    attributes(parent_variant, resolve_field, resolve_position, self_type_generics)
)]
```

In resolve_position_macro.rs, `find_resolve_field_attr` generalizes to collect both attributes from a field's or payload's attribute list — at most one of each, duplicates a compile error — and `parse_parent_construction` splits into the two shapes:

- `#[resolve_field]` must be `Meta::Path`; a `Meta::List` or `Meta::NameValue` errors with "expected bare `#[resolve_field]`; parent wrapping is `#[parent_variant = SomeVariant]`".
- `#[parent_variant = V]` must be `Meta::NameValue` whose value is a path; anything else errors with "expected `#[parent_variant = SomeVariant]`".

`ParentConstruction` itself is unchanged — the combinations map onto it exactly as before (`ContainerPath` for `resolve_field` alone, `EnumVariant` when `parent_variant` is present), and the emissions are untouched. The struct path errors when `parent_variant` appears without `resolve_field`; the enum path errors when `resolve_field` appears at all, keeping its current message with the new spelling: "an enum payload always resolves and passes the parent through; annotate only to wrap it: `#[parent_variant = SomeVariant]`".

## Tests

No new behavior: the suite passing with zero assertion edits is the check, since every existing site generates identical code under the new spelling.

## Landing checklist

1. The macro changes and the respelling of every existing derive site; `cargo test -p isograph_parser` and the clippy pre-commit hook pass with no assertion edits.
2. Move this doc to refactors/past. If chunking.md is still pending, respell its snippets to the new grammar.
