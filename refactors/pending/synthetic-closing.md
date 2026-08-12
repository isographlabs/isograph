# synthetic-closing: grouping returns the tree beside its errors

A prefactor to the parsing series, replacing demotion in the bracket matcher. Every open bracket forms a group: closed by its real close, or synthetically at its level's end when that close never comes. A close no enclosing group owns is extracted: recorded as an error and given no tree node. The tree therefore cannot represent a bracket error, and the matcher returns the errors beside it:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>)
```

What this buys, downstream: the bracket-item variants disappear from the bracket tree and the chunk tree, so no later stage handles, skips, or stops at an unmatched bracket; the mid-typing literal `field Query.Foo {\n bar\n` parses as a field declaration with the selection `bar` (the unclosed brace is a synthetic group); and `foo( { bar, baz }` keeps its subtree as a synthetically closed argument group. The one report per bracket mistake is the matcher's own, in the returned vec.

## Types

Before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    pub children: WithSpan<MatchedBrackets>,
    pub closing: WithSpan<CloseBracket>,
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(OpenBracket),
    Close(CloseBracket),
}

pub enum BracketError {
    /// An open bracket that sits raw in a level: its group never closed.
    UnmatchedOpen(WithSpan<OpenBracket>),
    /// A close bracket that sits raw in a level: no open of its kind was waiting.
    UnmatchedClose(WithSpan<CloseBracket>),
}
```

After (`RawToken` and `MatchedBrackets::errors` are deleted; `ParsedGroup` and `UnclosedGroup` too, since every group parse now yields a `Bracketed`):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
}

pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's
    /// start, or to the interior's end when the closing is synthetic.
    pub children: WithSpan<MatchedBrackets>,
    /// `None` when the group closed synthetically at its level's end.
    pub closing: Option<WithSpan<CloseBracket>>,
}

/// The matcher's errors, returned beside the tree, in source order. The tree itself
/// cannot represent a bracket error.
pub enum BracketError {
    /// An open bracket whose close never came; its group closed synthetically.
    Unclosed(WithSpan<OpenBracket>),
    /// A close no enclosing group owns; it has no tree node.
    StrayClose(WithSpan<CloseBracket>),
}
```

A synthetic group's item span runs from its opening's start to its interior's end (the opening's end when the interior is empty).

## The matcher

Errors accumulate in a vec threaded through the walk and are sorted by start position before returning, since an `Unclosed` is only known at the level's end, later than its opening's position.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
    // stack exists to classify a close that does not close the innermost group: in
    // `foo { bar ) }`, no enclosing group is a parenthesis, so the `)` is stray, while
    // a brace is on the stack, so the `}` closes the group.
    let mut enclosing_stack = Stack::new();
    let mut errors = Vec::new();
    let mut items = parse_items(&mut tokens, &mut enclosing_stack, &mut errors);
    strip_captured_line_breaks(&mut items);
    errors.sort_by_key(|error| match error {
        BracketError::Unclosed(open) => open.location.start,
        BracketError::StrayClose(close) => close.location.start,
    });
    (
        WithSpan::new(MatchedBrackets(items), Span::new(0, literal_length)),
        errors,
    )
}

fn parse_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    errors: &mut Vec<BracketError>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    while let Some(peek) = tokens.peek() {
        match SplitToken::from(peek.view().item) {
            SplitToken::NonBracket(kind) => {
                let token = peek.commit();
                items.push(WithSpan::new(
                    BracketItem::Raw(NonBracketToken(kind)),
                    token.location,
                ));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                let token = peek.commit();
                let opening = WithSpan::new(OpenBracket(kind), token.location);
                let group = parse_bracketed(tokens, enclosing_stack, errors, opening);
                let end = match &group.closing {
                    Some(closing) => closing.location.end,
                    None => group.children.location.end,
                };
                items.push(WithSpan::new(
                    BracketItem::Bracketed(group),
                    Span::new(opening.location.start, end),
                ));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(&kind) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed, which is what synthetically closes every group
                    // between here and its owner.
                    break;
                }
                let token = peek.commit();
                errors.push(BracketError::StrayClose(WithSpan::new(
                    CloseBracket(kind),
                    token.location,
                )));
            }
        }
    }
    items
}

/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it closes synthetically
/// at its level's end, keeping its children.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    errors: &mut Vec<BracketError>,
    opening: WithSpan<OpenBracket>,
) -> Bracketed {
    let mut children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack, errors)
    });
    strip_captured_line_breaks(&mut children);
    let interior_end = children
        .last()
        .map(|item| item.location.end)
        .unwrap_or(opening.location.end);
    match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            let closing = WithSpan::new(CloseBracket(opening.item.0), token.location);
            let interior = Span::between(opening.location, closing.location);
            Bracketed {
                opening,
                children: WithSpan::new(MatchedBrackets(children), interior),
                closing: Some(closing),
            }
        }
        _ => {
            errors.push(BracketError::Unclosed(opening));
            Bracketed {
                opening,
                children: WithSpan::new(
                    MatchedBrackets(children),
                    Span::new(opening.location.end, interior_end),
                ),
                closing: None,
            }
        }
    }
}
```

