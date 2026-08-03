# Resilient parser: brace matching and position-based tests

`isograph_parser` parses isograph literals in stages, and every stage is resilient: a malformed region degrades that region, never its siblings and never the file.

1. Extract the isograph literal from a source file. (Not a parsing step; it hands the stages below a `&str` and the literal's offset in the file.)
2. Match braces within the literal, recovering from unbalanced braces by grouping the broken section so its siblings still parse.
3. Parse each matched group's contents into the real AST.

This doc specifies stage 2 and its tests. Stage 3 gets its own doc once stage 2 lands; extraction is scheduled with it. The changes here land after `parser-lang-types.md`, whose `parser_lang_types` crate supplies `Span` and `WithSpan`.

Position lookup reuses `resolve_position`, the read-only analogue of freddie's laserbeam: given a `Span`, walk down the tree to the node containing it, producing a typed path from leaf to root. A test then asserts facts about that path — above all, whether any ancestor is an unmatched group.

## Change 1: the brace tree and `match_braces`

New module `crates/isograph_parser/src/brace_tree.rs`, re-exported from `lib.rs`. Spans are byte offsets into the literal (not the containing file).

```rust
use parser_lang_types::{Span, WithSpan};

/// One isograph literal with its braces matched. Content between braces is
/// uninterpreted at this stage.
#[derive(Debug, PartialEq, Eq)]
pub struct BraceTree {
    pub items: Vec<WithSpan<BraceItem>>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum BraceItem {
    Text(TextSegment),
    Matched(MatchedGroup),
    UnmatchedOpen(UnmatchedGroup),
    UnmatchedClose(UnmatchedClose),
}

/// A maximal run of text containing no braces. Its span lives on the enclosing `WithSpan`.
#[derive(Debug, PartialEq, Eq)]
pub struct TextSegment;

/// `{`, its matching `}`, and everything between them. The enclosing `WithSpan`'s span
/// runs from the start of the `{` to the end of the `}`.
#[derive(Debug, PartialEq, Eq)]
pub struct MatchedGroup {
    pub open: Span,
    pub close: Span,
    pub children: Vec<WithSpan<BraceItem>>,
}

/// `{` with no matching `}`. Its children are still brace-matched, so positions inside
/// resolve to real nodes, but the group is poisoned: stage 3 does not interpret its contents.
#[derive(Debug, PartialEq, Eq)]
pub struct UnmatchedGroup {
    pub open: Span,
    pub children: Vec<WithSpan<BraceItem>>,
}

/// `}` with no matching `{`. Its span lives on the enclosing `WithSpan`.
#[derive(Debug, PartialEq, Eq)]
pub struct UnmatchedClose;
```

Every `{` and `}` in the literal is structural at this stage.

### Recovery rule

A `}` pairs with the innermost open group whose opening line is indented no deeper than the `}`'s line. Open groups indented deeper than the `}` are finalized as `UnmatchedOpen`, ending just before the `}`. So in

```
first {
  broken {
}
second {
  ok
}
```

the first `}` (indent 0) does not pair with `broken` (opened at indent 2): `broken` becomes an `UnmatchedOpen` and the `}` closes `first`. `second { ok }` is an ordinary `Matched` sibling. Open groups still on the stack at the end of the literal become `UnmatchedOpen` groups ending at the end of the literal.

### Implementation

Line starts and indents are precomputed once and shared with the test harness (Change 3), so they are `pub`:

```rust
/// Byte offsets of line starts and each line's indentation, for one literal.
pub struct LineTable {
    /// The byte offset at which each line begins. `line_starts[0] == 0`.
    line_starts: Vec<u32>,
    /// The number of leading space/tab bytes on each line.
    indents: Vec<u32>,
}

impl LineTable {
    pub fn new(text: &str) -> LineTable;

    /// The line containing `offset` (0-indexed).
    pub fn line_of(&self, offset: u32) -> u32;

    /// The indentation of the line containing `offset`.
    pub fn indent_at(&self, offset: u32) -> u32;

    /// The byte offset of a 0-indexed line and character. The character indexes bytes
    /// within the line.
    pub fn offset(&self, line: u32, character: u32) -> u32;
}
```

The matcher:

```rust
pub fn match_braces(literal: &str) -> BraceTree {
    let lines = LineTable::new(literal);
    let mut root: Vec<WithSpan<BraceItem>> = Vec::new();
    let mut stack: Vec<OpenGroup> = Vec::new();
    let mut text_start: u32 = 0;

    for (index, byte) in literal.bytes().enumerate() {
        let index = index as u32;
        if byte != b'{' && byte != b'}' {
            continue;
        }
        flush_text(&mut root, &mut stack, text_start, index);
        text_start = index + 1;

        if byte == b'{' {
            stack.push(OpenGroup {
                open: Span::new(index, index + 1),
                children: Vec::new(),
            });
            continue;
        }

        // A `}`. Groups opened on lines indented deeper than this close's line do not
        // pair with it; they end here, unmatched.
        let close_indent = lines.indent_at(index);
        while stack
            .last()
            .is_some_and(|group| lines.indent_at(group.open.start) > close_indent)
        {
            let group = stack.pop().expect("the loop condition saw a last element");
            finalize_unmatched(&mut root, &mut stack, group, index);
        }

        match stack.pop() {
            Some(group) => {
                let close = Span::new(index, index + 1);
                let span = Span::new(group.open.start, close.end);
                let item = BraceItem::Matched(MatchedGroup {
                    open: group.open,
                    close,
                    children: group.children,
                });
                push_child(&mut root, &mut stack, WithSpan::new(item, span));
            }
            None => {
                let span = Span::new(index, index + 1);
                root.push(WithSpan::new(BraceItem::UnmatchedClose(UnmatchedClose), span));
            }
        }
    }

    flush_text(&mut root, &mut stack, text_start, literal.len() as u32);
    while let Some(group) = stack.pop() {
        finalize_unmatched(&mut root, &mut stack, group, literal.len() as u32);
    }

    BraceTree { items: root }
}

struct OpenGroup {
    open: Span,
    children: Vec<WithSpan<BraceItem>>,
}

/// The children of the innermost open group, or of the root when no group is open.
fn current_children<'a>(
    root: &'a mut Vec<WithSpan<BraceItem>>,
    stack: &'a mut Vec<OpenGroup>,
) -> &'a mut Vec<WithSpan<BraceItem>> {
    match stack.last_mut() {
        Some(group) => &mut group.children,
        None => root,
    }
}

fn push_child(
    root: &mut Vec<WithSpan<BraceItem>>,
    stack: &mut Vec<OpenGroup>,
    child: WithSpan<BraceItem>,
) {
    current_children(root, stack).push(child);
}

/// Record `[text_start, end)` as a text segment, if it is nonempty.
fn flush_text(
    root: &mut Vec<WithSpan<BraceItem>>,
    stack: &mut Vec<OpenGroup>,
    text_start: u32,
    end: u32,
) {
    if text_start < end {
        let span = Span::new(text_start, end);
        push_child(root, stack, WithSpan::new(BraceItem::Text(TextSegment), span));
    }
}

/// End `group` at `end` (exclusive) as an `UnmatchedOpen`, attaching it to its parent.
fn finalize_unmatched(
    root: &mut Vec<WithSpan<BraceItem>>,
    stack: &mut Vec<OpenGroup>,
    group: OpenGroup,
    end: u32,
) {
    let span = Span::new(group.open.start, end);
    let item = BraceItem::UnmatchedOpen(UnmatchedGroup {
        open: group.open,
        children: group.children,
    });
    push_child(root, stack, WithSpan::new(item, span));
}
```

