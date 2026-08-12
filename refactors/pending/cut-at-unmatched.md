# cut-at-unmatched: grouping returns the tree beside its errors

A prefactor to the parsing series, replacing demotion in the bracket matcher. An unmatched bracket and everything after it in its level are dropped from the tree; groups fully matched before it survive, and their interiors are treated the same way, recursively. The matcher returns the errors beside the tree, which can no longer represent them:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>)
```

Synthetic closing, which would keep the dropped region by completing the group instead, is a future optimization: unclosed-group-recovery.md.

The cut by example (each with its error vec):

```
foo ( { bar }     ->  foo         [UnmatchedOpen(()]      the ( cuts the root level; the matched {bar} follows it and is dropped
foo { ( }         ->  foo {}      [UnmatchedOpen(()]      the ( cuts the brace's interior; the brace itself matched before nothing
foo { (} )        ->  foo {}      [UnmatchedOpen((), UnmatchedClose())]
foo { bar         ->  foo         [UnmatchedOpen({)]
a ) b             ->  a           [UnmatchedClose())]
a { b { c }       ->  a           [UnmatchedOpen(outer {)]   the } pairs with the nearest open; the outer { never closes
{ foo, bar) }     ->  { foo, bar }  [UnmatchedClose())]      the ) cuts only the interior's tail
```

## Types

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(OpenBracket),
    Close(CloseBracket),
}
```

After (`RawToken` is deleted; `Bracketed` and `BracketError` are unchanged; `MatchedBrackets::errors` is deleted, the vec being returned instead):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
}
```

## The matcher

`parse_items` tracks where its level was cut; from that point it keeps consuming, so enclosing groups still find their closes, but produces nothing and records the errors.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
fn parse_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    errors: &mut Vec<BracketError>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    // The span of this level's first unmatched bracket, when one was met: everything
    // from it on is dropped, though still consumed and diagnosed.
    let mut cut_at: Option<Span> = None;
    while let Some(peek) = tokens.peek() {
        match SplitToken::from(peek.view().item) {
            SplitToken::NonBracket(kind) => {
                let token = peek.commit();
                if cut_at.is_none() {
                    items.push(WithSpan::new(
                        BracketItem::Raw(NonBracketToken(kind)),
                        token.location,
                    ));
                }
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                let token = peek.commit();
                let opening = WithSpan::new(OpenBracket(kind), token.location);
                match parse_bracketed(tokens, enclosing_stack, errors, opening) {
                    ParsedGroup::Closed(group) => {
                        if cut_at.is_none() {
                            let span = Span::join(
                                group.opening.location,
                                group.closing.location,
                            );
                            items.push(WithSpan::new(BracketItem::Bracketed(group), span));
                        }
                    }
                    ParsedGroup::Unclosed => {
                        cut_at.get_or_insert(opening.location);
                    }
                }
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(&kind) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed for its owner; every group between here and the owner
                    // reports unclosed.
                    break;
                }
                let token = peek.commit();
                errors.push(BracketError::UnmatchedClose(WithSpan::new(
                    CloseBracket(kind),
                    token.location,
                )));
                cut_at.get_or_insert(token.location);
            }
        }
    }
    items
}

/// What parsing a group produced. An unclosed group records its error and yields
/// nothing: its opening and everything it would have contained are the cut of the
/// caller's level.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed,
}

fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    errors: &mut Vec<BracketError>,
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    let mut children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack, errors)
    });
    match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            let closing = WithSpan::new(CloseBracket(opening.item.0), token.location);
            let interior = Span::between(opening.location, closing.location);
            strip_captured_line_breaks(&mut children);
            ParsedGroup::Closed(Bracketed {
                opening,
                children: WithSpan::new(MatchedBrackets(children), interior),
                closing,
            })
        }
        _ => {
            errors.push(BracketError::UnmatchedOpen(opening));
            ParsedGroup::Unclosed
        }
    }
}

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last: it
    // classifies a close that does not close the innermost group.
    let mut enclosing_stack = Stack::new();
    let mut errors = Vec::new();
    let mut items = parse_items(&mut tokens, &mut enclosing_stack, &mut errors);
    strip_captured_line_breaks(&mut items);
    errors.sort_by_key(|error| match error {
        BracketError::UnmatchedOpen(open) => open.location.start,
        BracketError::UnmatchedClose(close) => close.location.start,
    });
    (
        WithSpan::new(MatchedBrackets(items), Span::new(0, literal_length)),
        errors,
    )
}
```

The unclosed case fires later than its opening's position, so the errors sort by start before returning. The root's span stays the whole literal; positions in a dropped region resolve to the level that dropped them, the containing-node fallback. Line-break capture is unchanged: a really closed group strips its interior's leading line breaks, the root strips its own, and the demotion carve-out is deleted along with demotion.

## chunk.rs

The chunk tree inherits the clean item type:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}
```

`absorb_chunk`'s content phase loses its `UnmatchedOpen`/`UnmatchedClose` arms; `ChunkedGroup`, `separator_of`, and the boundary phase are untouched. Callers compose with the pair:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
let (tree, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
let chunked = chunk(&tree);
```

## Resolution

A bracket token now only exists as a group's own opening or closing, so `BracketTokenParent` is deleted and both leaves take the group's path directly:

```rust
// from crates/isograph_parser/src/chunk.rs
pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, ChunkedGroupPath<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, ChunkedGroupPath<'a>>;
```

`ChunkedGroup`'s `opening` and `closing` fields respell from `#[resolve_field(parent_variant = Matched)]` to bare `#[resolve_field]`, emitting:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
        if self.opening.location.contains(position) {
            let new_parent = self.path(parent);
            return self.opening.item.resolve(new_parent, position);
        }
```

`IsographResolutionNode`'s variants are unchanged; the path aliases retarget. `ChunkContentItem`'s derive loses its two `parent_variant = Unmatched` payloads with the variants themselves.

## Tests

This is the pass where errors become a returned product, and every test accounts for them: no fixture leaves the vec unasserted.

```rust
// from crates/isograph_parser/src/matched_brackets.rs (test module; chunk.rs builds on it)
    fn tree(literal: &str) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    /// The tree of a fixture that must produce no bracket errors; failing loudly here
    /// keeps a well-formed test from silently exercising a cut tree.
    fn well_formed(literal: &str) -> WithSpan<MatchedBrackets> {
        let (tree, errors) = tree(literal);
        assert_eq!(errors, vec![]);
        tree
    }
```

Well-formed fixtures go through `well_formed` (chunk.rs's `chunked` helper wraps it); error fixtures destructure the pair and assert the exact vec, in source order.

The error-case tests rewrite to the example table above: `a_stray_close_is_a_raw_item_inside_the_brace` becomes the `{ foo, bar) }` interior-tail cut; `an_unclosed_open_is_a_raw_item_inside_the_brace` becomes `foo { ( }` -> `foo {}`; `crossing_junk_leaks_past_the_early_close` becomes `foo { (} )` -> `foo {}` with both errors; `the_close_pairs_with_the_nearest_open` becomes `a { b { c }` -> `a`; `an_extra_close_after_the_balanced_brace_is_raw_at_the_top` and `a_wrong_kind_close_inside_a_matched_pair_is_raw` assert their cuts and vecs the same way; `an_unclosed_open_inside_a_matched_brace_is_the_only_error` becomes `foo { bar(a: }`, whose brace interior keeps `bar` and cuts at the `(`, with `[UnmatchedOpen(()]`. In chunk.rs, `an_unmatched_close_rides_inside_a_chunk_and_errors_stay_on_the_bracket_tree` becomes `a ) b` -> the single chunk `a` plus the error from the pair; `an_unclosed_brace_demotes_to_raw_items_at_the_top` becomes `foo { bar` -> the single chunk `foo`; the two unmatched resolution tests and `a_demoted_opening_captures_nothing` / `a_demoted_brace_leaves_its_line_break_as_a_boundary` are deleted with the states they resolved, replaced by one test that a position inside a dropped region resolves to its containing level. Each rewritten test also asserts spans via `span_of` anchors as today.

## Landing checklist

1. The matcher changes, the chunk.rs and resolution changes, and the test rewrites; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. Every parsing-series doc assumes this world: no unmatched-bracket state exists downstream of the matcher.
