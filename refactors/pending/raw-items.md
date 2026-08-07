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

`OpenBracket(pub BracketKind)` and `CloseBracket(pub BracketKind)` are unchanged.

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

The structural half of the suite rewrites to these shapes, with the helper building `match_brackets(tokenize(text), text.len() as u32)` and walking `.item`; resolution assertions move to Change 2.

- `field Query.Foo`: four raw items, no errors.
- `field Query.Foo { bar(arg: [1, 2]) { id } }`: groups nested as typed, every closing real, no errors.
- `foo { ) }`: the brace's interior holds the `)` as `RawToken::Close`; one `UnmatchedClose`.
- `foo { ( }`: the brace closes; its interior holds the `(` as `RawToken::Open`; one `UnmatchedOpen`.
- `foo { (}) }`: the brace's interior holds the raw `(`; the brace closes at the first `}`; `)` and the trailing `}` are raw items at the top level. One `UnmatchedOpen`, two `UnmatchedClose`, in source order.
- `foo { ( } }`: the brace's interior holds the raw `(`; the brace closes; the trailing `}` is a raw item at the top level.
- `foo { bar(a: }`: neither group ever closes, so both come apart at the end of the tokens; the top level is six raw items — `foo`, `{`, `bar`, `(`, `a`, `:` — with two `UnmatchedOpen`.
- `a { b { c }`: the one `}` closes `b`'s group; `a`'s brace never closes and comes apart; the top level is `a`, the raw `{`, `b`, then the group `{ c }`. One `UnmatchedOpen`.
- `{ name: "a}" }`: brackets inside strings stay lexical; no errors.
- `( } )`: the parenthesis pair matches; its interior holds the raw `}`; one `UnmatchedClose`.

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

/// The one place an item can sit: its level.
#[derive(Debug)]
pub enum BracketItemParent<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
}

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

The matched positions construct `Bracketed` variants through `Bracketed`'s field emissions, which name the containing struct. The raw positions flow through `RawToken`, and `RawToken` needs the two parked macro capabilities, which revive with it as their consumer:

- resolve-position-parent-conversion.md: delegation arms call `inner.resolve(parent.into(), position)`, so a marked variant's payload may have its own parent enum one total `From` away.
- resolve-option-like-enums.md: an enum with marked variants delegates those and answers a declared `fallback` for the unmarked rest.

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
    #[resolve_field]
    Open(OpenBracket),
    #[resolve_field]
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

and the three total conversions written beside the parent enums: `From<BracketItemParent> for MatchedBracketsPath` (the fallback's unwrap), `From<BracketItemParent> for OpenBracketParent`, and `From<BracketItemParent> for CloseBracketParent` (each wrapping the level path in its `MatchedBrackets` variant). `BracketItem` itself is all-delegate over `Raw` and `Bracketed`.

Resolution answers, pinned by the rewritten tests:

- `foo { ( }`: the `(` answers `OpenBracket` with `OpenBracketParent::MatchedBrackets(level)`, the level's parent being the brace group; the brace's own `{` and `}` answer their leaves with `Bracketed` parents.
- `a }`: the `}` answers `CloseBracket` with `CloseBracketParent::MatchedBrackets(level)`, the level's parent being `Root`.
- `foo` answers the root level; whitespace inside a group answers the interior level, whose parent is the group.

## Consequences elsewhere

- generic-resolution.md, close-bracket-parent.md, and parent-validity.md move to refactors/past: the trait they generalize and the positions they distinguish no longer exist.
- resolve-position-parent-conversion.md and resolve-option-like-enums.md return to pending, rewritten around `RawToken` as their consumer, and land between the two changes here.
- chunking.md is stale until rewritten: chunking becomes a bespoke recursive pass over levels, producing a v1-shaped chunk tree where a chunk holds its tokens and its trailing group (`bar { baz }` is one chunk), and raw bracket tokens ride inside chunks as content for the chunk parser to report.

## Landing checklist

1. Change 1, with the rewritten structural tests and bracket-matching-cases.md; `cargo test` green.
2. The two macro docs land.
3. Change 2, with the resolution tests; `cargo test` green.
4. Move this doc to refactors/past; rewrite chunking.md against the new tree.