The one `expect` names an invariant the surrounding two lines establish; if the `while let` idiom can express the pop-and-test without it, use that instead.

## Change 2: position resolution and validity

New module `crates/isograph_parser/src/resolve_brace_tree.rs`, implementing `resolve_position::ResolvePosition` for the brace tree, mirroring how `resolve_position`'s own doc comment describes the isograph AST impl.

```rust
use parser_lang_types::{Span, WithSpan};
use resolve_position::{PositionResolutionPath, ResolvePosition};

pub enum ResolvedBraceNode<'a> {
    BraceTree(BraceTreePath<'a>),
    Text(TextSegmentPath<'a>),
    Matched(MatchedGroupPath<'a>),
    UnmatchedOpen(UnmatchedGroupPath<'a>),
    UnmatchedClose(UnmatchedClosePath<'a>),
}

pub type BraceTreePath<'a> = PositionResolutionPath<&'a BraceTree, ()>;

/// Everything a `BraceItem` can sit inside.
pub enum BraceItemParent<'a> {
    BraceTree(BraceTreePath<'a>),
    Matched(Box<MatchedGroupPath<'a>>),
    UnmatchedOpen(Box<UnmatchedGroupPath<'a>>),
}

pub type TextSegmentPath<'a> =
    PositionResolutionPath<&'a WithSpan<TextSegment>, BraceItemParent<'a>>;
pub type MatchedGroupPath<'a> =
    PositionResolutionPath<&'a MatchedGroup, BraceItemParent<'a>>;
pub type UnmatchedGroupPath<'a> =
    PositionResolutionPath<&'a UnmatchedGroup, BraceItemParent<'a>>;
pub type UnmatchedClosePath<'a> =
    PositionResolutionPath<&'a WithSpan<UnmatchedClose>, BraceItemParent<'a>>;
```

