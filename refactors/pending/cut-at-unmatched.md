# cut-at-unmatched: grouping returns the tree beside its errors

A prefactor to the parsing series, replacing demotion in the bracket matcher. An unmatched bracket and everything after it in its level are dropped from the tree; groups fully matched before it survive, and their interiors are treated the same way, recursively. The matcher returns the errors beside the tree, which can no longer represent them. Synthetic closing, which would keep the dropped region by completing the group instead, is a future optimization: unclosed-group-recovery.md.

The cut by example (each with its error vec):

```
foo ( { bar }     ->  foo           [UnmatchedOpen(()]      the ( cuts the root level; the matched {bar} follows it and is dropped
foo { ( }         ->  foo {}        [UnmatchedOpen(()]      the ( cuts the brace's interior; the brace itself matched
foo { (} )        ->  foo {}        [UnmatchedOpen((), UnmatchedClose())]
foo { bar         ->  foo           [UnmatchedOpen({)]
foo { bar(a: }    ->  foo { bar }   [UnmatchedOpen(()]      the interior keeps bar, cuts at (
a ) b             ->  a             [UnmatchedClose())]
a { b { c }       ->  a             [UnmatchedOpen(outer {)]  the } pairs with the nearest open; the outer { never closes
{ foo, bar) }     ->  { foo, bar }  [UnmatchedClose())]       the ) cuts only the interior's tail
```

The root's span stays the whole literal; positions in a dropped region resolve to the level that dropped them.

## matched_brackets.rs: types

`BracketItem`, before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(OpenBracket),
    Close(CloseBracket),
}
```

After (`RawToken` is deleted):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Raw(NonBracketToken),
    Bracketed(Bracketed),
}
```

`Bracketed` and `BracketError` are unchanged. `MatchedBrackets::errors` and its helper are deleted, the vec being returned instead; before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl MatchedBrackets {
    /// Every unmatched bracket under this level, in source order.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(self, &mut errors);
        errors
    }
}

fn collect_errors(level: &MatchedBrackets, errors: &mut Vec<BracketError>) {
    for item in &level.0 {
        match &item.item {
            BracketItem::Raw(RawToken::NonBracket(_)) => {}
            BracketItem::Raw(RawToken::Open(open)) => {
                errors.push(BracketError::UnmatchedOpen(WithSpan::new(
                    *open,
                    item.location,
                )));
            }
            BracketItem::Raw(RawToken::Close(close)) => {
                errors.push(BracketError::UnmatchedClose(WithSpan::new(
                    *close,
                    item.location,
                )));
            }
            BracketItem::Bracketed(group) => collect_errors(&group.children.item, errors),
        }
    }
}
```

After: both deleted.

`ParsedGroup`, before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// What parsing a group produced: the group closed for real, or it never got its close,
/// in which case the caller stores the opening as a raw item and the children as its
/// siblings.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed(UnclosedGroup),
}

struct UnclosedGroup {
    opening: WithSpan<OpenBracket>,
    children: Vec<WithSpan<BracketItem>>,
}
```

After (`UnclosedGroup` is deleted; an unclosed group yields nothing):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// What parsing a group produced. An unclosed group records its error inside
/// `parse_bracketed` and yields nothing: its opening and everything it would have
/// contained are the cut of the caller's level.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed,
}
```

## matched_brackets.rs: the leaf types

`OpenBracket` (and `CloseBracket`, identically), before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// An opening bracket; its parent says whether it is a group's own opening or
/// unmatched content of a chunk.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct OpenBracket(pub BracketKind);
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// An opening bracket: a group's own opening.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedGroupPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct OpenBracket(pub BracketKind);
```

