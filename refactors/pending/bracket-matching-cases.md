# Bracket matching: the cases and why

The behavior of stage 2 of `resilient-parser.md`, pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass, one token of lookahead — the discipline the existing parser's peekable lexer already sets — and no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

Bracket matching is the first pass, always, and its output is balanced: every group it hands downstream has a close, real or synthesized, so no later pass ever sees an unclosed bracket. Each pass owns its own errors: the `UnexpectedClose` and `Unclosed` errors below are this pass's, and stage 3 produces its own, separate error tokens over the balanced tree. There will be other such passes.

The rule:

- `()`, `{}`, and `[]` are all matched.
- An open bracket begins a group. The group's children are parsed until the literal ends or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: `Closing::Real`. Otherwise the group is closed synthetically the moment the close it cannot match (or the end of the literal) is encountered — a zero-width close at that position, `Closing::Synthetic` — which makes it an invalid section and an `Unclosed` error. Groups between a close and the group that owns it all end this way, innermost first.
- Seen from the close's side, the same rule reads: a close bracket pairs with the nearest open bracket of its kind, and open brackets of other kinds above that one are synthetically closed just before it.
- A close bracket whose kind is open nowhere is a `StrayClose` where it stands: an invalid section one character wide and an `UnexpectedClose` error. It consumes nothing.
- Invalidity never spreads outward: not to siblings, not to the enclosing group. A position is in an invalid section iff the node it resolves to, or an ancestor, is a `StrayClose` or a synthetically closed group.
- Text outside any bracket is a valid section by itself.

## The tree

What the pass generates (`resilient-parser.md`'s Change 1 implements exactly this):

```rust
pub struct BracketTree {
    pub items: Vec<BracketItem>,
}

pub enum BracketItem {
    Text(WithSpan<Text>),
    Bracketed(WithSpan<Bracketed>),
    /// A close bracket no open of its kind was waiting for: an invalid section one character
    /// wide.
    StrayClose(WithSpan<Bracket>),
}

/// A maximal run of text containing no brackets. Its span lives on the enclosing `WithSpan`.
pub struct Text;

/// An open bracket, everything up to its close, and the close — always present, so every pass
/// after this one works with guaranteed matching brackets. The enclosing `WithSpan`'s span
/// runs from the start of the opening to the end of the closing.
pub struct Bracketed {
    pub opening: WithSpan<Bracket>,
    pub closing: Closing,
    pub children: Vec<BracketItem>,
}

pub enum Closing {
    /// The close bracket the author typed.
    Real(Span),
    /// A zero-width close at the position the group was forced to end: just before the close
    /// bracket an enclosing group owns, or at the end of the literal. What makes the group an
    /// invalid section.
    Synthetic(Span),
}

pub enum Bracket {
    Paren,
    Curly,
    Square,
}
```

A matched group and an unmatched one are one variant: unmatchedness is `Closing::Synthetic`, not a different node, so position resolution and stage 3 walk one shape. The stray close is its own variant because it is neither text nor a group: it has no opening and no children, and folding it into `Bracketed` would make an item with neither bracket representable.

The pass's errors are derived from the tree, in source order:

```rust
pub enum BracketError {
    /// A close bracket no open of its kind was waiting for.
    UnexpectedClose(WithSpan<Bracket>),
    /// A group whose close was synthesized.
    Unclosed(UnclosedGroup),
}

pub struct UnclosedGroup {
    pub opening: WithSpan<Bracket>,
    /// The zero-width close synthesized where the group was forced to end.
    pub synthetic_close: Span,
}

impl BracketTree {
    /// Empty iff every bracket matched.
    pub fn errors(&self) -> Vec<BracketError>;
}
```

Positions marked below use `^` under the character; `valid`/`invalid` states what `validity()` returns there.

## Case: text outside any bracket

```
field Query.Foo
      ^ valid
```

Generates: one `Text` item covering the whole literal. No errors.

Unbracketed text cannot be malformed at this stage, so it is a valid section on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ valid      ^ valid
```

Generates: `Bracketed` items nested as typed, every one `Closing::Real`, with the runs between brackets as `Text` items. No errors.

Every close is its group's own; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
       ^ valid        ^
                 (paren section: invalid)
```

Generates: the curly group with `Closing::Real`; among its children, the paren group with `Closing::Synthetic` zero-width just before the `}`. One error: `Unclosed` for the paren.

Reason: the `}` is strong evidence the author considers the curly section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and stage 3 keep working everywhere else in the group.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ invalid (paren section)
    ^ invalid (square section, nested inside the paren section)
```

Generates: the curly group with `Closing::Real`; inside it the paren group, and inside that the square group, both `Closing::Synthetic` zero-width just before the `}`. Two `Unclosed` errors, in source order of their openings.

Same reason as above, applied twice; nesting is preserved so a position resolves through the same ancestry the author typed.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
  ^ valid
      ^ invalid (the `)` alone)
        ^ valid
```

