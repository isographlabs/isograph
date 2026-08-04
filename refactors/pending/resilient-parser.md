# Resilient parser: bracket matching and position-based tests

`isograph_parser` parses isograph literals in passes, and every pass is resilient: a malformed region degrades that region, never its siblings and never the file.

1. Extract the isograph literal from a source file. (Not a parsing pass; it hands the passes below a `&str` and the literal's offset in the file.)
2. Match brackets — `()`, `{}`, `[]` — within the literal. This pass runs first, always, and its output is balanced: every group has a close, real or synthesized, so no later pass ever sees an unclosed bracket. A synthetically closed group is an invalid section; everything around it stays valid.
3. Parse each group's contents into the real AST, over the guaranteed-balanced tree. There will be other such passes.

Each pass owns its errors. This pass reports unexpected closes and unclosed groups; stage 3 produces its own, separate error tokens, and one input may carry both kinds at once.

This doc specifies stage 2 and its tests. Stage 3 gets its own doc once stage 2 lands; extraction is scheduled with it. The changes here land after `parser-lang-types.md`, whose `parser_lang_types` crate supplies `Span` and `WithSpan`. The matching rule's behavior, case by case, with the reasons behind it, the tree each case generates, and the open validity-at-end-of-literal question, lives in `bracket-matching-cases.md`; this doc implements what that one decides. Everything here is spans relative to the literal; no location or file type appears.

The tests need three functions, and stage 2 is done when they exist and the fixture suite passes:

1. Turn a unique string in a fixture into a `Span` (test-only, on the harness).
2. Given a span and the bracket tree, produce a path in the `resolve_position` sense, from which a test asserts facts — above all, whether the position sits inside matched or unmatched brackets.
3. Ask the tree whether the pass produced any errors, such as an unexpected closing bracket.

## Change 1: the bracket tree and `match_brackets`

New module `crates/isograph_parser/src/bracket_tree.rs`, re-exported from `lib.rs`. Spans are byte offsets into the literal (not the containing file).

The tree is the one `bracket-matching-cases.md` specifies:

```rust
use parser_lang_types::{Span, WithSpan};

/// One isograph literal with its brackets matched. Content between brackets is uninterpreted
/// at this stage.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketTree {
    pub items: Vec<BracketItem>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Text(WithSpan<Text>),
    Bracketed(WithSpan<Bracketed>),
    /// A close bracket no open of its kind was waiting for: an invalid section one character
    /// wide.
    StrayClose(WithSpan<Bracket>),
}

/// A maximal run of text containing no brackets. Its span lives on the enclosing `WithSpan`.
#[derive(Debug, PartialEq, Eq)]
pub struct Text;

/// An open bracket, everything up to its close, and the close — always present, so every pass
/// after this one works with guaranteed matching brackets. The enclosing `WithSpan`'s span
/// runs from the start of the opening to the end of the closing.
#[derive(Debug, PartialEq, Eq)]
pub struct Bracketed {
    pub opening: WithSpan<Bracket>,
    pub closing: Closing,
    pub children: Vec<BracketItem>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum Closing {
    /// The close bracket the author typed.
    Real(Span),
    /// A zero-width close at the position the group was forced to end: just before the close
    /// bracket an enclosing group owns, or at the end of the literal. What makes the group an
    /// invalid section.
    Synthetic(Span),
}

impl Closing {
    pub fn span(&self) -> Span {
        match self {
            Closing::Real(span) | Closing::Synthetic(span) => *span,
        }
    }
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum Bracket {
    Paren,
    Curly,
    Square,
}

/// The span an item covers, wherever its variant keeps it.
pub fn item_span(item: &BracketItem) -> Span {
    match item {
        BracketItem::Text(text) => text.span,
        BracketItem::Bracketed(bracketed) => bracketed.span,
        BracketItem::StrayClose(close) => close.span,
    }
}
```

A synthetic close's span is zero-width at a real offset the scanner reached, built with `Span::new`; `parser_lang_types` needs no generated-span constructor for it.

Every `(`, `)`, `{`, `}`, `[`, and `]` in the literal is structural at this stage.

### The matching rule

- An open bracket begins a group; its children are parsed until the literal ends or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: `Closing::Real`. Otherwise the group is closed synthetically the moment the close it cannot match, or the end of the literal, is encountered: `Closing::Synthetic`, zero-width at that position. Groups between a close and the group that owns it all end this way, innermost first, which is the nesting `bracket-matching-cases.md` shows.
- A close bracket that no group in the enclosing stack owns is a `StrayClose` where it stands. It consumes nothing: an open of a different kind stays open and may still match later.

### The errors

Derived from the tree rather than accumulated beside it, so there is one source of truth:

```rust
#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<Bracket>),
    /// A group whose close was synthesized.
    Unclosed(UnclosedGroup),
}

#[derive(Debug, PartialEq, Eq)]
pub struct UnclosedGroup {
    pub opening: WithSpan<Bracket>,
    /// The zero-width close synthesized where the group was forced to end.
    pub synthetic_close: Span,
}

impl BracketTree {
    /// Every error the pass produced, in source order of the position each error starts at.
    /// Empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError> {
        let mut errors = Vec::new();
        collect_errors(&self.items, &mut errors);
        errors
    }
}

fn collect_errors(items: &[BracketItem], errors: &mut Vec<BracketError>) {
    for item in items {
        match item {
            BracketItem::Text(_) => {}
            BracketItem::StrayClose(close) => {
                errors.push(BracketError::UnexpectedClose(*close));
            }
            BracketItem::Bracketed(bracketed) => {
                if let Closing::Synthetic(synthetic_close) = bracketed.item.closing {
                    errors.push(BracketError::Unclosed(UnclosedGroup {
                        opening: bracketed.item.opening,
                        synthetic_close,
                    }));
                }
                collect_errors(&bracketed.item.children, errors);
            }
        }
    }
}
```

(A group's opening precedes its children, so pushing a group's `Unclosed` before descending is source order.)

### Implementation

A scanner with one token of lookahead and a recursive descent over it — the same shape as the existing parser's `PeekableLexer`. The `enclosing` vector is context about brackets already consumed, not lookahead: the parser never sees past the one peeked token.

```rust
pub fn match_brackets(literal: &str) -> BracketTree {
    let mut lexer = BracketLexer::new(literal);
    let mut enclosing = Vec::new();
    let items = parse_items(&mut lexer, &mut enclosing);
    BracketTree { items }
}

/// What the scanner hands out: one bracket, or the maximal run between brackets.
#[derive(Debug, PartialEq, Eq, Copy, Clone)]
enum BracketToken {
    Text,
    Open(Bracket),
    Close(Bracket),
}

fn bracket_token(byte: u8) -> Option<BracketToken> {
    Some(match byte {
        b'(' => BracketToken::Open(Bracket::Paren),
        b')' => BracketToken::Close(Bracket::Paren),
        b'{' => BracketToken::Open(Bracket::Curly),
        b'}' => BracketToken::Close(Bracket::Curly),
        b'[' => BracketToken::Open(Bracket::Square),
        b']' => BracketToken::Close(Bracket::Square),
        _ => None,
    })
}

/// The scanner, holding exactly one token of lookahead.
struct BracketLexer<'a> {
    bytes: &'a [u8],
    /// The one token of lookahead; `None` once the literal is exhausted.
    peeked: Option<WithSpan<BracketToken>>,
    /// Where the token after `peeked` starts.
    offset: usize,
}

impl<'a> BracketLexer<'a> {
    fn new(literal: &'a str) -> Self {
        let mut lexer = BracketLexer {
            bytes: literal.as_bytes(),
            peeked: None,
            offset: 0,
        };
        lexer.peeked = lexer.lex();
        lexer
    }

    /// The next token without consuming it: the whole of the parser's lookahead.
    fn peek(&self) -> Option<WithSpan<BracketToken>> {
        self.peeked
    }

    /// Consume the peeked token; `peek` then sees the one after it.
    fn advance(&mut self) {
        self.peeked = self.lex();
    }

    /// The zero-width span at the end of the literal.
    fn end_of_literal(&self) -> Span {
        Span::from_usize(self.bytes.len(), self.bytes.len())
    }

    /// The token starting at `offset`: one bracket, or the maximal bracket-free run.
    fn lex(&mut self) -> Option<WithSpan<BracketToken>> {
        let start = self.offset;
        let first = *self.bytes.get(start)?;
        if let Some(token) = bracket_token(first) {
            self.offset = start + 1;
            return Some(WithSpan::new(token, Span::from_usize(start, start + 1)));
        }
        let mut end = start + 1;
        while end < self.bytes.len() && bracket_token(self.bytes[end]).is_none() {
            end += 1;
        }
        self.offset = end;
        Some(WithSpan::new(BracketToken::Text, Span::from_usize(start, end)))
    }
}

/// Parse items until a close bracket some enclosing group owns, or the end of the literal.
///
/// `enclosing` is the kind of every group this level sits inside, innermost last, the group
/// being parsed included; it is how a close bracket with no open of its kind anywhere is
/// recognized as stray rather than left to end this level.
fn parse_items(lexer: &mut BracketLexer<'_>, enclosing: &mut Vec<Bracket>) -> Vec<BracketItem> {
    let mut items = Vec::new();
    while let Some(token) = lexer.peek() {
        match token.item {
            BracketToken::Text => {
                lexer.advance();
                items.push(BracketItem::Text(WithSpan::new(Text, token.span)));
            }
            BracketToken::Open(kind) => {
                lexer.advance();
                items.push(parse_bracketed(lexer, enclosing, WithSpan::new(kind, token.span)));
            }
            BracketToken::Close(kind) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // synthetically closes every group between here and its owner.
                    break;
                }
                lexer.advance();
                items.push(BracketItem::StrayClose(WithSpan::new(kind, token.span)));
            }
        }
    }
    items
}

/// One group, its opening already consumed: parse children, then look at the one token that
/// stopped them — this group's own close (consumed, `Closing::Real`) or something an
/// enclosing group owns (left alone, and this group is closed synthetically where it stands).
fn parse_bracketed(
    lexer: &mut BracketLexer<'_>,
    enclosing: &mut Vec<Bracket>,
    opening: WithSpan<Bracket>,
) -> BracketItem {
    enclosing.push(opening.item);
    let children = parse_items(lexer, enclosing);
    enclosing.pop();

    let closing = match lexer.peek() {
        Some(token) if token.item == BracketToken::Close(opening.item) => {
            lexer.advance();
            Closing::Real(token.span)
        }
        Some(token) => Closing::Synthetic(Span::new(token.span.start, token.span.start)),
        None => Closing::Synthetic(lexer.end_of_literal()),
    };

    let span = Span::new(opening.span.start, closing.span().end);
    BracketItem::Bracketed(WithSpan::new(
        Bracketed {
            opening,
            closing,
            children,
        },
        span,
    ))
}
```

At the top level `enclosing` is empty, so `parse_items` never breaks there: every close bracket the recursion hands back up is consumed as a stray or by the group that owns it, and `match_brackets` consumes the whole literal. Synthetically closed groups nest correctly because each recursion level returns without consuming the close that stopped it: closing `}` against open `{`, `(`, `[` returns out of the square level, then the paren level, closing each synthetically at the `}`'s position, before the curly level consumes the `}` as its own.

## Change 2: position resolution and validity

New module `crates/isograph_parser/src/resolve_bracket_tree.rs`, implementing `resolve_position::ResolvePosition` for the bracket tree, the way `resolve_position`'s own doc comment describes.

`Text` and `StrayClose` paths point at the `WithSpan` (the node alone carries nothing); the `Bracketed` path points at the `Bracketed` itself, which is where `closing` — the fact validity reads — lives, and which is the type `ResolvePosition` is implemented on.

```rust
use parser_lang_types::{Span, WithSpan};
use resolve_position::{PositionResolutionPath, ResolvePosition};

#[derive(Debug)]
pub enum ResolvedBracketNode<'a> {
    BracketTree(BracketTreePath<'a>),
    Text(TextPath<'a>),
    Bracketed(BracketedPath<'a>),
    StrayClose(StrayClosePath<'a>),
}

pub type BracketTreePath<'a> = PositionResolutionPath<&'a BracketTree, ()>;

/// Everything a `BracketItem` can sit inside.
#[derive(Debug)]
pub enum BracketItemParent<'a> {
    BracketTree(BracketTreePath<'a>),
    Bracketed(Box<BracketedPath<'a>>),
}

pub type TextPath<'a> = PositionResolutionPath<&'a WithSpan<Text>, BracketItemParent<'a>>;
pub type BracketedPath<'a> = PositionResolutionPath<&'a Bracketed, BracketItemParent<'a>>;
pub type StrayClosePath<'a> = PositionResolutionPath<&'a WithSpan<Bracket>, BracketItemParent<'a>>;
```

Each impl finds the child containing the position and delegates; a node none of whose children contain the position is the leaf. The parent path is built once, in the branch that uses it:

```rust
impl ResolvePosition for BracketTree {
    type Parent<'a> = ();
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(&'a self, parent: (), position: Span) -> ResolvedBracketNode<'a> {
        match containing_child(&self.items, position) {
            Some(child) => {
                let parent = BracketItemParent::BracketTree(self.path(parent));
                resolve_child(child, parent, position)
            }
            None => ResolvedBracketNode::BracketTree(self.path(parent)),
        }
    }
}

impl ResolvePosition for Bracketed {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: BracketItemParent<'a>,
        position: Span,
    ) -> ResolvedBracketNode<'a> {
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
fn containing_child(items: &[BracketItem], position: Span) -> Option<&BracketItem> {
    items.iter().find(|item| item_span(item).contains(position))
}

/// Resolve into an item already known to contain the position.
fn resolve_child<'a>(
    child: &'a BracketItem,
    parent: BracketItemParent<'a>,
    position: Span,
) -> ResolvedBracketNode<'a> {
    match child {
        BracketItem::Text(text) => ResolvedBracketNode::Text(PositionResolutionPath {
            inner: text,
            parent,
        }),
        BracketItem::Bracketed(bracketed) => bracketed.item.resolve(parent, position),
        BracketItem::StrayClose(close) => ResolvedBracketNode::StrayClose(PositionResolutionPath {
            inner: close,
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

impl ResolvedBracketNode<'_> {
    pub fn validity(&self) -> SectionValidity {
        match self {
            ResolvedBracketNode::StrayClose(_) => SectionValidity::Invalid,
            ResolvedBracketNode::BracketTree(_) => SectionValidity::Valid,
            ResolvedBracketNode::Text(path) => path.parent.validity(),
            ResolvedBracketNode::Bracketed(path) => match path.inner.closing {
                Closing::Real(_) => path.parent.validity(),
                // Provisional: whether a group synthetically closed at the end of the
                // literal counts as invalid is the open question in
                // bracket-matching-cases.md.
                Closing::Synthetic(_) => SectionValidity::Invalid,
            },
        }
    }
}

impl BracketItemParent<'_> {
    fn validity(&self) -> SectionValidity {
        match self {
            BracketItemParent::BracketTree(_) => SectionValidity::Valid,
            BracketItemParent::Bracketed(path) => match path.inner.closing {
                Closing::Real(_) => path.parent.validity(),
                Closing::Synthetic(_) => SectionValidity::Invalid,
            },
        }
    }
}
```

A position inside a synthetically closed group is `Invalid` however deep it sits: the `Text` and really-closed `Bracketed` arms keep walking up, and the walk stops at the first `Closing::Synthetic` ancestor.

## Change 3: the test harness

`crates/tests` holds the harness in `src/lib.rs` and the tests in `tests/bracket_matching.rs`. Fixtures are files under `crates/tests/fixtures/`, extension `.iso`, containing literal text only (no `iso(...)` wrapper). A fixture is an input, never an expected output.

`crates/tests/Cargo.toml`:

```toml
[dependencies]
isograph_parser = { path = "../isograph_parser" }
parser_lang_types = { path = "../parser_lang_types" }
resolve_position = { path = "../resolve_position" }
```

`src/lib.rs`:

```rust
use std::path::Path;

use isograph_parser::{match_brackets, BracketTree, ResolvedBracketNode};
use parser_lang_types::Span;
use resolve_position::ResolvePosition;

pub struct Fixture {
    pub text: String,
    pub tree: BracketTree,
}

impl Fixture {
    pub fn load(name: &str) -> Fixture {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(format!("{name}.iso"));
        let text = std::fs::read_to_string(&path)
            .expect("the fixture named by the test exists under crates/tests/fixtures");
        let tree = match_brackets(&text);
        Fixture { text, tree }
    }

    /// The span of `pattern`, which must occur exactly once in the fixture. The preferred
    /// way to point at a position: editing the fixture cannot silently shift what the test
    /// asserts about, and a pattern that stops being unique fails loudly instead.
    pub fn span_of(&self, pattern: &str) -> Span {
        let mut occurrences = self.text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the fixture");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the fixture"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    /// The path to the node containing `span`.
    pub fn resolve(&self, span: Span) -> ResolvedBracketNode<'_> {
        self.tree.resolve((), span)
    }

    /// `resolve` at the unique occurrence of `pattern`.
    pub fn on(&self, pattern: &str) -> ResolvedBracketNode<'_> {
        self.resolve(self.span_of(pattern))
    }

    /// The node at a 0-indexed line and character; the character indexes bytes in the
    /// line. For positions no distinctive text names, such as whitespace between two
    /// sibling groups.
    pub fn at(&self, line: u32, character: u32) -> ResolvedBracketNode<'_> {
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

`on` resolves the whole pattern's span, so the node it lands on must contain every byte of the pattern: `on("broken")` sits in the text before the `(`, `on("(")` sits on the group whose opening that is, and a pattern spanning two siblings resolves to their common ancestor, which is itself a fact a test may assert.

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
use tests::Fixture;

#[test]
fn unclosed_paren_is_an_invalid_section() {
    let fixture = Fixture::load("unclosed_paren");
    // The `(` that the `}` refuses to close; it is the fixture's only paren.
    assert!(matches!(fixture.on("(").validity(), SectionValidity::Invalid));
}

#[test]
fn the_enclosing_curly_group_stays_valid() {
    let fixture = Fixture::load("unclosed_paren");
    // The text inside `first { ... }`, before the invalid paren section begins.
    assert!(matches!(fixture.on("broken").validity(), SectionValidity::Valid));
    // Inside `second { ok }`, after the broken section.
    assert!(matches!(fixture.on("ok").validity(), SectionValidity::Valid));
}

#[test]
fn the_unclosed_group_is_the_leaf_it_resolves_to() {
    let fixture = Fixture::load("unclosed_paren");
    match fixture.on("(") {
        ResolvedBracketNode::Bracketed(path) => {
            assert!(matches!(path.inner.closing, Closing::Synthetic(_)));
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
            assert_eq!(unclosed.opening.span, fixture.span_of("("));
        }
        errors => panic!("expected exactly the unclosed paren, got {errors:?}"),
    }
}
```

The initial fixture set follows `bracket-matching-cases.md`, one fixture per case there, each with the assertions its case states — validity at the marked positions, and the exact error list, which is empty for the two well-formed cases.
