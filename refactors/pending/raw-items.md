# Raw items

A level is a flat sequence of individual items, each raw or grouped: what can be grouped is grouped, and everything else is raw. The bracket tree keeps only matched pairs as structure. A close bracket with no open of its kind is a raw item where it stands. When a group never gets its close, the group is taken apart: its opening becomes a raw item, and its children move into the enclosing level, matched groups among them surviving. Every `Bracketed` has a real opening and a real closing, required fields, and a group's interior is the same type as the root — `WithSpan<MatchedBrackets>` in both positions, the root's span being the whole literal — so no level is special. The chunk-parsing pass reports leftover bracket tokens it finds inside chunks.

With one stage shape left, `TreeContents`, `BracketsMatched`, `Inner`, and `map` have no callers and are deleted; every tree type is concrete.

Two changes: the tree and matcher reshape, then resolution over the new shape.

## Change 1: the tree, the matcher, the errors

### Tree types

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One level: the whole literal at the root, a group's interior below.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedBrackets(pub Vec<WithSpan<BracketItem>>);

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

/// A token that is not part of any structure. Bracket kinds here are the unmatched ones;
/// matched brackets are structure, never raw.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum RawToken {
    NonBracket(NonBracketTokenKind),
    /// An open bracket whose group never closed and was taken apart.
    Open(OpenBracket),
    /// A close bracket no open of its kind was waiting for.
    Close(CloseBracket),
}

/// A matched pair: the opening, the interior level, the closing.
#[derive(Debug, PartialEq, Eq)]
pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    pub children: WithSpan<MatchedBrackets>,
    pub closing: WithSpan<CloseBracket>,
}
```

`OpenBracket(pub BracketKind)` and `CloseBracket(pub BracketKind)` keep their shapes.

Change 1 removes the resolution machinery along with the old tree shape: the landed path family and `ResolvedBracketNode` reference `TreeContents` and the deleted types, so the `ResolvePosition` derives, the parent enums, the path aliases, the resolved enum, and the resolution tests all come out here, and Change 2 rebuilds every one of them over the new tree. Between the two changes the crate parses and reports errors but answers no positions — which is why the derive lines in this change's types are the plain ones.

### The matcher

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
                    BracketItem::Raw(RawToken::NonBracket(kind)),
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

### Errors

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

### The cases, restated as tests

The structural half of the suite, written out in full; resolution assertions move to Change 2.

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

bracket-matching-cases.md is rewritten against these shapes as part of this change: taking unclosed groups apart replaces forcing them shut, "invalid section" becomes "unmatched token", and the end-of-tokens open question closes — content after an unmatched open sits in the enclosing level, so nothing is trapped inside an invalid group while typing.

## Change 2: resolution

The path family, all concrete. A position on whitespace answers the level; a position on an ordinary raw token answers the level too (finer answers are the chunk stage's business); a position on any bracket token answers its bracket leaf, whose parent says matched or raw.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    Bracketed(BracketedPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}

/// The two positions a level can sit in.
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

/// The two positions an open bracket can sit in.
#[derive(Debug)]
pub enum OpenBracketParent<'a> {
    /// A matched group's opening.
    Bracketed(Box<BracketedPath<'a>>),
    /// An unmatched token, in the level it sits in.
    MatchedBrackets(MatchedBracketsPath<'a>),
}

/// The two positions a close bracket can sit in.
#[derive(Debug)]
pub enum CloseBracketParent<'a> {
    /// A matched group's closing.
    Bracketed(Box<BracketedPath<'a>>),
    /// An unmatched token, in the level it sits in.
    MatchedBrackets(MatchedBracketsPath<'a>),
}

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, OpenBracketParent<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, CloseBracketParent<'a>>;
```

`OpenBracket` and `CloseBracket` keep plain derives — their fallbacks answer their own leaves, and both parent variants are constructed outside them:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = OpenBracketParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct OpenBracket(pub BracketKind);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = CloseBracketParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);
```

The tree types' derive sites, in full:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = MatchedBracketsParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct MatchedBrackets(#[resolve_field] pub Vec<WithSpan<BracketItem>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Bracketed {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: WithSpan<MatchedBrackets>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}
```

and their generated impls. The root is entered through the `WithSpan` blanket impl — `tree.resolve(MatchedBracketsParent::Root, position)` on the `WithSpan<MatchedBrackets>` the matcher returned.

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for MatchedBrackets {
    type Parent<'a> = MatchedBracketsParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        for item in self.0.iter() {
            if item.location.contains(position) {
                let new_parent =
                    <BracketItem as ::resolve_position::ResolvePosition>::Parent::from(
                        self.path(parent),
                    );
                return item.item.resolve(new_parent, position);
            }
        }
        return Self::ResolvedNode::MatchedBrackets(self.path(parent).into());
    }
}

