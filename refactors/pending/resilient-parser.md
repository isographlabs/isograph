# Resilient parser: bracket matching and position-based tests

`isograph_parser` parses isograph literals in stages, and every stage is resilient: a malformed region degrades that region, never its siblings and never the file.

1. Extract the isograph literal from a source file. (Not a parsing step; it hands the stages below a `&str` and the literal's offset in the file.)
2. Match brackets — `()`, `{}`, `[]` — within the literal. A section that does not match becomes an invalid section; everything around it stays valid.
3. Parse each matched group's contents into the real AST.

This doc specifies stage 2 and its tests. Stage 3 gets its own doc once stage 2 lands; extraction is scheduled with it. The changes here land after `parser-lang-types.md`, whose `parser_lang_types` crate supplies `Span` and `WithSpan`. The matching rule's behavior, case by case, with the reasons behind it and the open end-of-literal question, lives in `bracket-matching-cases.md`; this doc implements what that one decides.

Position lookup reuses `resolve_position`, the read-only analogue of freddie's laserbeam: given a `Span`, walk down the tree to the node containing it, producing a typed path from leaf to root. A test then asserts facts about that path — above all, whether any ancestor is an unmatched group.

## Change 1: the bracket tree and `match_brackets`

New module `crates/isograph_parser/src/bracket_tree.rs`, re-exported from `lib.rs`. Spans are byte offsets into the literal (not the containing file).

```rust
use parser_lang_types::{Span, WithSpan};

/// One isograph literal with its brackets matched. Content between brackets is
/// uninterpreted at this stage.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketTree {
    pub items: Vec<WithSpan<BracketItem>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BracketItem {
    Text(TextSegment),
    Matched(MatchedGroup),
    UnmatchedOpen(UnmatchedGroup),
    UnmatchedClose(UnmatchedClose),
}

#[derive(Debug, PartialEq, Eq, Copy, Clone)]
pub enum BracketKind {
    Paren,
    Curly,
    Square,
}

/// A maximal run of text containing no brackets. Its span lives on the enclosing `WithSpan`.
#[derive(Debug, PartialEq, Eq)]
pub struct TextSegment;

/// An open bracket, its matching close of the same kind, and everything between them. The
/// enclosing `WithSpan`'s span runs from the start of the open to the end of the close.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedGroup {
    pub kind: BracketKind,
    pub open: Span,
    pub close: Span,
    pub children: Vec<WithSpan<BracketItem>>,
}

/// An open bracket with no matching close: an invalid section. Its children are still
/// bracket-matched, so positions inside resolve to real nodes, but stage 3 does not
/// interpret its contents.
#[derive(Debug, PartialEq, Eq)]
pub struct UnmatchedGroup {
    pub kind: BracketKind,
    pub open: Span,
    pub children: Vec<WithSpan<BracketItem>>,
}

/// A close bracket with no open of its kind anywhere on the stack: an invalid section of
/// exactly one character. Its span lives on the enclosing `WithSpan`.
#[derive(Debug, PartialEq, Eq)]
pub struct UnmatchedClose {
    pub kind: BracketKind,
}
```

Every `(`, `)`, `{`, `}`, `[`, and `]` in the literal is structural at this stage.

### The matching rule

- An open bracket opens a group.
- A close bracket pairs with the nearest open bracket of its kind. Open brackets of other kinds sitting above that one never got their close: each becomes an `UnmatchedGroup` ending just before the close, nested in order.
- A close bracket whose kind has no open anywhere on the stack is an `UnmatchedClose` where it stands. It consumes nothing: an open of a different kind stays open and may still match later.
- At the end of the literal, every group still open becomes an `UnmatchedGroup` ending at the end of the literal. (Provisional: this is the open question in `bracket-matching-cases.md`, and that doc's decision supersedes this rule.)

### Implementation

```rust
pub fn match_brackets(literal: &str) -> BracketTree {
    let mut root: Vec<WithSpan<BracketItem>> = Vec::new();
    let mut stack: Vec<OpenGroup> = Vec::new();
    let mut text_start: u32 = 0;

    for (index, byte) in literal.bytes().enumerate() {
        let index = index as u32;
        let bracket = match byte {
            b'(' => Bracket::Open(BracketKind::Paren),
            b')' => Bracket::Close(BracketKind::Paren),
            b'{' => Bracket::Open(BracketKind::Curly),
            b'}' => Bracket::Close(BracketKind::Curly),
            b'[' => Bracket::Open(BracketKind::Square),
            b']' => Bracket::Close(BracketKind::Square),
            _ => continue,
        };
        flush_text(&mut root, &mut stack, text_start, index);
        text_start = index + 1;

        match bracket {
            Bracket::Open(kind) => stack.push(OpenGroup {
                kind,
                open: Span::new(index, index + 1),
                children: Vec::new(),
            }),
            Bracket::Close(kind) => close_bracket(&mut root, &mut stack, kind, index),
        }
    }

    flush_text(&mut root, &mut stack, text_start, literal.len() as u32);
    while let Some(group) = stack.pop() {
        finalize_unmatched(&mut root, &mut stack, group, literal.len() as u32);
    }

    BracketTree { items: root }
}

enum Bracket {
    Open(BracketKind),
    Close(BracketKind),
}

struct OpenGroup {
    kind: BracketKind,
    open: Span,
    children: Vec<WithSpan<BracketItem>>,
}

/// Pair a close bracket with the nearest open of its kind.
fn close_bracket(
    root: &mut Vec<WithSpan<BracketItem>>,
    stack: &mut Vec<OpenGroup>,
    kind: BracketKind,
    index: u32,
) {
    // Checked up front because a close whose kind is nowhere on the stack consumes
    // nothing: without this check, the pops below would end groups that may still match.
    if !stack.iter().any(|group| group.kind == kind) {
        let span = Span::new(index, index + 1);
        let item = BracketItem::UnmatchedClose(UnmatchedClose { kind });
        push_child(root, stack, WithSpan::new(item, span));
        return;
    }

    while let Some(group) = stack.pop_if(|group| group.kind != kind) {
        finalize_unmatched(root, stack, group, index);
    }

    let group = stack
        .pop()
        .expect("the any() above found an open of this kind, and pop_if stopped at it");
    let close = Span::new(index, index + 1);
    let span = Span::new(group.open.start, close.end);
    let item = BracketItem::Matched(MatchedGroup {
        kind: group.kind,
        open: group.open,
        close,
        children: group.children,
    });
    push_child(root, stack, WithSpan::new(item, span));
}

/// The children of the innermost open group, or of the root when no group is open.
fn current_children<'a>(
    root: &'a mut Vec<WithSpan<BracketItem>>,
    stack: &'a mut Vec<OpenGroup>,
) -> &'a mut Vec<WithSpan<BracketItem>> {
    match stack.last_mut() {
        Some(group) => &mut group.children,
        None => root,
    }
}

fn push_child(
    root: &mut Vec<WithSpan<BracketItem>>,
    stack: &mut Vec<OpenGroup>,
    child: WithSpan<BracketItem>,
) {
    current_children(root, stack).push(child);
}

/// Record `[text_start, end)` as a text segment, if it is nonempty.
fn flush_text(
    root: &mut Vec<WithSpan<BracketItem>>,
    stack: &mut Vec<OpenGroup>,
    text_start: u32,
    end: u32,
) {
    if text_start < end {
        let span = Span::new(text_start, end);
        push_child(root, stack, WithSpan::new(BracketItem::Text(TextSegment), span));
    }
}

/// End `group` at `end` (exclusive) as an `UnmatchedGroup`, attaching it to its parent.
fn finalize_unmatched(
    root: &mut Vec<WithSpan<BracketItem>>,
    stack: &mut Vec<OpenGroup>,
    group: OpenGroup,
    end: u32,
) {
    let span = Span::new(group.open.start, end);
    let item = BracketItem::UnmatchedOpen(UnmatchedGroup {
        kind: group.kind,
        open: group.open,
        children: group.children,
    });
    push_child(root, stack, WithSpan::new(item, span));
}
```

The unmatched groups nest correctly because each pop attaches to the new top of the stack: closing `}` against a stack of `{`, `(`, `[` attaches the `[` group to the `(` group, then the `(` group (now containing it) to the `{` group. The one `expect` names an invariant the `any()` check establishes.

## Change 2: position resolution and validity

New module `crates/isograph_parser/src/resolve_bracket_tree.rs`, implementing `resolve_position::ResolvePosition` for the bracket tree, mirroring how `resolve_position`'s own doc comment describes the isograph AST impl.

```rust
use parser_lang_types::{Span, WithSpan};
use resolve_position::{PositionResolutionPath, ResolvePosition};

pub enum ResolvedBracketNode<'a> {
    BracketTree(BracketTreePath<'a>),
    Text(TextSegmentPath<'a>),
    Matched(MatchedGroupPath<'a>),
    UnmatchedOpen(UnmatchedGroupPath<'a>),
    UnmatchedClose(UnmatchedClosePath<'a>),
}

pub type BracketTreePath<'a> = PositionResolutionPath<&'a BracketTree, ()>;

/// Everything a `BracketItem` can sit inside.
pub enum BracketItemParent<'a> {
    BracketTree(BracketTreePath<'a>),
    Matched(Box<MatchedGroupPath<'a>>),
    UnmatchedOpen(Box<UnmatchedGroupPath<'a>>),
}

pub type TextSegmentPath<'a> =
    PositionResolutionPath<&'a WithSpan<TextSegment>, BracketItemParent<'a>>;
pub type MatchedGroupPath<'a> =
    PositionResolutionPath<&'a MatchedGroup, BracketItemParent<'a>>;
pub type UnmatchedGroupPath<'a> =
    PositionResolutionPath<&'a UnmatchedGroup, BracketItemParent<'a>>;
pub type UnmatchedClosePath<'a> =
    PositionResolutionPath<&'a WithSpan<UnmatchedClose>, BracketItemParent<'a>>;
```

Each impl checks which child's span contains the position and delegates; a node whose children do not contain the position is the leaf:

```rust
fn resolve_items<'a>(
    items: &'a [WithSpan<BracketItem>],
    parent: impl Fn() -> BracketItemParent<'a>,
    position: Span,
) -> Option<ResolvedBracketNode<'a>> {
    for item in items {
        if item.span.contains(position) {
            return Some(match &item.item {
                BracketItem::Text(_) => ResolvedBracketNode::Text(PositionResolutionPath {
                    // The span lives on the WithSpan, so the path points at the WithSpan.
                    inner: /* &WithSpan<TextSegment>, projected from item */,
                    parent: parent(),
                }),
                BracketItem::Matched(group) => group.resolve(parent(), position),
                BracketItem::UnmatchedOpen(group) => group.resolve(parent(), position),
                BracketItem::UnmatchedClose(_) => ResolvedBracketNode::UnmatchedClose(
                    PositionResolutionPath {
                        inner: /* &WithSpan<UnmatchedClose>, projected from item */,
                        parent: parent(),
                    },
                ),
            });
        }
    }
    None
}

impl ResolvePosition for BracketTree {
    type Parent<'a> = ();
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(&'a self, parent: (), position: Span) -> ResolvedBracketNode<'a> {
        resolve_items(&self.items, || BracketItemParent::BracketTree(self.path(parent)), position)
            .unwrap_or_else(|| ResolvedBracketNode::BracketTree(self.path(parent)))
    }
}

impl ResolvePosition for MatchedGroup {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: BracketItemParent<'a>,
        position: Span,
    ) -> ResolvedBracketNode<'a> {
        resolve_items(
            &self.children,
            || BracketItemParent::Matched(Box::new(self.path(parent))),
            position,
        )
        .unwrap_or_else(|| ResolvedBracketNode::Matched(self.path(parent)))
    }
}

impl ResolvePosition for UnmatchedGroup {
    type Parent<'a> = BracketItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBracketNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: BracketItemParent<'a>,
        position: Span,
    ) -> ResolvedBracketNode<'a> {
        resolve_items(
            &self.children,
            || BracketItemParent::UnmatchedOpen(Box::new(self.path(parent))),
            position,
        )
        .unwrap_or_else(|| ResolvedBracketNode::UnmatchedOpen(self.path(parent)))
    }
}
```

`resolve_items` is written here with a `Fn` closure and projection holes; the implementation may instead inline the loop into each impl (the shape `resolve_position`'s own test module uses) if the projection from `&WithSpan<BracketItem>` to `&WithSpan<TextSegment>` is not expressible. Either way the two `Path` types point at what carries the span, and `parent` is built at most once per call.

Validity is a fact about the whole path, not the leaf: a position is in a valid section iff neither its node nor any ancestor is unmatched. Invalidity never spreads outward — not to siblings, not to the enclosing matched group.

```rust
pub enum SectionValidity {
    Valid,
    Invalid,
}

impl ResolvedBracketNode<'_> {
    pub fn validity(&self) -> SectionValidity {
        match self {
            ResolvedBracketNode::UnmatchedOpen(_) | ResolvedBracketNode::UnmatchedClose(_) => {
                SectionValidity::Invalid
            }
            ResolvedBracketNode::BracketTree(_) => SectionValidity::Valid,
            ResolvedBracketNode::Text(path) => path.parent.validity(),
            ResolvedBracketNode::Matched(path) => path.parent.validity(),
        }
    }
}

impl BracketItemParent<'_> {
    fn validity(&self) -> SectionValidity {
        match self {
            BracketItemParent::BracketTree(_) => SectionValidity::Valid,
            BracketItemParent::Matched(path) => path.parent.validity(),
            BracketItemParent::UnmatchedOpen(_) => SectionValidity::Invalid,
        }
    }
}
```

A position inside an `UnmatchedGroup` is `Invalid` however deep it sits: the `Matched` and `Text` arms keep walking up, and the walk stops at the first `UnmatchedOpen` ancestor.

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

    /// The node at a 0-indexed line and character; the character indexes bytes in the line.
    pub fn at(&self, line: u32, character: u32) -> ResolvedBracketNode<'_> {
        let offset = self.offset(line, character);
        self.tree.resolve((), Span::new(offset, offset + 1))
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
use isograph_parser::{ResolvedBracketNode, SectionValidity};
use tests::Fixture;

#[test]
fn unclosed_paren_is_an_invalid_section() {
    let fixture = Fixture::load("unclosed_paren");
    // On `broken (`, which the `}` refuses to close.
    assert!(matches!(fixture.at(1, 9).validity(), SectionValidity::Invalid));
}

#[test]
fn the_enclosing_curly_group_stays_valid() {
    let fixture = Fixture::load("unclosed_paren");
    // Inside `first { ... }`, before the invalid paren section.
    assert!(matches!(fixture.at(0, 3).validity(), SectionValidity::Valid));
    // Inside `second { ok }`, after the broken section.
    assert!(matches!(fixture.at(4, 3).validity(), SectionValidity::Valid));
}

#[test]
fn the_unmatched_group_is_the_leaf_it_resolves_to() {
    let fixture = Fixture::load("unclosed_paren");
    let node = fixture.at(1, 9);
    assert!(matches!(node, ResolvedBracketNode::UnmatchedOpen(_)));
}
```

The initial fixture set follows `bracket-matching-cases.md`, one fixture per case there, each with the assertions its case states.
