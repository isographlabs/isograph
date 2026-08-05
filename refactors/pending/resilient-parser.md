# Resilient parser: bracket matching and position-based tests

`isograph_parser` parses isograph literals in passes, and every pass is resilient: a malformed region degrades that region, never its siblings and never the file.

1. Extract the isograph literal from a source file. (Not a parsing pass; it hands the passes below a `&str` and the literal's offset in the file.)
2. Tokenize the literal. This landed with the tokenizer copy: `token_kind.rs` (isograph's `IsographLangTokenKind`, verbatim), `tokenize()` producing `Vec<WithSpan<IsographLangTokenKind>>`, and the split layer — `NonBracketTokenKind` (the token enum with the six brackets unrepresentable), `BracketKind`, `BracketToken`, and the total `SplitToken` conversion.
3. Match brackets over the token stream. This pass runs before everything downstream of the tokenizer, and its output is balanced: every group has a close, real or synthesized, so no later pass ever sees an unclosed bracket. A synthetically closed group is an invalid section; everything around it stays valid.
4. Parse each run of non-bracket tokens into the real AST, over the guaranteed-balanced tree. There will be other such passes, each adding structure to what sits between the brackets.

Each pass owns its errors. The bracket matcher reports unexpected closes and unclosed groups; stage 4 produces its own, separate error tokens (the tokenizer's `Error*` kinds already flow through as run tokens), and one input may carry several kinds at once.

This doc specifies the bracket matcher (pass 3) and its tests. Stage 4 gets its own doc once this lands; extraction is scheduled with it. The matching rule's behavior, case by case, with the reasons behind it, the tree each case generates, and the open validity-at-end question, lives in `bracket-matching-cases.md`; this doc implements what that one decides. Everything here is spans relative to the literal, from the `span` crate (`refactors/past/parser-lang-types.md`); no location or file type appears.

The tests need three functions, and this pass is done when they exist and the fixture suite passes:

1. Turn a unique string into a `Span` (test-only, a free function over the original string; it needs no parsed structure).
2. Given a span and the matched-brackets tree, produce a path in the `resolve_position` sense, from which a test asserts facts — above all, whether the position sits inside matched or unmatched brackets.
3. Ask the tree whether the pass produced any errors, such as an unexpected closing bracket.

## Change 1: `MatchedBrackets` and `match_brackets`

New module `crates/isograph_parser/src/matched_brackets.rs`, re-exported from `lib.rs`. Spans are byte offsets into the literal (not the containing file).

The tree is the one `bracket-matching-cases.md` specifies:

```rust
use std::fmt;

use span::{Span, WithSpan};

use crate::{BracketKind, NonBracketTokenKind};

/// The types one matched-brackets tree holds: what a run between brackets is, and what the
/// two bracket errors carry. A pipeline stage is an implementor, and a pass that changes any
/// of these changes all of them at once, through `try_map` (error-refinement.md). The bounds
/// on the slots are what keep the tree types' derives.
pub trait TreeContents {
    /// A maximal run between brackets: lexed tokens at first, parsed nodes later.
    type Run: fmt::Debug + PartialEq + Eq;
    /// What a stray close carries: the bracket kind, or nothing constructible.
    type Stray: fmt::Debug + PartialEq + Eq;
    /// What a synthetic closing carries: unit, or nothing constructible.
    type Unclosed: fmt::Debug + PartialEq + Eq;
}

/// What `match_brackets` produces: runs of lexed tokens, both bracket errors representable.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketsMatched;

impl TreeContents for BracketsMatched {
    type Run = Vec<WithSpan<NonBracketTokenKind>>;
    type Stray = BracketKind;
    type Unclosed = ();
}

/// One isograph literal with its brackets matched. Spans live on the `WithSpan` wrapping
/// each item.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedBrackets<TContents: TreeContents>(pub Vec<WithSpan<BracketItem<TContents>>>);

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem<TContents: TreeContents> {
    /// A maximal run containing no brackets. Its span runs from its first token's start to
    /// its last token's end, whitespace between them included.
    Run(TContents::Run),
    Bracketed(Bracketed<TContents>),
    /// A close bracket no open of its kind was waiting for: an invalid section one token
    /// wide.
    StrayClose(TContents::Stray),
}

/// An open bracket, everything up to its close, and the close — always present, so every pass
/// after this one works with guaranteed matching brackets. The wrapping `WithSpan`'s span runs
/// from the start of the opening to the end of a real closing, or to the end of the last
/// child when the closing is synthetic.
#[derive(Debug, PartialEq, Eq)]
pub struct Bracketed<TContents: TreeContents> {
    pub opening: WithSpan<BracketKind>,
    pub closing: Closing<TContents>,
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Closing<TContents: TreeContents> {
    /// The close bracket the author typed.
    Real(Span),
    /// The group never got its close and was forced to end: at the close bracket an
    /// enclosing group owns, or at the end of the tokens. Where it ended is the end of the
    /// wrapping `WithSpan`'s span; the missing close has no span of its own. A group closed
    /// this way is an invalid section.
    Synthetic(TContents::Unclosed),
}
```

### The matching rule

- An open bracket begins a group; its children are parsed until the tokens end or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: `Closing::Real`. Otherwise the group is `Closing::Synthetic`, forced to end the moment the close it cannot match, or the end of the tokens, is encountered; it ends where its last child does, or at its opening when it has no children. Groups between a close and the group that owns it all end this way, innermost first, which is the nesting `bracket-matching-cases.md` shows.
- A close bracket that no group in the enclosing stack owns is a `StrayClose` where it stands. It consumes nothing: an open of a different kind stays open and may still match later.

### The errors

Derived from the tree rather than accumulated beside it, so there is one source of truth. `errors()` exists for every stage that still represents bracket errors — the `where` bound below states exactly that condition; a refined stage (error-refinement.md) has nothing for it to find and does not carry it:

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<BracketKind>),
    /// A group whose close was synthesized.
    Unclosed(WithSpan<UnclosedGroup>),
}

/// The wrapping `WithSpan`'s span is the whole group; its end is where the close should have
/// been.
#[derive(Debug, PartialEq, Eq)]
pub struct UnclosedGroup {
    pub opening: WithSpan<BracketKind>,
}

impl<TContents> MatchedBrackets<TContents>
where
    TContents: TreeContents<Stray = BracketKind, Unclosed = ()>,
{
    /// Every error the pass produced, in source order of the position each error starts at.
    /// Empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(&self.0, &mut errors);
        errors
    }
}

