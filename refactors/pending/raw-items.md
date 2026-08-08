# Raw items

A level is a flat sequence of individual items, each raw or grouped: what can be grouped is grouped, and everything else is raw. The bracket tree keeps only matched pairs as structure. A close bracket with no open of its kind is a raw item where it stands. When a group never gets its close, the group is taken apart: its opening becomes a raw item, and its children move into the enclosing level, matched groups among them surviving. Every `Bracketed` has a real opening and a real closing, required fields, and a group's interior is the same type as the root — `WithSpan<MatchedBrackets>` in both positions, the root's span being the whole literal — so no level is special. The chunk-parsing pass reports leftover bracket tokens it finds inside chunks.

With one stage shape left, `TreeContents`, `BracketsMatched`, `Inner`, and `map` have no callers and are deleted; every type below is concrete. The shipping order is at the end; everything before it is the finished state.

## The shape

The five tree types, with their finished derives:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One level: the whole literal at the root, a group's interior below.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedBrackets(pub Vec<WithSpan<BracketItem>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

/// A matched pair: the opening, the interior level, the closing.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Bracketed {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field]
    pub children: WithSpan<MatchedBrackets>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}

/// A token that is not part of any structure. Bracket kinds here are the unmatched ones;
/// matched brackets are structure, never raw. Every variant resolves into its token's
/// leaf.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(OpenBracket),
    Close(CloseBracket),
}

/// An ordinary token: the leaf a position on it resolves to.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct NonBracketToken(pub NonBracketTokenKind);

/// A group's opening bracket.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketTokenParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);
```

No macro changes anywhere: where the derive's shape does not fit — `RawToken`'s bracket arms construct a different parent type, and `MatchedBrackets`'s item parent is a path alias with no variant to name — the impl is hand-written, the way iso1 hand-writes `Selection`'s in base_types.rs. `RawToken` and `MatchedBrackets` therefore carry no `ResolvePosition` derive; their impls are under Generated impls, marked as hand-written.

## The result of resolving

A position on whitespace answers the level; a position on any token answers that token's leaf — the ordinary token's own node, or a bracket leaf whose parent says matched or raw.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(BracketedPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}

#[derive(Debug)]
pub enum MatchedBracketsParent<'a> {
    Root,
    Bracketed(Box<BracketedPath<'a>>),
}

pub type MatchedBracketsPath<'a> =
    PositionResolutionPath<&'a MatchedBrackets, MatchedBracketsParent<'a>>;

/// The one place an item can sit: its level, so the parent is the path directly.
pub type BracketItemParent<'a> = MatchedBracketsPath<'a>;

pub type BracketedPath<'a> = PositionResolutionPath<&'a Bracketed, BracketItemParent<'a>>;

/// The one place an ordinary token can sit: its level, so the parent is the path directly.
pub type NonBracketTokenPath<'a> =
    PositionResolutionPath<&'a NonBracketToken, BracketItemParent<'a>>;

/// Shared by `OpenBracket` and `CloseBracket`; the leaf type says which token it is.
#[derive(Debug)]
pub enum BracketTokenParent<'a> {
    Bracketed(Box<BracketedPath<'a>>),
    MatchedBrackets(MatchedBracketsPath<'a>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, BracketTokenParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, BracketTokenParent<'a>>;
```

There are no `From` impls: the derived field emissions name their parent variants as the landed macro always has, and the two hand-written impls construct their parents directly.

## The matcher

The control flow keeps the landed rules — nearest open of the kind, a close owned by an enclosing group ends every group between here and its owner — and a group that never gets its close is taken apart:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The root's span is the whole literal, leading and trailing whitespace included, which
/// the tokens alone do not record; hence the length parameter.
pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> WithSpan<MatchedBrackets> {
    let mut tokens = tokens.into_iter().peekable();
    let mut enclosing = Vec::new();
    WithSpan::new(
        MatchedBrackets(parse_items(&mut tokens, &mut enclosing)),
        Span::new(0, literal_length),
    )
}

/// What parsing a group produced: the group closed for real, or it never got its close,
/// in which case the caller stores the opening as a raw item and the children as its
/// siblings.
enum ParsedGroup {
    Closed(Bracketed),
    Unclosed(UnclosedGroup),
}

/// A group that never got its close: its opening, and the children it had parsed.
struct UnclosedGroup {
    opening: WithSpan<OpenBracket>,
    children: Vec<WithSpan<BracketItem>>,
}

fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem>> {
    let mut items = Vec::new();
    while let Some(&token) = tokens.peek() {
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                tokens.next();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::NonBracket(NonBracketToken(kind))),
                    token.location,
                ));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                tokens.next();
                let opening = WithSpan::new(OpenBracket(kind), token.location);
                match parse_bracketed(tokens, enclosing, opening) {
                    ParsedGroup::Closed(group) => {
                        let span = Span::new(
                            group.opening.location.start,
                            group.closing.location.end,
                        );
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
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // takes apart every group between here and its owner.
                    break;
                }
                tokens.next();
                items.push(WithSpan::new(
                    BracketItem::Raw(RawToken::Close(CloseBracket(kind))),
                    token.location,
                ));
            }
        }
    }
    items
}

/// One group, whose opening the caller already consumed. Its own close closes it; at a
/// close an enclosing group owns, or at the end of the tokens, it never closes.
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
    opening: WithSpan<OpenBracket>,
) -> ParsedGroup {
    enclosing.push(opening.item.0);
    let children = parse_items(tokens, enclosing);
    enclosing.pop();

    match tokens.peek() {
        Some(&token)
            if SplitToken::from(token.item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            tokens.next();
            let closing = WithSpan::new(CloseBracket(opening.item.0), token.location);
            let interior = Span::new(opening.location.end, closing.location.start);
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

`flush_run` and the run merging logic are deleted with `Inner`. The caller stores an unclosed group's pieces with one push and `items.extend`, and inner unclosed groups have already flattened by the time an outer one comes apart.

## Errors

`UnclosedGroup` is deleted; both errors are unmatched tokens, found where they sit:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// An open bracket whose group never closed.
    UnmatchedOpen(WithSpan<OpenBracket>),
    /// A close bracket no open of its kind was waiting for.
    UnmatchedClose(WithSpan<CloseBracket>),
}

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

## Generated impls

The root is entered on the `WithSpan<MatchedBrackets>` the matcher returned, through the `WithSpan` blanket impl: `tree.resolve(MatchedBracketsParent::Root, position)`.

The two hand-written impls, with iso1's `Selection` as the precedent for hand-writing where the derive's shape does not fit:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// Hand-written: an item's parent is the level path itself, and the derive can only
/// construct named variants of a parent enum.
impl ResolvePosition for MatchedBrackets {
    type Parent<'a> = MatchedBracketsParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.0.iter() {
            if item.location.contains(position) {
                return item.item.resolve(self.path(parent), position);
            }
        }
        ResolvedBracketNode::MatchedBrackets(self.path(parent))
    }
}

/// Hand-written: the bracket arms hand their leaves a different parent type than the
/// enum's own, which the derive's delegation cannot express.
impl ResolvePosition for RawToken {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            RawToken::NonBracket(inner) => inner.resolve(parent, position),
            RawToken::Open(inner) => {
                inner.resolve(BracketTokenParent::MatchedBrackets(parent), position)
            }
            RawToken::Close(inner) => {
                inner.resolve(BracketTokenParent::MatchedBrackets(parent), position)
            }
        }
    }
}
```

The derived impls, generated by the landed macro unchanged:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for BracketItem {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            BracketItem::Raw(inner) => inner.resolve(parent, position),
            BracketItem::Bracketed(inner) => inner.resolve(parent, position),
        }
    }
}

impl ::resolve_position::ResolvePosition for Bracketed {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        if self.opening.location.contains(position) {
            let new_parent = <OpenBracket as ::resolve_position::ResolvePosition>::Parent::Bracketed(self.path(parent).into());
            return self.opening.item.resolve(new_parent, position);
        }
        if self.children.location.contains(position) {
            let new_parent = <MatchedBrackets as ::resolve_position::ResolvePosition>::Parent::Bracketed(self.path(parent).into());
            return self.children.item.resolve(new_parent, position);
        }
        if self.closing.location.contains(position) {
            let new_parent = <CloseBracket as ::resolve_position::ResolvePosition>::Parent::Bracketed(self.path(parent).into());
            return self.closing.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::Bracketed(self.path(parent).into());
    }
}

impl ::resolve_position::ResolvePosition for NonBracketToken {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        return Self::ResolvedNode::NonBracketToken(self.path(parent).into());
    }
}
```

`OpenBracket` and `CloseBracket` generate the same leaf shape as `NonBracketToken`, over their own parent enums. Every `.into()` above is the reflexive `From` or the standard boxing `From`; the `Bracketed` field emissions construct the `Bracketed` variant of each leaf's parent enum by name, as the landed macro always has.

## Tests