impl ::resolve_position::ResolvePosition for BracketItem {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            BracketItem::Raw(inner) => inner.resolve(parent.into(), position),
            BracketItem::Bracketed(inner) => inner.resolve(parent.into(), position),
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
            let new_parent =
                <OpenBracket as ::resolve_position::ResolvePosition>::Parent::from(
                    self.path(parent),
                );
            return self.opening.item.resolve(new_parent, position);
        }
        if self.children.location.contains(position) {
            let new_parent =
                <MatchedBrackets as ::resolve_position::ResolvePosition>::Parent::from(
                    self.path(parent),
                );
            return self.children.item.resolve(new_parent, position);
        }
        if self.closing.location.contains(position) {
            let new_parent =
                <CloseBracket as ::resolve_position::ResolvePosition>::Parent::from(
                    self.path(parent),
                );
            return self.closing.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::Bracketed(self.path(parent).into());
    }
}
```

`BracketItem`'s arms carry the `parent.into()` of the revived parent-conversion doc; both conversions are reflexive there, since `RawToken` and `Bracketed` declare `BracketItemParent` themselves. `RawToken` is where the revived macro capabilities do real work:

- resolve-position-parent-conversion.md: both parent-construction sites go through `From` — delegation arms call `inner.resolve(parent.into(), position)`, field emissions call `Parent::from(self.path(parent))` — so a payload's parent may be its own enum or a plain alias.
- resolve-option-like-enums.md: `#[resolve_into]` variants continue into their payload's resolution, and unmarked variants answer the declared `fallback`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    fallback = MatchedBrackets
)]
pub enum RawToken {
    NonBracket(NonBracketTokenKind),
    #[resolve_into]
    Open(OpenBracket),
    #[resolve_into]
    Close(CloseBracket),
}
```

with the generated impl:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for RawToken {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: Self::Parent<'a>,
        position: ::span::Span,
    ) -> Self::ResolvedNode<'a> {
        match self {
            RawToken::NonBracket(_) => Self::ResolvedNode::MatchedBrackets(parent.into()),
            RawToken::Open(inner) => inner.resolve(parent.into(), position),
            RawToken::Close(inner) => inner.resolve(parent.into(), position),
        }
    }
}
```

and the conversions, written out. The fallback's `parent.into()` is the reflexive `From`, since `BracketItemParent` is the level path itself; the bracket parents convert from both of their positions:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
impl<'a> From<BracketItemParent<'a>> for OpenBracketParent<'a> {
    fn from(level: BracketItemParent<'a>) -> Self {
        OpenBracketParent::MatchedBrackets(level)
    }
}

impl<'a> From<BracketItemParent<'a>> for CloseBracketParent<'a> {
    fn from(level: BracketItemParent<'a>) -> Self {
        CloseBracketParent::MatchedBrackets(level)
    }
}

impl<'a> From<BracketedPath<'a>> for OpenBracketParent<'a> {
    fn from(group: BracketedPath<'a>) -> Self {
        OpenBracketParent::Bracketed(Box::new(group))
    }
}

impl<'a> From<BracketedPath<'a>> for CloseBracketParent<'a> {
    fn from(group: BracketedPath<'a>) -> Self {
        CloseBracketParent::Bracketed(Box::new(group))
    }
}

impl<'a> From<BracketedPath<'a>> for MatchedBracketsParent<'a> {
    fn from(group: BracketedPath<'a>) -> Self {
        MatchedBracketsParent::Bracketed(Box::new(group))
    }
}
```

The resolution tests, added to the same module:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
    #[test]
    fn an_unmatched_open_resolves_with_the_level_as_parent() {
        let text = "foo { ( }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "(")) {
            ResolvedBracketNode::OpenBracket(open) => {
                assert_eq!(open.inner.0, Parenthesis);
                let OpenBracketParent::MatchedBrackets(level) = open.parent else {
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
                let CloseBracketParent::MatchedBrackets(level) = close.parent else {
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
                assert!(matches!(open.parent, OpenBracketParent::Bracketed(_)));
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "}")) {
            ResolvedBracketNode::CloseBracket(close) => {
                assert!(matches!(close.parent, CloseBracketParent::Bracketed(_)));
            }
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn ordinary_tokens_and_whitespace_resolve_to_their_level() {
        let text = "foo { bar }";
        let tree = tree(text);
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "bar")) {
            ResolvedBracketNode::MatchedBrackets(level) => {
                assert!(matches!(
                    level.parent,
                    MatchedBracketsParent::Bracketed(_)
                ));
            }
            node => panic!("expected the level, got {node:?}"),
        }
        match tree.resolve(MatchedBracketsParent::Root, span_of(text, "foo")) {
            ResolvedBracketNode::MatchedBrackets(level) => {
                assert!(matches!(level.parent, MatchedBracketsParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }
```

## Consequences elsewhere

- generic-resolution.md, close-bracket-parent.md, and parent-validity.md move to refactors/past: the trait they generalize and the positions they distinguish no longer exist.
- resolve-position-parent-conversion.md and resolve-option-like-enums.md return to pending, rewritten around `RawToken` as their consumer, and land between the two changes here.
- chunking.md is stale until rewritten: chunking becomes a bespoke recursive pass over levels, producing a v1-shaped chunk tree where a chunk holds its tokens and its trailing group (`bar { baz }` is one chunk), and raw bracket tokens ride inside chunks as content for the chunk parser to report.

## Landing checklist

1. Change 1, with the rewritten structural tests and bracket-matching-cases.md; `cargo test` green.
2. The two macro docs land.
3. Change 2, with the resolution tests; `cargo test` green.
4. Move this doc to refactors/past; rewrite chunking.md against the new tree.