fn collect_errors<TContents>(
    items: &[WithSpan<BracketItem<TContents>>],
    errors: &mut Vec<BracketError>,
) where
    TContents: TreeContents<Stray = BracketKind, Unclosed = ()>,
{
    for item in items {
        match &item.item {
            BracketItem::Run(_) => {}
            BracketItem::StrayClose(kind) => {
                errors.push(BracketError::UnexpectedClose(WithSpan::new(*kind, item.span)));
            }
            BracketItem::Bracketed(bracketed) => {
                if matches!(bracketed.closing, Closing::Synthetic(())) {
                    errors.push(BracketError::Unclosed(WithSpan::new(
                        UnclosedGroup {
                            opening: bracketed.opening,
                        },
                        item.span,
                    )));
                }
                collect_errors(&bracketed.children, errors);
            }
        }
    }
}
```

(A group's opening precedes its children, so pushing a group's `Unclosed` before descending is source order.)

### Implementation

A recursive descent over the token stream with one token of lookahead — `Peekable` over the tokenizer's output, the same discipline as the existing parser's `PeekableLexer`. The `enclosing` vector is context about brackets already consumed, not lookahead: the parser never sees past the one peeked token. `SplitToken` is the only place bracket-ness is decided, so the matcher and the runs cannot disagree about what counts as a bracket.

```rust
use std::iter::Peekable;

use crate::{BracketToken, IsographLangTokenKind, SplitToken};

type TokenStream = Peekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = tokens.into_iter().peekable();
    let mut enclosing = Vec::new();
    let items = parse_items(&mut tokens, &mut enclosing);
    MatchedBrackets(items)
}

/// Parse items until a close bracket some enclosing group owns, or the end of the tokens.
///
/// `enclosing` is the kind of every group this level sits inside, innermost last, the group
/// being parsed included; it is how a close bracket with no open of its kind anywhere is
/// recognized as stray rather than left to end this level.
fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem<BracketsMatched>>> {
    let mut items = Vec::new();
    let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    while let Some(&token) = tokens.peek() {
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                tokens.next();
                run.push(WithSpan::new(kind, token.span));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(parse_bracketed(tokens, enclosing, WithSpan::new(kind, token.span)));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // synthetically closes every group between here and its owner.
                    break;
                }
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(WithSpan::new(BracketItem::StrayClose(kind), token.span));
            }
        }
    }
    flush_run(&mut items, &mut run);
    items
}

