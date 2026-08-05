# Bracket matching: the cases and why

The behavior of the bracket matcher (`resilient-parser.md`), pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass over the tokenizer's output, one token of lookahead — the discipline the existing parser's peekable lexer already sets — and no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

The tokenizer feeds the matcher, and everything else runs after it, on its balanced output: every group the matcher hands downstream has a close, real or synthesized, so no later pass ever sees an unclosed bracket. Each pass owns its own errors: the `UnexpectedClose` and `Unclosed` errors below are the matcher's; the tokenizer's `Error*` kinds ride through as run tokens, and stage 4 produces its own error tokens over the balanced tree. There will be other such passes.

The rule:

- `()`, `{}`, and `[]` are all matched.
- An open bracket begins a group. The group's children are parsed until the tokens end or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: `Closing::Real`. Otherwise the group is `Closing::Synthetic`: it is forced to end the moment the close it cannot match (or the end of the tokens) is encountered, it ends where its last child does (at its opening, when it has none), and it is an invalid section and an `Unclosed` error.
- Seen from the close's side, the same rule reads: a close bracket pairs with the nearest open bracket of its kind, and open brackets of other kinds above that one are synthetically closed before it, innermost first.
- A close bracket whose kind is open nowhere is a `StrayClose` where it stands: an invalid section one token wide and an `UnexpectedClose` error. It consumes nothing.
- Invalidity never spreads outward: not to siblings, not to the enclosing group. A position is in an invalid section iff the node it resolves to, or an ancestor, is a `StrayClose` or a synthetically closed group.
- A run of non-bracket tokens outside any bracket is a valid section by itself.

## The tree

What the matcher generates (`resilient-parser.md`'s Change 1 implements exactly this):

```rust
/// The types one matched-brackets tree holds: what a run between brackets is, and what the
/// two bracket errors carry. A pipeline stage is an implementor, and a pass that changes any
/// of these changes all of them at once, through `try_map` (error-refinement.md).
pub trait TreeContents {
    type Run: fmt::Debug + PartialEq + Eq;
    type Stray: fmt::Debug + PartialEq + Eq;
    type Unclosed: fmt::Debug + PartialEq + Eq;
}

/// What `match_brackets` produces: runs of lexed tokens, both bracket errors representable.
pub struct BracketsMatched;

impl TreeContents for BracketsMatched {
    type Run = Vec<WithSpan<NonBracketTokenKind>>;
    type Stray = BracketKind;
    type Unclosed = ();
}

/// One isograph literal with its brackets matched. Spans live on the `WithSpan` wrapping
/// each item.
pub struct MatchedBrackets<TContents: TreeContents>(pub Vec<WithSpan<BracketItem<TContents>>>);

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
pub struct Bracketed<TContents: TreeContents> {
    pub opening: WithSpan<BracketKind>,
    pub closing: Closing<TContents>,
    pub children: Vec<WithSpan<BracketItem<TContents>>>,
}

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

`BracketKind` (paren `()`, brace `{}`, bracket `[]`, isograph's token vocabulary) and `NonBracketTokenKind` landed with the tokenizer.

A matched group and an unmatched one are one variant: unmatchedness is `Closing::Synthetic`, not a different node, so position resolution and stage 4 walk one shape. The stray close is its own variant because it is neither a run nor a group: it has no opening and no children, and folding it into `Bracketed` would make an item with neither bracket representable. The cases below spell the `BracketsMatched` instantiation, since that is what the matcher generates; `Closing::Synthetic` in them abbreviates `Closing::Synthetic(())`.

The matcher's errors are derived from the tree, in source order:

```rust
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<BracketKind>),
    /// A group whose close was synthesized.
    Unclosed(WithSpan<UnclosedGroup>),
}

/// The wrapping `WithSpan`'s span is the whole group; its end is where the close should have
/// been.
pub struct UnclosedGroup {
    pub opening: WithSpan<BracketKind>,
}

