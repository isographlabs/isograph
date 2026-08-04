# Bracket matching: the cases and why

The behavior of stage 2 of `resilient-parser.md`, pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass, one token of lookahead — the discipline the existing parser's peekable lexer already sets — and no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

The rule:

- `()`, `{}`, and `[]` are all matched.
- An open bracket begins a group. The group's children are parsed until the literal ends or a close bracket that this group or an enclosing one owns appears. The group then consumes that close if it is its own (`closing: Some`), and otherwise ends without one (`closing: None`), which is what makes it an invalid section.
- Seen from the close's side, the same rule reads: a close bracket pairs with the nearest open bracket of its kind, and open brackets of other kinds above that one end as invalid sections just before the close.
- A close bracket whose kind is open nowhere is a `StrayClose` where it stands: an invalid section one character wide. It consumes nothing.
- Invalidity never spreads outward: not to siblings, not to the enclosing group. A position is in an invalid section iff the node it resolves to, or an ancestor, is a `StrayClose` or a group with `closing: None`.
- Text outside any bracket is a valid section by itself.

## The tree

What stage 2 generates (`resilient-parser.md`'s Change 1 implements exactly this):

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

/// An open bracket, everything up to its close, and the close if it ever arrived. The
/// enclosing `WithSpan`'s span runs from the start of the opening to the end of the closing,
/// or to the end of the children when there is none.
pub struct Bracketed {
    pub opening: WithSpan<Bracket>,
    /// The matching close. `None` is what makes the group an invalid section.
    pub closing: Option<Span>,
    pub children: Vec<BracketItem>,
}

pub enum Bracket {
    Paren,
    Curly,
    Square,
}
```

A matched group and an unmatched one are one variant: a group that never got its close is `closing: None`, not a different node, so position resolution and stage 3 walk one shape and unmatchedness is a field to look at, not a case to remember. The stray close is its own variant because it is neither text nor a group: it has no opening and no children, and folding it into `Bracketed` would put an `Option` on `opening` beside the one on `closing`, making a both-`None` item representable that means nothing.

Positions marked below use `^` under the character; `valid`/`invalid` states what `validity()` returns there.

## Case: text outside any bracket

```
field Query.Foo
      ^ valid
```

Generates: one `Text` item covering the whole literal.

Unbracketed text cannot be malformed at this stage, so it is a valid section on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ valid      ^ valid
```

Generates: `Bracketed` items nested as typed, every one `closing: Some`, with the runs between brackets as `Text` items.

Every close is its group's own; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
       ^ valid        ^
                 (paren section: invalid)
```

Generates: the curly group with `closing: Some`; among its children, the paren group with `closing: None`, ending just before the `}`.

Reason: the `}` is strong evidence the author considers the curly section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and stage 3 keep working everywhere else in the group.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ invalid (paren section)
    ^ invalid (square section, nested inside the paren section)
```

Generates: the curly group with `closing: Some`; inside it the paren group with `closing: None`, whose children hold the square group with `closing: None`.

Same reason as above, applied twice; nesting is preserved so a position resolves through the same ancestry the author typed.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
  ^ valid
      ^ invalid (the `)` alone)
        ^ valid
```

Generates: the curly group with `closing: Some`, whose children are a `Text`, a `StrayClose(Paren)`, and a `Text`.

The `)` does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The stray-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ invalid (the `}` alone)
```

Generates: the paren group with `closing: Some`, holding a `StrayClose(Curly)`.

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one invalid section.

## Case: crossing pairs

```
( { ) }
  ^ invalid (curly section, ends before the `)`)
      ^ invalid (the trailing `}`, whose `{` was already consumed)
```

Generates: the paren group with `closing: Some`, holding the curly group with `closing: None`; after it, a top-level `StrayClose(Curly)`.

One crossing produces two invalid sections even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires looking past the `)` an unbounded distance, and any such rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single, left-to-right, one token ahead.

## Case: adjacent same-kind opens, one close

```
a {
  b {
    c
}
```

Generates: b's curly group with `closing: Some`; a's curly group with `closing: None`, reaching the end of the literal (the open question below).

The `}` pairs with the nearest `{` (b's), because same-kind matching is always nearest-first: nesting is the common intent, and "nearest of its kind" is the rule everywhere else.

## Open question: the end of the literal

What happens to brackets still open when the literal ends. `resilient-parser.md` provisionally implements option A; deciding this doc's question updates both docs.

The dominant real-world input is a literal being typed: the user has just written `{` and everything that follows is momentarily "after an unclosed open". Whatever we pick is the LSP experience during typing.

### Option A: unmatched to the end

Every still-open group is `closing: None` with its span running from its opening to the end of the literal, nested.

```
field Query.Foo {
  id
  ^ invalid
```

Simple, and symmetric with every other unclosed-group case. The cost: while the user types inside a new `{`, the entire rest of the literal is invalid, so stage 3 has nothing to say about the content most likely to be under the cursor.

### Option B: only the open bracket is invalid

The unclosed open becomes a `Bracketed` with `closing: None` and no children; its would-be children are matched as if it were absent, attached to the enclosing level.

```
field Query.Foo {
  id
  ^ valid (but at the top level, not inside any group)
```

Content stays valid while typing. The cost: the tree lies about nesting — `id` reads as a sibling of `field Query.Foo` rather than a child — and the whole tree restructures the moment the close is typed. "As if it were absent" also breaks the symmetry with mid-literal recovery, where an unclosed open keeps its children.

### Option C: the end of the literal closes everything

Still-open groups count as closed by the end of the literal; their content is valid.

```
field Query.Foo {
  id
  ^ valid (inside the curly group)
```

Correct nesting and valid content while typing, and no restructuring when the real close arrives. The cost: `closing` must either hold a fabricated zero-width span no one typed, or stay `None` with validity special-casing groups that reach the end of the literal; either way an unclosed group at the end and a finished one become hard to tell apart, and a genuinely forgotten close brace at the end of a finished literal is reported by nothing at this stage.

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

The scanner learns double-quoted strings with backslash escapes, and brackets inside them are text. This adds one piece of scanner state and one new end-of-literal question (an unterminated string swallows every bracket after it, which is option-A-of-the-previous-section behavior applied to quotes).
