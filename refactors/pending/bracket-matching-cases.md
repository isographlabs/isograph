# Bracket matching: the cases and why

The behavior of the bracket matcher (`resilient-parser.md`), pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass over the tokenizer's output, one token of lookahead — the discipline the existing parser's peekable lexer already sets — and no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

The tokenizer feeds the matcher, and everything else runs after it, on its balanced output: every group the matcher hands downstream has a definite end: its own close token, or a forced end recorded as a `None` closing, so no later pass ever sees a group without an extent. Each pass owns its own errors: the `UnexpectedClose` and `Unclosed` errors below are the matcher's; the tokenizer's `Error*` kinds ride through as run tokens, and stage 4 produces its own error tokens over the balanced tree. There will be other such passes.

The rule:

- `()`, `{}`, and `[]` are all matched.
- An open bracket begins a group. The group's children are parsed until the tokens end or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: `Some(CloseBracket)`. Otherwise the closing is `None`: the group is forced to end the moment the close it cannot match (or the end of the tokens) is encountered, it ends where its last child does (at its opening, when it has none), and it is an invalid section and an `Unclosed` error.
- Seen from the close's side, the same rule reads: a close bracket pairs with the nearest open bracket of its kind, and open brackets of other kinds above that one are forced shut before it, innermost first.
- A close bracket whose kind is open nowhere is a `StrayClose` where it stands: an invalid section one token wide and an `UnexpectedClose` error. It consumes nothing.
- Invalidity never spreads outward: not to siblings, not to the enclosing group. A position is in an invalid section iff the node it resolves to, or an ancestor, is a `StrayClose` or a group whose closing is `None`.
- A run of non-bracket tokens outside any bracket is a valid section by itself.

## The tree

What the matcher generates (`resilient-parser.md`'s Change 1 implements exactly this). `BracketKind` (paren `()`, brace `{}`, bracket `[]`) and `NonBracketTokenKind` are landed code from the tokenizer's split layer:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// The types one matched-brackets tree holds: what a run between brackets is, and what a
/// stray close carries. A pipeline stage is an implementor, and a pass that changes any
/// of these changes all of them at once, through `try_map` (error-refinement.md).
pub trait TreeContents {
    type Inner: fmt::Debug + PartialEq + Eq;
    type StrayClose: fmt::Debug + PartialEq + Eq;
}

/// The stage `match_brackets` produces: its runs hold lexed tokens, and the tree can carry
/// both bracket errors.
#[derive(Debug, PartialEq, Eq)]
pub struct BracketsMatched;

impl TreeContents for BracketsMatched {
    type Inner = Inner;
    type StrayClose = CloseBracket;
}

/// A maximal run of non-bracket tokens between brackets. Its span runs from its first
/// token's start to its last token's end, whitespace between them included.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct Inner(pub Vec<WithSpan<NonBracketTokenKind>>);

/// A group's opening bracket.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = OpenBracketParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct OpenBracket(pub BracketKind);

/// A close bracket token.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = BracketItemParent<'a>, resolved_node = ResolvedBracketNode<'a>)]
pub struct CloseBracket(pub BracketKind);

/// One isograph literal with its brackets matched. Spans live on the `WithSpan` wrapping
/// each item.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct MatchedBrackets<TContents: TreeContents>(
    #[resolve_field] pub Vec<WithSpan<BracketItem<TContents>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub enum BracketItem<TContents: TreeContents> {
    Inner(TContents::Inner),
    Bracketed(Bracketed<TContents>),
    StrayClose(TContents::StrayClose),
}

/// An open bracket, its children, and its close. The wrapping `WithSpan`'s span runs from
/// the start of the opening to the end of a real closing, or to the end of the last child
/// when the closing is `None`.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = BracketItemParent<'a>,
    resolved_node = ResolvedBracketNode<'a>,
    self_type_generics = <BracketsMatched>
)]
pub struct Bracketed<TContents: TreeContents> {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    #[resolve_field]
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
    /// The close the author typed, or `None` for a group that never got its close and was
    /// forced to end: at the close bracket an enclosing group owns, or at the end of the
    /// tokens. A `None` group is an invalid section.
    #[resolve_field]
    pub closing: Option<WithSpan<CloseBracket>>,
}
```

A matched group and an unmatched one are one shape: unmatchedness is the `None` closing, not a different node, so position resolution and stage 4 walk one shape. The stray close is its own variant because it is neither a run nor a group: it has no opening and no children, and folding it into `Bracketed` would make an item with neither bracket representable. The cases below are written against the `BracketsMatched` instantiation, since that is what the matcher generates; `StrayClose(Paren)` in them abbreviates `StrayClose(CloseBracket(Paren))`.

Every resolve impl is derived: the tree types and the three role types (`Inner`, `OpenBracket`, `CloseBracket`) carry `#[derive(ResolvePosition)]`, with `#[resolve_field]` on the fields shown above, concretely over `BracketsMatched`. Resolving a position against the tree yields a `ResolvedBracketNode` path whose leaves are a run, an opening bracket, or a close bracket; any close bracket resolves to `CloseBracket`, and the path says whether it is a group's closing or a stray item. A position on whitespace inside a group resolves to the group, and one outside every item resolves to the root.