Generates: the curly group with `Closing::Real`, whose children are a `Text`, a `StrayClose(Paren)`, and a `Text`. One error: `UnexpectedClose`.

The `)` does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The stray-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ invalid (the `}` alone)
```

Generates: the paren group with `Closing::Real`, holding a `StrayClose(Curly)`. One error: `UnexpectedClose`.

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one invalid section.

## Case: crossing pairs

```
( { ) }
  ^ invalid (curly section, ends before the `)`)
      ^ invalid (the trailing `}`, whose `{` was already consumed)
```

Generates: the paren group with `Closing::Real`, holding the curly group with `Closing::Synthetic` zero-width just before the `)`; after it, a top-level `StrayClose(Curly)`. Two errors: `Unclosed` for the curly, then `UnexpectedClose` for the trailing `}`.

One crossing produces two invalid sections even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires looking past the `)` an unbounded distance, and any such rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single, left-to-right, one token ahead.

## Case: adjacent same-kind opens, one close

```
a {
  b {
    c
}
```

Generates: b's curly group with `Closing::Real`; a's curly group with `Closing::Synthetic` zero-width at the end of the literal, its children kept. One error: `Unclosed` for a's group.

The `}` pairs with the nearest `{` (b's), because same-kind matching is always nearest-first: nesting is the common intent, and "nearest of its kind" is the rule everywhere else.

## Open question: validity at the end of the literal

The tree at the end of the literal is settled: every group still open is closed synthetically at the end, its children kept and their nesting preserved, and each one is an `Unclosed` error. What is open is what `validity()` reports inside such a group. `resilient-parser.md` provisionally implements option A; deciding this question updates it.

The dominant real-world input is a literal being typed: the user has just written `{` and everything that follows is momentarily "after an unclosed open". Whatever we pick is the LSP experience during typing.

### Option A: invalid, like every synthetic close

```
field Query.Foo {
  id
  ^ invalid
```

One rule with no special case: `Closing::Synthetic` is invalid wherever it sits. The cost: while the user types inside a new `{`, the entire rest of the literal is invalid, so stage 3 has nothing to say about the content most likely to be under the cursor.

### Option B: valid when the synthetic close sits at the end of the literal

```
field Query.Foo {
  id
  ^ valid (inside the curly group)
```

Content stays valid while typing, nesting is already correct, nothing restructures when the real close is typed, and the missing brace is still reported: the `Unclosed` error exists either way, because errors are separate from validity. The cost: `validity()` special-cases where the synthetic close sits, and a group that is valid while it touches the end of the literal flips invalid when a wrong-kind close later forces it shut mid-literal — a change of state from an edit made elsewhere.

## Open question: brackets inside strings

The scanner treats every bracket as structural, including inside quoted strings:

```
{ name: "a}" }
          ^ this ends the curly group
             ^ stray close, invalid
```

### Option A: accept it

String arguments containing brackets are rare in isograph literals today. No scanner state, no new cases.

### Option B: skip quoted regions

The scanner learns double-quoted strings with backslash escapes, and brackets inside them are text. This adds one piece of scanner state and one new end-of-literal question (an unterminated string swallows every bracket after it, which is the previous section's question applied to quotes).