/// End the run in progress, if any: one `Run` item spanning its first token's start to its
/// last token's end.
fn flush_run(
    items: &mut Vec<WithSpan<BracketItem<BracketsMatched>>>,
    run: &mut Vec<WithSpan<NonBracketTokenKind>>,
) {
    let span = match (run.first(), run.last()) {
        (Some(first), Some(last)) => Span::join(first.span, last.span),
        _ => return,
    };
    items.push(WithSpan::new(BracketItem::Run(std::mem::take(run)), span));
}

/// One group, its opening already consumed: parse children, then look at the one token that
/// stopped them — this group's own close (consumed, `Closing::Real`) or something an
/// enclosing group owns (left alone, and this group is closed synthetically where it stands).
fn parse_bracketed(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
    opening: WithSpan<BracketKind>,
) -> WithSpan<BracketItem<BracketsMatched>> {
    enclosing.push(opening.item);
    let children = parse_items(tokens, enclosing);
    enclosing.pop();

    let (closing, end) = match tokens.peek() {
        Some(&token)
            if SplitToken::from(token.item)
                == SplitToken::Bracket(BracketToken::Close(opening.item)) =>
        {
            tokens.next();
            (Closing::Real(token.span), token.span.end)
        }
        // The group was forced to end: at a close an enclosing group owns, or at the end of
        // the tokens. It ends where its last child does.
        _ => (
            Closing::Synthetic(()),
            children.last().map_or(opening.span.end, |last| last.span.end),
        ),
    };

    let span = Span::new(opening.span.start, end);
    WithSpan::new(
        BracketItem::Bracketed(Bracketed {
            opening,
            closing,
            children,
        }),
        span,
    )
}
```

At the top level `enclosing` is empty, so `parse_items` never breaks there: every close bracket the recursion hands back up is consumed as a stray or by the group that owns it, and `match_brackets` consumes every token. Synthetically closed groups nest correctly because each recursion level returns without consuming the close that stopped it: closing `}` against open `{`, `(`, `[` returns out of the `[` group's level, then the `(` group's, ending each at its own last child, before the `{` group consumes the `}` as its own.

## Change 2: position resolution and validity

New module `crates/isograph_parser/src/resolve_matched_brackets.rs`, implementing `resolve_position::ResolvePosition` for the matched-brackets tree, the way `resolve_position`'s own doc comment describes.

A path points at its variant's payload — the `TContents::Run` of a run, the `Bracketed<TContents>`, a stray's `TContents::Stray` — plus the parent chain. The resolved item's span lives on the `WithSpan` wrapping it in the tree, and the caller of `resolve` already holds the position it asked about. A position the tokenizer skipped (whitespace between items) resolves to the enclosing group, which is the leaf that contains it.

```rust
use span::{Span, WithSpan};
use resolve_position::{PositionResolutionPath, ResolvePosition};

#[derive(Debug)]
pub enum ResolvedBracketNode<'a, TContents: TreeContents> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Run(RunPath<'a, TContents>),
    Bracketed(BracketedPath<'a, TContents>),
    StrayClose(StrayClosePath<'a, TContents>),
}

pub type MatchedBracketsPath<'a, TContents> =
    PositionResolutionPath<&'a MatchedBrackets<TContents>, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a, TContents: TreeContents> {
    MatchedBrackets(MatchedBracketsPath<'a, TContents>),
    Bracketed(Box<BracketedPath<'a, TContents>>),
}

pub type RunPath<'a, TContents: TreeContents> =
    PositionResolutionPath<&'a TContents::Run, BracketItemParent<'a, TContents>>;
pub type BracketedPath<'a, TContents: TreeContents> =
    PositionResolutionPath<&'a Bracketed<TContents>, BracketItemParent<'a, TContents>>;
pub type StrayClosePath<'a, TContents: TreeContents> =
    PositionResolutionPath<&'a TContents::Stray, BracketItemParent<'a, TContents>>;
```

Each impl finds the child containing the position and delegates; a node none of whose children contain the position is the leaf. The parent path is built once, in the branch that uses it:

```rust
impl<TContents: TreeContents> ResolvePosition for MatchedBrackets<TContents> {
    type Parent<'a>
        = ()
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(&'a self, parent: (), position: Span) -> ResolvedBracketNode<'a, TContents> {
        match containing_child(&self.0, position) {
            Some(child) => {
                let parent = BracketItemParent::MatchedBrackets(self.path(parent));
                resolve_child(child, parent, position)
            }
            None => ResolvedBracketNode::MatchedBrackets(self.path(parent)),
        }
    }
}

impl<TContents: TreeContents> ResolvePosition for Bracketed<TContents> {
    type Parent<'a>
        = BracketItemParent<'a, TContents>
    where
        Self: 'a;
    type ResolvedNode<'a>
        = ResolvedBracketNode<'a, TContents>
    where
        Self: 'a;

    fn resolve<'a>(
        &'a self,
        parent: BracketItemParent<'a, TContents>,
        position: Span,
    ) -> ResolvedBracketNode<'a, TContents> {
        match containing_child(&self.children, position) {
            Some(child) => {
                let parent = BracketItemParent::Bracketed(Box::new(self.path(parent)));
                resolve_child(child, parent, position)
            }
            None => ResolvedBracketNode::Bracketed(self.path(parent)),
        }
    }
}

/// The first item whose span contains the position.
fn containing_child<'a, TContents: TreeContents>(
    items: &'a [WithSpan<BracketItem<TContents>>],
    position: Span,
) -> Option<&'a WithSpan<BracketItem<TContents>>> {
    items.iter().find(|item| item.span.contains(position))
}

/// Resolve into an item already known to contain the position.
fn resolve_child<'a, TContents: TreeContents>(
    child: &'a WithSpan<BracketItem<TContents>>,
    parent: BracketItemParent<'a, TContents>,
    position: Span,
) -> ResolvedBracketNode<'a, TContents> {
    match &child.item {
        BracketItem::Run(run) => ResolvedBracketNode::Run(PositionResolutionPath {
            inner: run,
            parent,
        }),
        BracketItem::Bracketed(bracketed) => bracketed.resolve(parent, position),
        BracketItem::StrayClose(stray) => ResolvedBracketNode::StrayClose(PositionResolutionPath {
            inner: stray,
            parent,
        }),
    }
}
```

Validity is a fact about the whole path, not the leaf: a position is in a valid section iff neither its node nor any ancestor is a `StrayClose` or a synthetically closed group. Invalidity never spreads outward — not to siblings, not to the enclosing really-closed group.

```rust
#[derive(Debug)]
pub enum SectionValidity {
    Valid,
    Invalid,
}

impl<TContents: TreeContents> ResolvedBracketNode<'_, TContents> {
    pub fn validity(&self) -> SectionValidity {
        match self {
            ResolvedBracketNode::StrayClose(_) => SectionValidity::Invalid,
            ResolvedBracketNode::MatchedBrackets(_) => SectionValidity::Valid,
            ResolvedBracketNode::Run(path) => path.parent.validity(),
            ResolvedBracketNode::Bracketed(path) => match path.inner.closing {
                Closing::Real(_) => path.parent.validity(),
                // Provisional: whether a group synthetically closed at the end of the
                // tokens counts as invalid is the open question in
                // bracket-matching-cases.md.
                Closing::Synthetic(_) => SectionValidity::Invalid,
            },
        }
    }
}