Each impl checks which child's span contains the position and delegates; a node whose children do not contain the position is the leaf:

```rust
fn resolve_items<'a>(
    items: &'a [WithSpan<BraceItem>],
    parent: impl Fn() -> BraceItemParent<'a>,
    position: Span,
) -> Option<ResolvedBraceNode<'a>> {
    for item in items {
        if item.span.contains(position) {
            return Some(match &item.item {
                BraceItem::Text(text) => ResolvedBraceNode::Text(PositionResolutionPath {
                    // The span lives on the WithSpan, so the path points at the WithSpan.
                    inner: /* &WithSpan<TextSegment>, projected from item */,
                    parent: parent(),
                }),
                BraceItem::Matched(group) => group.resolve(parent(), position),
                BraceItem::UnmatchedOpen(group) => group.resolve(parent(), position),
                BraceItem::UnmatchedClose(close) => ResolvedBraceNode::UnmatchedClose(
                    PositionResolutionPath { inner: /* &WithSpan<UnmatchedClose> */, parent: parent() },
                ),
            });
        }
    }
    None
}

impl ResolvePosition for BraceTree {
    type Parent<'a> = ();
    type ResolvedNode<'a> = ResolvedBraceNode<'a>;

    fn resolve<'a>(&'a self, parent: (), position: Span) -> ResolvedBraceNode<'a> {
        resolve_items(&self.items, || BraceItemParent::BraceTree(self.path(parent)), position)
            .unwrap_or_else(|| ResolvedBraceNode::BraceTree(self.path(parent)))
    }
}

impl ResolvePosition for MatchedGroup {
    type Parent<'a> = BraceItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBraceNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: BraceItemParent<'a>,
        position: Span,
    ) -> ResolvedBraceNode<'a> {
        resolve_items(
            &self.children,
            || BraceItemParent::Matched(Box::new(self.path(parent))),
            position,
        )
        .unwrap_or_else(|| ResolvedBraceNode::Matched(self.path(parent)))
    }
}

impl ResolvePosition for UnmatchedGroup {
    type Parent<'a> = BraceItemParent<'a>;
    type ResolvedNode<'a> = ResolvedBraceNode<'a>;

    fn resolve<'a>(
        &'a self,
        parent: BraceItemParent<'a>,
        position: Span,
    ) -> ResolvedBraceNode<'a> {
        resolve_items(
            &self.children,
            || BraceItemParent::UnmatchedOpen(Box::new(self.path(parent))),
            position,
        )
        .unwrap_or_else(|| ResolvedBraceNode::UnmatchedOpen(self.path(parent)))
    }
}
```