impl<TContents> MatchedBrackets<TContents>
where
    TContents: TreeContents<Stray = BracketKind, Unclosed = ()>,
{
    /// Empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError>;
}
```

Positions marked below use `^` under the character; `valid`/`invalid` states what `validity()` returns there. A position on whitespace the tokenizer skipped resolves to the enclosing group.

## Case: text outside any bracket

```
field Query.Foo
      ^ valid
```

Generates: one `Run` item holding the lexed tokens (`Identifier`, `Identifier`, `Period`, `Identifier`). No errors.

An unbracketed run cannot be malformed at this stage, so it is a valid section on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ valid      ^ valid
```

Generates: `Bracketed` items nested as typed, every one `Closing::Real`, with the runs between brackets as `Run` items. No errors.

Every close is its group's own; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
       ^ valid       ^ invalid (the paren group, which is just the `(`)
```

Generates: the brace group with `Closing::Real`; among its children, the paren group with `Closing::Synthetic` and no children, so its span is the `(` alone. One error: `Unclosed` for the paren.

Reason: the `}` is strong evidence the author considers the brace section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and stage 4 keep working everywhere else in the group.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ invalid (paren section)
    ^ invalid (the `[` section, nested inside the paren section)
```

Generates: the brace group with `Closing::Real`; inside it the paren group, and inside that the `[` group, both `Closing::Synthetic` — the `[` group childless (its span is the `[` alone), the paren group ending at its last child, the `[` group. Two `Unclosed` errors, in source order of their openings.

Same reason as above, applied twice; nesting is preserved so a position resolves through the same ancestry the author typed.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
  ^ valid
      ^ invalid (the `)` alone)
        ^ valid
```

Generates: the brace group with `Closing::Real`, whose children are a `Run` item, a `StrayClose(Paren)`, and a `Run` item. One error: `UnexpectedClose`.

The `)` does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The stray-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ invalid (the `}` alone)
```

Generates: the paren group with `Closing::Real`, holding a `StrayClose(Brace)`. One error: `UnexpectedClose`.

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one invalid section.

## Case: crossing pairs

```
( { ) }
  ^ invalid (the brace group, which is just the `{`)
      ^ invalid (the trailing `}`, whose `{` was already consumed)
```

Generates: the paren group with `Closing::Real`, holding the brace group with `Closing::Synthetic` and no children (its span is the `{` alone); after the paren group, a top-level `StrayClose(Brace)`. Two errors: `Unclosed` for the brace group, then `UnexpectedClose` for the trailing `}`.

One crossing produces two invalid sections even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires looking past the `)` an unbounded distance, and any such rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single, left-to-right, one token ahead.

## Case: adjacent same-kind opens, one close

```
a {
  b {
    c
}
```

Generates: b's brace group with `Closing::Real`; a's brace group with `Closing::Synthetic`, its children kept and its span reaching its last child (b's group). One error: `Unclosed` for a's group.

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

The tree at the end of the tokens is settled: every group still open is closed synthetically, ending at its last child, its children kept and their nesting preserved, and each one is an `Unclosed` error. What is open is what `validity()` reports inside such a group. `resilient-parser.md` provisionally implements option A; deciding this question updates it.

The dominant real-world input is a literal being typed: the user has just written `{` and everything that follows is momentarily "after an unclosed open". Whatever we pick is the LSP experience during typing.

### Option A: invalid, like every synthetic close

```
field Query.Foo {
  id
  ^ invalid
```

One rule with no special case: `Closing::Synthetic` is invalid wherever the group ended. The cost: while the user types inside a new `{`, the entire rest of the literal is invalid, so stage 4 has nothing to say about the content most likely to be under the cursor.

### Option B: valid when the group's forced end is the end of the tokens

```
field Query.Foo {
  id
  ^ valid (inside the brace group)
```

Content stays valid while typing, nesting is already correct, nothing restructures when the real close is typed, and the missing brace is still reported: the `Unclosed` error exists either way, because errors are separate from validity. The cost: `validity()` special-cases where the group was forced to end, and a group that is valid while it reaches the end of the tokens flips invalid when a wrong-kind close later forces it shut mid-literal — a change of state from an edit made elsewhere.