The crate import in matched_brackets.rs swaps `BracketTokenParent, ChunkPath` for `ChunkedGroupPath, ChunkPath` (`NonBracketToken`'s parent stays `ChunkPath`).

## matched_brackets.rs: the matcher

`match_brackets`, before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> WithSpan<MatchedBrackets> {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
    // stack exists to classify a close that does not close the innermost group: in
    // `foo { bar ) }`, no enclosing group is a parenthesis, so the `)` is a stray raw
    // token, while a brace is on the stack, so the `}` closes the group.
    let mut enclosing_stack = Stack::new();
    let mut items = parse_items(&mut tokens, &mut enclosing_stack);
    strip_captured_line_breaks(&mut items);
    WithSpan::new(MatchedBrackets(items), Span::new(0, literal_length))
}
```

After (the `Unclosed` case fires later than its opening's position, so the errors sort by start before returning):

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    // The kind of every group the level being parsed sits inside, innermost last. The
    // stack exists to classify a close that does not close the innermost group: in
    // `foo { bar ) }`, no enclosing group is a parenthesis, so the `)` is unmatched,
    // while a brace is on the stack, so the `}` closes the group.
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

`parse_items`, before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
fn parse_items(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    while let Some(peek) = tokens.peek() {
        match SplitToken::from(peek.view().item) {
            SplitToken::NonBracket(kind) => {
                let token = peek.commit();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::NonBracket(NonBracketToken(kind))),
                    token.location,
                ));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                let token = peek.commit();
                let opening = WithSpan::new(OpenBracket(kind), token.location);
                match parse_bracketed(tokens, enclosing_stack, opening) {
                    ParsedGroup::Closed(group) => {
                        let span =
                            Span::join(group.opening.location, group.closing.location);
                        items.push(WithSpan::new(BracketItem::Bracketed(group), span));
                    }
                    ParsedGroup::Unclosed(UnclosedGroup { opening, children }) => {
                        items.push(WithSpan::new(
                            BracketItem::Raw(RawToken::Open(opening.item)),
                            opening.location,
                        ));
                        items.extend(children);
                    }
                }
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing_stack.all().contains(&kind) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed, which is what takes apart every group between here and
                    // its owner.
                    break;
                }
                let token = peek.commit();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::Close(CloseBracket(kind))),
                    token.location,
                ));
            }
        }
    }
    items
}
```

After (from the level's first unmatched bracket on, the loop keeps consuming, so enclosing groups still find their closes, but produces nothing):

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
```

`parse_bracketed`, before:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it never closes.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing_stack: &mut Stack<BracketKind>,
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    let mut children = enclosing_stack.with_pushed(opening.item.0, |enclosing_stack| {
        parse_items(tokens, enclosing_stack)
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
        _ => ParsedGroup::Unclosed(UnclosedGroup { opening, children }),
    }
}
```

After:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it never closes, and it
/// yields nothing but its error.
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
```

`strip_captured_line_breaks` and its call sites are unchanged: a really closed group strips its interior's leading line breaks, the root strips its own, and no other case exists once demotion is gone.

## chunk.rs: types

`ChunkContentItem`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// What a chunk holds: every non-separator item of its level, groups included.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    UnmatchedOpen(#[resolve_field(parent_variant = Unmatched)] OpenBracket),
    UnmatchedClose(#[resolve_field(parent_variant = Unmatched)] CloseBracket),
    Group(ChunkedGroup),
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
/// What a chunk holds: every non-separator item of its level, groups included.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}
```

`ChunkedGroup`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedGroup {
    #[resolve_field(parent_variant = Matched)]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field(parent_variant = Matched)]
    pub closing: WithSpan<CloseBracket>,
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field(parent_variant = Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}
```

`BracketTokenParent` and the leaf path aliases, before:

```rust
// from crates/isograph_parser/src/chunk.rs
/// Shared by `OpenBracket` and `CloseBracket`: a bracket token is a group's own
/// opening or closing, or unmatched content of a chunk.
#[derive(Debug)]
pub enum BracketTokenParent<'a> {
    Matched(ChunkedGroupPath<'a>),
    Unmatched(ChunkPath<'a>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a>>;
```

After (the enum is deleted):

```rust
// from crates/isograph_parser/src/chunk.rs
pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, ChunkedGroupPath<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, ChunkedGroupPath<'a>>;
```

## chunk.rs: the pass

`separator_of`, before:

```rust
// from crates/isograph_parser/src/chunk.rs
fn separator_of(item: &WithSpan<BracketItem>) -> Option<SeparatorToken> {
    match &item.item {
        BracketItem::Raw(RawToken::NonBracket(token)) => separator_token(token.0),
        _ => None,
    }
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
fn separator_of(item: &WithSpan<BracketItem>) -> Option<SeparatorToken> {
    match &item.item {
        BracketItem::Raw(token) => separator_token(token.0),
        _ => None,
    }
}
```

`absorb_chunk`'s content phase, before:

```rust
// from crates/isograph_parser/src/chunk.rs
        let content_item = match &peek.view().item {
            BracketItem::Raw(RawToken::NonBracket(token)) => {
                if separator_token(token.0).is_some() {
                    // A separator ends the content phase.
                    break;
                }
                ChunkContentItem::NonBracket(*token)
            }
            BracketItem::Raw(RawToken::Open(open)) => ChunkContentItem::UnmatchedOpen(*open),
            BracketItem::Raw(RawToken::Close(close)) => ChunkContentItem::UnmatchedClose(*close),
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group)),
        };
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
        let content_item = match &peek.view().item {
            BracketItem::Raw(token) => {
                if separator_token(token.0).is_some() {
                    // A separator ends the content phase.
                    break;
                }
                ChunkContentItem::NonBracket(*token)
            }
            BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group)),
        };
```

The boundary phase, `chunk_level`, `chunk_group`, and `chunk` itself are unchanged; callers compose with the pair:

```rust
// from crates/isograph_parser/src/chunk.rs (test module)
let (tree, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
let chunked = chunk(&tree);
```

## Resolution

`IsographResolutionNode` is unchanged: every variant stays, `OpenBracket` and `CloseBracket` included, since matched groups still have both tokens as leaves. What changes is who can be their parent (only a group now) and which positions reach them at all.

The generated impls, for each changed derive site. `OpenBracket` (and `CloseBracket`, identically), before:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for OpenBracket {
    type Parent<'a> = BracketTokenParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::OpenBracket(self.path(parent).into());
    }
}
```

After:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for OpenBracket {
    type Parent<'a> = ChunkedGroupPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::OpenBracket(self.path(parent).into());
    }
}
```

`ChunkContentItem`, before:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkContentItem {
    type Parent<'a> = ChunkPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        match self {
            ChunkContentItem::NonBracket(inner) => inner.resolve(parent, position),
            ChunkContentItem::UnmatchedOpen(inner) => inner.resolve(
                <OpenBracket as ::resolve_position::ResolvePosition>::Parent::Unmatched(parent.into()),
                position,
            ),
            ChunkContentItem::UnmatchedClose(inner) => inner.resolve(
                <CloseBracket as ::resolve_position::ResolvePosition>::Parent::Unmatched(parent.into()),
                position,
            ),
            ChunkContentItem::Group(inner) => inner.resolve(parent, position),
        }
    }
}
```

After:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkContentItem {
    type Parent<'a> = ChunkPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        match self {
            ChunkContentItem::NonBracket(inner) => inner.resolve(parent, position),
            ChunkContentItem::Group(inner) => inner.resolve(parent, position),
        }
    }
}
```

`ChunkedGroup`, before:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkedGroup {
    type Parent<'a> = ChunkPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.opening.location.contains(position) {
            let new_parent = <OpenBracket as ::resolve_position::ResolvePosition>::Parent::Matched(self.path(parent).into());
            return self.opening.item.resolve(new_parent, position);
        }
        if self.children.location.contains(position) {
            let new_parent = <ChunkedLevel as ::resolve_position::ResolvePosition>::Parent::Interior(self.path(parent).into());
            return self.children.item.resolve(new_parent, position);
        }
        if self.closing.location.contains(position) {
            let new_parent = <CloseBracket as ::resolve_position::ResolvePosition>::Parent::Matched(self.path(parent).into());
            return self.closing.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::ChunkedGroup(self.path(parent).into());
    }
}
```

After:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ChunkedGroup {
    type Parent<'a> = ChunkPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.opening.location.contains(position) {
            let new_parent = self.path(parent);
            return self.opening.item.resolve(new_parent, position);
        }
        if self.children.location.contains(position) {
            let new_parent = <ChunkedLevel as ::resolve_position::ResolvePosition>::Parent::Interior(self.path(parent).into());
            return self.children.item.resolve(new_parent, position);
        }
        if self.closing.location.contains(position) {
            let new_parent = self.path(parent);
            return self.closing.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::ChunkedGroup(self.path(parent).into());
    }
}
```

Resolution outcomes for the states that no longer exist:

- A position on a dropped token resolves to the level that dropped it. In `a ) b`, the root level holds the one chunk `a`; positions on `)` and on `b` fall inside no chunk and answer `IsographResolutionNode::ChunkedLevel` with `ChunkedLevelParent::Root`. Previously `)` answered a `CloseBracket` leaf with `BracketTokenParent::Unmatched`.
- A position inside a cut interior resolves to that interior's level. In `foo { ( }`, the brace's interior span still runs from `{`'s end to `}`'s start, its level holds no chunks, and a position on the `(` answers the interior `ChunkedLevel` with `ChunkedLevelParent::Interior`.
- Matched groups are unaffected: `{` and `}` of a real group answer their `OpenBracket`/`CloseBracket` leaves, whose paths now carry `ChunkedGroupPath` directly where they carried `BracketTokenParent::Matched(...)`.

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