`resolve_items` is written here with a `Fn` closure and projection holes; the implementation may instead inline the loop into each impl (the shape `resolve_position`'s own test module uses) if the projection from `&WithSpan<BraceItem>` to `&WithSpan<TextSegment>` is not expressible. Either way the two `Path` types point at what carries the span, and `parent` is built at most once per call.

Validity is a fact about the whole path, not the leaf: a position is in a matched section iff neither its node nor any ancestor is unmatched.

```rust
pub enum BraceValidity {
    Matched,
    Unmatched,
}

impl ResolvedBraceNode<'_> {
    pub fn validity(&self) -> BraceValidity {
        match self {
            ResolvedBraceNode::UnmatchedOpen(_) | ResolvedBraceNode::UnmatchedClose(_) => {
                BraceValidity::Unmatched
            }
            ResolvedBraceNode::BraceTree(_) => BraceValidity::Matched,
            ResolvedBraceNode::Text(path) => path.parent.validity(),
            ResolvedBraceNode::Matched(path) => path.parent.validity(),
        }
    }
}

impl BraceItemParent<'_> {
    fn validity(&self) -> BraceValidity {
        match self {
            BraceItemParent::BraceTree(_) => BraceValidity::Matched,
            BraceItemParent::Matched(path) => path.parent.validity(),
            BraceItemParent::UnmatchedOpen(_) => BraceValidity::Unmatched,
        }
    }
}
```

A position inside an `UnmatchedOpen` group is `Unmatched` however deep it sits: the `Matched` and `Text` arms keep walking up, and the walk stops at the first `UnmatchedOpen` ancestor.

## Change 3: the test harness

`crates/tests` holds the harness in `src/lib.rs` and the tests in `tests/brace_matching.rs`. Fixtures are files under `crates/tests/fixtures/`, extension `.iso`, containing literal text only (no `iso(...)` wrapper). A fixture is an input, never an expected output.

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

use isograph_parser::{match_braces, BraceTree, LineTable, ResolvedBraceNode};
use parser_lang_types::Span;
use resolve_position::ResolvePosition;

pub struct Fixture {
    pub text: String,
    pub tree: BraceTree,
    lines: LineTable,
}

impl Fixture {
    pub fn load(name: &str) -> Fixture {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("fixtures")
            .join(format!("{name}.iso"));
        let text = std::fs::read_to_string(&path)
            .expect("the fixture named by the test exists under crates/tests/fixtures");
        let tree = match_braces(&text);
        let lines = LineTable::new(&text);
        Fixture { text, tree, lines }
    }

    /// The node at a 0-indexed line and character; the character indexes bytes in the line.
    pub fn at(&self, line: u32, character: u32) -> ResolvedBraceNode<'_> {
        let offset = self.lines.offset(line, character);
        self.tree.resolve((), Span::new(offset, offset + 1))
    }
}
```

A test names a fixture, points at positions, and asserts facts:

`fixtures/unclosed_inner.iso`:

```
first {
  broken {
}
second {
  ok
}
```

`tests/brace_matching.rs`:

```rust
use isograph_parser::{BraceValidity, ResolvedBraceNode};
use tests::Fixture;

#[test]
fn unclosed_group_is_unmatched() {
    let fixture = Fixture::load("unclosed_inner");
    // Inside `broken {`, which never closes.
    assert!(matches!(fixture.at(1, 10).validity(), BraceValidity::Unmatched));
}

#[test]
fn siblings_of_an_unclosed_group_still_match() {
    let fixture = Fixture::load("unclosed_inner");
    // Inside `first { ... }`, whose close the recovery rule preserved.
    assert!(matches!(fixture.at(0, 3).validity(), BraceValidity::Matched));
    // Inside `second { ok }`, after the broken section.
    assert!(matches!(fixture.at(4, 3).validity(), BraceValidity::Matched));
}

#[test]
fn the_unmatched_group_is_the_leaf_it_resolves_to() {
    let fixture = Fixture::load("unclosed_inner");
    let node = fixture.at(1, 10);
    assert!(matches!(node, ResolvedBraceNode::UnmatchedOpen(_)));
}
```

The initial fixture set, each with the assertions its shape calls for:

- `balanced.iso` — nested matched groups; every position is `Matched`, and a position between two sibling groups resolves to the tree or a text segment, not a group.
- `unclosed_inner.iso` — as above.
- `unclosed_outer.iso` — an unclosed top-level group with matched children; positions inside the children are `Unmatched` (the poisoned ancestor), positions before the group are `Matched`.
- `stray_close.iso` — a `}` with nothing open; the close itself is `Unmatched`, text before and after it is `Matched`.
- `same_line.iso` — `a { b { c } }` on one line; the indentation rule must not fire, everything is `Matched`.