impl<TContents: TreeContents> BracketItemParent<'_, TContents> {
    fn validity(&self) -> SectionValidity {
        match self {
            BracketItemParent::MatchedBrackets(_) => SectionValidity::Valid,
            BracketItemParent::Bracketed(path) => match path.inner.closing {
                Closing::Real(_) => path.parent.validity(),
                Closing::Synthetic(_) => SectionValidity::Invalid,
            },
        }
    }
}
```

A position inside a synthetically closed group is `Invalid` however deep it sits: the `Run` and really-closed `Bracketed` arms keep walking up, and the walk stops at the first `Closing::Synthetic` ancestor.

## Change 3: the test harness

`crates/tests` holds the harness in `src/lib.rs` and the tests in `tests/bracket_matching.rs`. Fixtures are files under `crates/tests/fixtures/`, extension `.iso`, containing literal text only (no `iso(...)` wrapper). A fixture is an input, never an expected output.

`crates/tests/Cargo.toml`:

```toml
[dependencies]
isograph_parser = { path = "../isograph_parser" }
resolve_position = { path = "../resolve_position" }
span = { path = "../span" }
```

`src/lib.rs`:

```rust
use std::path::Path;

use isograph_parser::{match_brackets, tokenize, BracketsMatched, MatchedBrackets, ResolvedBracketNode};
use resolve_position::ResolvePosition;
use span::Span;