The matcher's errors are derived from the tree, in source order:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<CloseBracket>),
    /// A group that never got its close.
    Unclosed(WithSpan<UnclosedGroup>),
}

/// The opening bracket of a group that never got its close. The wrapping `WithSpan`'s span
/// is the whole group; its end is where the close should have been.
#[derive(Debug, PartialEq, Eq)]
pub struct UnclosedGroup(pub WithSpan<OpenBracket>);

impl<TContents> MatchedBrackets<TContents>
where
    TContents: TreeContents<StrayClose = CloseBracket>,
{
    /// Empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError>;
}
```

Positions marked below use `^` under the character. A position is called invalid when the path from the node it resolves to up to the root passes through an unbalanced group or a stray close; the pass ships no collapsed answer, so a consumer reads this off the path. A position on whitespace the tokenizer skipped resolves to the enclosing group.

## Case: text outside any bracket

```
field Query.Foo
      ^ valid
```

Generates: one `Inner` item holding the lexed tokens (`Identifier`, `Identifier`, `Period`, `Identifier`). No errors.

An unbracketed run cannot be malformed at this stage, so it is a valid section on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ valid      ^ valid
```

Generates: `Bracketed` items nested as typed, every closing real, with the runs between brackets as `Inner` items. No errors.

Every close is its group's own; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
       ^ valid       ^ invalid (the paren group, which is just the `(`)
```

Generates: the brace group with its real closing; among its children, the paren group with a `None` closing and no children, so its span is the `(` alone. One error: `Unclosed` for the paren.

Reason: the `}` is strong evidence the author considers the brace section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and stage 4 keep working everywhere else in the group.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ invalid (paren section)
    ^ invalid (the `[` section, nested inside the paren section)
```

Generates: the brace group with its real closing; inside it the paren group, and inside that the `[` group, both with `None` closings — the `[` group childless (its span is the `[` alone), the paren group ending at its last child, the `[` group. Two `Unclosed` errors, in source order of their openings.

Same reason as above, applied twice; nesting is preserved so a position resolves through the same ancestry the author typed.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
  ^ valid
      ^ invalid (the `)` alone)
        ^ valid
```

Generates: the brace group with its real closing, whose children are a `Inner` item, a `StrayClose(Paren)`, and a `Inner` item. One error: `UnexpectedClose`.

The `)` does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The stray-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ invalid (the `}` alone)
```

Generates: the paren group with its real closing, holding a `StrayClose(Brace)`. One error: `UnexpectedClose`.

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one invalid section.

## Case: crossing pairs

```
( { ) }
  ^ invalid (the brace group, which is just the `{`)
      ^ invalid (the trailing `}`, whose `{` was already consumed)
```

Generates: the paren group with its real closing, holding the brace group with a `None` closing and no children (its span is the `{` alone); after the paren group, a top-level `StrayClose(Brace)`. Two errors: `Unclosed` for the brace group, then `UnexpectedClose` for the trailing `}`.

One crossing produces two invalid sections even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires looking past the `)` an unbounded distance, and any such rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single, left-to-right, one token ahead.

## Case: adjacent same-kind opens, one close

```
a {
  b {
    c
}
```

Generates: b's brace group with its real closing; a's brace group with a `None` closing, its children kept and its span reaching its last child (b's group). One error: `Unclosed` for a's group.

The `}` pairs with the nearest `{` (b's), because same-kind matching is always nearest-first: nesting is the common intent, and "nearest of its kind" is the rule everywhere else.

## Brackets inside strings

Resolved by running the matcher over the tokenizer's output: the tokenizer lexes string and block-string literals whole, so a bracket inside a string is part of a `StringLiteral` token and never structural.

```
{ name: "a}" }
          ^ valid (inside a StringLiteral token, in the brace group's run)
             ^ this closes the brace group
```

A malformed string lexes as whatever the tokenizer produces for it (an `Error` run token); that is an inner error for a later pass to report, and the matcher just sees a non-bracket token.

## Open question: validity at the end of the tokens

The tree at the end of the tokens is settled: every group still open is forced to end with a `None` closing, ending at its last child, its children kept and their nesting preserved, and each one is an `Unclosed` error. The pass exposes the path and nothing else, so what is open is the policy of whichever consumer collapses the path into one answer (the LSP's diagnostics and features): does content inside a group forced shut at the end of the tokens count as inside an unbalanced group?

The dominant real-world input is a literal being typed: the user has just written `{` and everything that follows is momentarily "after an unclosed open". Whatever we pick is the LSP experience during typing.

### Option A: invalid, like every forced end

```
field Query.Foo {
  id
  ^ invalid
```

One rule with no special case: a group with a `None` closing counts as unbalanced wherever it ended. The cost: while the user types inside a new `{`, the entire rest of the literal counts as unbalanced, so stage 4 has nothing to say about the content most likely to be under the cursor.

### Option B: balanced when the group's forced end is the end of the tokens

```
field Query.Foo {
  id
  ^ valid (inside the brace group)
```

Content counts as balanced while typing, nesting is already correct, nothing restructures when the real close is typed, and the missing brace is still reported: the `Unclosed` error exists either way, because errors are separate from the collapse. The cost: the collapse special-cases where the group was forced to end, and a group that counts as balanced while it reaches the end of the tokens flips when a wrong-kind close later forces it shut mid-literal — a change of state from an edit made elsewhere.