The structural half of the suite, written out in full:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenize;
    use BracketKind::{Brace, Parenthesis};

    fn tree(literal: &str) -> WithSpan<MatchedBrackets> {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    fn raw(items: &[WithSpan<BracketItem>], index: usize) -> RawToken {
        match &items[index].item {
            BracketItem::Raw(token) => *token,
            item => panic!("expected a raw token at {index}, got {item:?}"),
        }
    }

    fn group(items: &[WithSpan<BracketItem>], index: usize) -> &Bracketed {
        match &items[index].item {
            BracketItem::Bracketed(group) => group,
            item => panic!("expected a group at {index}, got {item:?}"),
        }
    }

    #[test]
    fn text_outside_any_bracket_is_raw_items() {
        let tree = tree("field Query.Foo");
        assert_eq!(tree.item.0.len(), 4);
        for index in 0..4 {
            assert!(matches!(raw(&tree.item.0, index), RawToken::NonBracket(_)));
        }
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn balanced_input_nests_as_typed() {
        let text = "field Query.Foo { bar(arg: [1, 2]) { id } }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 4);
        assert_eq!(brace.opening.item.0, Brace);
        let brace_anchor = span_of(text, "{ bar");
        assert_eq!(
            brace.opening.location,
            Span::new(brace_anchor.start, brace_anchor.start + 1)
        );
        let parenthesis = group(&brace.children.item.0, 1);
        assert_eq!(parenthesis.opening.item.0, Parenthesis);
        let square = group(&parenthesis.children.item.0, 2);
        assert_eq!(square.opening.item.0, BracketKind::Bracket);
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn a_stray_close_is_a_raw_item_inside_the_brace() {
        let text = "foo { ) }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Close(CloseBracket(Parenthesis))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn an_unclosed_open_is_a_raw_item_inside_the_brace() {
        let text = "foo { ( }";
        let tree = tree(text);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn crossing_junk_leaks_past_the_early_close() {
        let text = "foo { (}) }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 4);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        assert_eq!(raw(&tree.item.0, 2), RawToken::Close(CloseBracket(Parenthesis)));
        assert_eq!(raw(&tree.item.0, 3), RawToken::Close(CloseBracket(Brace)));
        match tree.item.errors().as_slice() {
            [
                BracketError::UnmatchedOpen(open),
                BracketError::UnmatchedClose(parenthesis),
                BracketError::UnmatchedClose(brace_close),
            ] => {
                assert_eq!(open.location, span_of(text, "("));
                assert_eq!(parenthesis.location, span_of(text, ")"));
                let tail = span_of(text, ") }");
                assert_eq!(brace_close.location, Span::new(tail.end - 1, tail.end));
            }
            errors => panic!("expected three unmatched brackets, got {errors:?}"),
        }
    }

    #[test]
    fn an_extra_close_after_the_balanced_brace_is_raw_at_the_top() {
        let text = "foo { ( } }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 3);
        let brace = group(&tree.item.0, 1);
        assert_eq!(
            raw(&brace.children.item.0, 0),
            RawToken::Open(OpenBracket(Parenthesis))
        );
        assert_eq!(raw(&tree.item.0, 2), RawToken::Close(CloseBracket(Brace)));
    }

    #[test]
    fn everything_unclosed_at_the_end_of_tokens_comes_apart() {
        let text = "foo { bar(a: }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 7);
        assert_eq!(raw(&tree.item.0, 1), RawToken::Open(OpenBracket(Brace)));
        assert_eq!(raw(&tree.item.0, 3), RawToken::Open(OpenBracket(Parenthesis)));
        assert_eq!(raw(&tree.item.0, 6), RawToken::Close(CloseBracket(Brace)));
        match tree.item.errors().as_slice() {
            [
                BracketError::UnmatchedOpen(brace),
                BracketError::UnmatchedOpen(parenthesis),
                BracketError::UnmatchedClose(close),
            ] => {
                assert_eq!(brace.location, span_of(text, "{"));
                assert_eq!(parenthesis.location, span_of(text, "("));
                assert_eq!(close.location, span_of(text, "}"));
            }
            errors => panic!("expected two opens and the close, got {errors:?}"),
        }
    }

    #[test]
    fn the_close_pairs_with_the_nearest_open() {
        let text = "a { b { c }";
        let tree = tree(text);
        assert_eq!(tree.item.0.len(), 4);
        assert_eq!(raw(&tree.item.0, 1), RawToken::Open(OpenBracket(Brace)));
        let inner = group(&tree.item.0, 3);
        assert!(matches!(
            raw(&inner.children.item.0, 0),
            RawToken::NonBracket(_)
        ));
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                let open_anchor = span_of(text, "{ b");
                assert_eq!(
                    open.location,
                    Span::new(open_anchor.start, open_anchor.start + 1)
                );
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
    }

    #[test]
    fn brackets_inside_strings_are_not_structural() {
        let tree = tree("{ name: \"a}\" }");
        let brace = group(&tree.item.0, 0);
        assert_eq!(brace.children.item.0.len(), 3);
        assert_eq!(tree.item.errors(), vec![]);
    }

    #[test]
    fn a_wrong_kind_close_inside_a_matched_pair_is_raw() {
        let text = "( } )";
        let tree = tree(text);
        let parenthesis = group(&tree.item.0, 0);
        assert_eq!(
            raw(&parenthesis.children.item.0, 0),
            RawToken::Close(CloseBracket(Brace))
        );
        match tree.item.errors().as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, "}"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
    }

    #[test]
    fn spans_cover_the_literal_and_each_interior() {
        let text = "  { a }  ";
        let tree = tree(text);
        assert_eq!(tree.location, Span::from_usize(0, text.len()));
        let brace = group(&tree.item.0, 0);
        assert_eq!(
            brace.children.location,
            Span::new(span_of(text, "{").end, span_of(text, "}").start)
        );
    }
}
```

The resolution tests, in the same module:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    #[test]
    fn an_unmatched_open_resolves_with_the_level_as_parent() {
        let text = "foo { ( }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "(")) {
            ResolvedBracketNode::OpenBracket(open) => {
                assert_eq!(open.inner.0, Parenthesis);
                let BracketTokenParent::MatchedBrackets(level) = open.parent else {
                    panic!("expected the level parent");
                };
                assert!(matches!(
                    level.parent,
                    MatchedBracketsParent::Bracketed(_)
                ));
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_unmatched_close_at_the_root_resolves_with_the_root_level() {
        let text = "a }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "}")) {
            ResolvedBracketNode::CloseBracket(close) => {
                assert_eq!(close.inner.0, Brace);
                let BracketTokenParent::MatchedBrackets(level) = close.parent else {
                    panic!("expected the level parent");
                };
                assert!(matches!(level.parent, MatchedBracketsParent::Root));
            }
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_matched_pair_resolves_with_its_group_as_parent() {
        let text = "foo { bar }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "{")) {
            ResolvedBracketNode::OpenBracket(open) => {
                assert!(matches!(open.parent, BracketTokenParent::Bracketed(_)));
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "}")) {
            ResolvedBracketNode::CloseBracket(close) => {
                assert!(matches!(close.parent, BracketTokenParent::Bracketed(_)));
            }
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_ordinary_token_resolves_to_its_own_leaf() {
        let text = "foo { bar }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "bar")) {
            ResolvedBracketNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                assert!(matches!(
                    token.parent.parent,
                    MatchedBracketsParent::Bracketed(_)
                ));
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_resolves_to_its_level() {
        let text = "foo { bar }";
        let tree = tree(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(MatchedBracketsParent::Root, gap) {
            ResolvedBracketNode::MatchedBrackets(level) => {
                assert!(matches!(level.parent, MatchedBracketsParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }
```

## Shipping order

1. Change 1: the tree, the matcher, and the errors land with the resolution machinery removed — the landed path family and derives reference `TreeContents` and the deleted types, so the parent enums, `ResolvedBracketNode`, the `ResolvePosition` attributes, and the resolution tests come out with them, and the types land with plain derives. The structural tests and the bracket-matching-cases.md rewrite land here: taking unclosed groups apart replaces forcing them shut, "invalid section" becomes "unmatched token", and the end-of-tokens open question closes, since content after an unmatched open sits in the enclosing level. Between the changes the crate parses and reports errors but answers no positions.
2. Change 2: everything in "The shape", "The result of resolving", and "Generated impls" as printed — the derives, the two hand-written impls, and the resolution tests. No macro work exists anywhere in this doc.

## Consequences elsewhere

- generic-resolution.md, close-bracket-parent.md, and parent-validity.md move to refactors/past: the trait they generalize and the positions they distinguish no longer exist.
- Both macro docs sit in refactors/past with no consumer: the two places the derive does not fit are hand-written, per iso1's own practice.
- chunking.md is stale until rewritten: chunking becomes a bespoke recursive pass over levels, producing a v1-shaped chunk tree where a chunk holds its tokens and its trailing group (`bar { baz }` is one chunk), and raw bracket tokens ride inside chunks as content for the chunk parser to report.

## Landing checklist

1. Change 1, with the rewritten structural tests and bracket-matching-cases.md; `cargo test` green.
2. Change 2, with the resolution tests; `cargo test` green.
3. Move this doc to refactors/past; rewrite chunking.md against the new tree.