/// The span of `pattern`, which must occur exactly once in `text`. The preferred way for a
/// test to point at a position: it needs nothing but the original string, editing the fixture
/// cannot silently shift what the test asserts about, and a pattern that stops being unique
/// fails loudly instead.
pub fn span_of(text: &str, pattern: &str) -> Span {
    let mut occurrences = text.match_indices(pattern);
    let (offset, _) = occurrences
        .next()
        .expect("the pattern the test anchors on occurs in the fixture");
    assert!(
        occurrences.next().is_none(),
        "the pattern the test anchors on occurs exactly once in the fixture"
    );
    Span::from_usize(offset, offset + pattern.len())
}

pub struct Fixture {
    pub text: String,
    pub tree: MatchedBrackets<BracketsMatched>,
}

impl Fixture {
    pub fn load(name: &str) -> Fixture {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(format!("{name}.iso"));
        let text = std::fs::read_to_string(&path)
            .expect("the fixture named by the test exists under crates/tests/fixtures");
        let tree = match_brackets(tokenize(&text));
        Fixture { text, tree }
    }

    /// The path to the node containing `span`.
    pub fn resolve(&self, span: Span) -> ResolvedBracketNode<'_, BracketsMatched> {
        self.tree.resolve((), span)
    }

    /// `resolve` at the unique occurrence of `pattern` in the fixture's text.
    pub fn on(&self, pattern: &str) -> ResolvedBracketNode<'_, BracketsMatched> {
        self.resolve(span_of(&self.text, pattern))
    }

    /// The node at a 0-indexed line and character; the character indexes bytes in the
    /// line. For positions no distinctive text names.
    pub fn at(&self, line: u32, character: u32) -> ResolvedBracketNode<'_, BracketsMatched> {
        let offset = self.offset(line, character);
        self.resolve(Span::new(offset, offset + 1))
    }

    fn offset(&self, line: u32, character: u32) -> u32 {
        let line_start: usize = self
            .text
            .split_inclusive('\n')
            .take(line as usize)
            .map(str::len)
            .sum();
        line_start as u32 + character
    }
}
```

`on` resolves the whole pattern's span, so the node it lands on must contain every byte of the pattern: `on("broken")` sits in the run before the `(`, `on("(")` sits on the group whose opening that is, and a pattern spanning two siblings resolves to their common ancestor, which is itself a fact a test may assert.

A test names a fixture, points at positions, and asserts facts:

`fixtures/unclosed_paren.iso`:

```
first {
  broken (
}
second {
  ok
}
```

`tests/bracket_matching.rs`:

```rust
use isograph_parser::{BracketError, Closing, ResolvedBracketNode, SectionValidity};
use tests::{Fixture, span_of};

#[test]
fn unclosed_paren_is_an_invalid_section() {
    let fixture = Fixture::load("unclosed_paren");
    // The `(` that the `}` refuses to close; it is the fixture's only paren.
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Invalid));
}

#[test]
fn the_enclosing_brace_group_stays_valid() {
    let fixture = Fixture::load("unclosed_paren");
    // The run inside `first { ... }`, before the invalid paren section begins.
    assert!(matches!(fixture.on("broken").validity(), SectionValidity::Valid));
    // Inside `second { ok }`, after the broken section.
    assert!(matches!(fixture.on("ok").validity(), SectionValidity::Valid));
}

#[test]
fn the_unclosed_group_is_the_leaf_it_resolves_to() {
    let fixture = Fixture::load("unclosed_paren");
    match fixture.on("(") {
        ResolvedBracketNode::Bracketed(path) => {
            assert!(matches!(path.inner.closing, Closing::Synthetic(())));
        }
        node => panic!("expected the unclosed paren group, got {node:?}"),
    }
}

#[test]
fn the_unclosed_paren_is_the_only_error() {
    let fixture = Fixture::load("unclosed_paren");
    let errors = fixture.tree.errors();
    match errors.as_slice() {
        [BracketError::Unclosed(unclosed)] => {
            assert_eq!(unclosed.item.opening.span, span_of(&fixture.text, "("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}
```

The initial fixture set follows `bracket-matching-cases.md`, one fixture per case there, each with the assertions its case states — validity at the marked positions, and the exact error list, which is empty for the two well-formed cases.