Line-break capture simplifies: every group captures, because every opening now leads a group. `strip_captured_line_breaks` runs unconditionally after `parse_items` at both call sites, and the demotion carve-out ("a demoted opening captures nothing") is deleted along with demotion.

## Case behavior

The cases of refactors/past/bracket-matching-cases.md, under the new rule:

- `foo { bar }`: unchanged.
- `foo { bar`: the brace closes synthetically holding `bar`; error `Unclosed({)`. One chunk downstream: `foo {bar}`.
- `foo { ( }`: the paren stops at the `}` the brace owns and closes synthetically, empty; the brace closes really. Errors: `Unclosed(()`.
- `foo { ) }`: the `)` is stray (extracted); the brace closes really, holding nothing. Errors: `StrayClose())`.
- `foo { (} )`: the paren synthetically closes at the `}` (empty), the brace closes really holding the synthetic paren, the trailing `)` is stray at the root. Errors: `Unclosed(()`, `StrayClose())`. The crossing-junk demotion cascade no longer exists.
- `a { b { c }`: the `}` closes the inner brace (nearest open); the outer closes synthetically holding `b` and the inner group. Errors: `Unclosed(` at the outer `{`.
- `( } )`: the `}` is stray inside the parens; the parens close really, empty. Errors: `StrayClose(})`.

## chunk.rs

The chunk tree inherits the clean item type and the optional closing:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}

pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span matches the bracket tree's interior span.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    /// `None` when the group closed synthetically.
    #[resolve_field]
    pub closing: Option<WithSpan<CloseBracket>>,
}
```

`chunk` copies `closing` through as the `Option` it now is; `absorb_chunk`'s content phase loses its `UnmatchedOpen`/`UnmatchedClose` arms; `separator_of` and the boundary phase are untouched. The chunking entry point takes the tree alone; the caller keeps the error vec beside it:

```rust
// from crates/isograph_parser/src/chunk.rs
let (tree, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
let chunked = chunk(&tree);
```

## Resolution

`BracketTokenParent` is deleted: a bracket token can only be a group's own opening or closing now, so both leaf types take the group's path directly, and the `Matched`/`Unmatched` wrapping disappears from the derive sites:

```rust
// from crates/isograph_parser/src/chunk.rs
pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, ChunkedGroupPath<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, ChunkedGroupPath<'a>>;
```

`IsographResolutionNode` keeps its variants; only the path aliases retarget. The changed emission, `ChunkedGroup`'s opening (and closing, identically shaped, through the `Option`):

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
        if self.opening.location.contains(position) {
            let new_parent = self.path(parent);
            return self.opening.item.resolve(new_parent, position);
        }
```

Positions on a stray close resolve to whatever contains them (the chunk, the level), since no node exists for the token; that is the universal fallback, and the stray's error carries its span for diagnostics.

## Tests

This prefactor is the point where the pipeline first returns errors as a product, and every test accounts for them: the error vec is as much the pass's output as the tree, so no fixture leaves it unasserted. The test modules split their helpers accordingly:

```rust
// from crates/isograph_parser/src/matched_brackets.rs (test module; chunk.rs builds on it)
    fn tree(literal: &str) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    /// The tree of a fixture that must produce no bracket errors; failing loudly here
    /// keeps a well-formed test from silently exercising a degraded tree.
    fn well_formed(literal: &str) -> WithSpan<MatchedBrackets> {
        let (tree, errors) = tree(literal);
        assert_eq!(errors, vec![]);
        tree
    }
```

Well-formed fixtures go through `well_formed` (chunk.rs's `chunked` helper wraps it); error fixtures destructure the pair and assert the exact vec, in source order.

matched_brackets.rs tests rewrite per the case list above: `a_stray_close_is_a_raw_item_inside_the_brace`, `an_unclosed_open_is_a_raw_item_inside_the_brace`, `crossing_junk_leaks_past_the_early_close`, `an_extra_close_after_the_balanced_brace_is_raw_at_the_top`, `an_unclosed_open_inside_a_matched_brace_is_the_only_error`, `the_close_pairs_with_the_nearest_open`, and `a_wrong_kind_close_inside_a_matched_pair_is_raw` become assertions on synthetic groups, extracted strays, and the returned error vec (`tree(...)` helpers destructure the pair). `a_demoted_opening_captures_nothing` is deleted with demotion; its input `foo {\n bar` now asserts a synthetic group whose captured line break leaves `bar` as the interior's one chunk. In chunk.rs, `an_unmatched_close_rides_inside_a_chunk_and_errors_stay_on_the_bracket_tree`, `an_unclosed_brace_demotes_to_raw_items_at_the_top`, `an_unmatched_open_resolves_with_the_host_chunk_as_parent`, `an_unmatched_close_at_the_root_resolves_with_the_root_level`, and `a_demoted_brace_leaves_its_line_break_as_a_boundary` rewrite the same way, and the resolution tests' `BracketTokenParent::Matched` matches flatten to the direct group path. Every rewritten test asserts the error vec exactly, in source order.

## Landing checklist

1. The matcher changes, the chunk.rs and resolution changes, and the test rewrites; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past. The parsing series' docs assume this world: no unmatched variants exist anywhere downstream of the matcher.
