# Bracket matching: the cases and why

The behavior of the bracket matcher (`raw-items.md`), pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass over the tokenizer's output, one token of lookahead — the discipline the existing parser's peekable lexer already sets — and no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

The tokenizer feeds the matcher, and everything else runs after it, on its output: every group the matcher hands downstream has a real opening and a real closing, and a group's interior is the same type as the root. A close bracket with no open of its kind is a raw item where it stands. When a group never gets its close, the group is taken apart: its opening becomes a raw item, and its children move into the enclosing level, matched groups among them surviving. Each pass owns its own errors: the `UnmatchedOpen` and `UnmatchedClose` errors below are the matcher's; the tokenizer's `Error*` kinds ride through as raw tokens, and the chunk-parsing pass reports leftover bracket tokens it finds inside chunks.

The rule:

- `()`, `{}`, and `[]` are all matched.
- An open bracket begins a group. The group's children are parsed until the tokens end or a close bracket that this group or an enclosing one owns appears.
- The group consumes that close if it is its own: a real `CloseBracket` on the group. Otherwise the group never closed: its opening becomes a raw `OpenBracket` at the enclosing level, and its children move into that level as siblings.
- Seen from the close's side, the same rule reads: a close bracket pairs with the nearest open bracket of its kind, and open brackets of other kinds above that one come apart before it, innermost first.
- A close bracket whose kind is open nowhere is a raw `CloseBracket` where it stands. It consumes nothing.
- Unmatchedness never spreads outward: not to siblings, not to the enclosing group. An unmatched token is the token itself, one item wide.
- Non-bracket tokens outside any structure are raw items by themselves.

## The tree

What the matcher generates (`raw-items.md` implements exactly this). `BracketKind` (parenthesis `()`, brace `{}`, bracket `[]`) and `NonBracketTokenKind` are landed code from the tokenizer's split layer:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
/// One level: the whole literal at the root, a group's interior below.
pub struct MatchedBrackets(pub Vec<WithSpan<BracketItem>>);

pub enum BracketItem {
    Raw(RawToken),
    Bracketed(Bracketed),
}

pub struct Bracketed {
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    pub children: WithSpan<MatchedBrackets>,
    pub closing: WithSpan<CloseBracket>,
}

/// A token that is not part of any structure; matched brackets are structure, never
/// raw, so a bracket token here is unmatched.
pub enum RawToken {
    NonBracket(NonBracketToken),
    Open(OpenBracket),
    Close(CloseBracket),
}

pub struct NonBracketToken(pub NonBracketTokenKind);
pub struct OpenBracket(pub BracketKind);
pub struct CloseBracket(pub BracketKind);
```

Every `Bracketed` has a real opening and a real closing, required fields. A group's interior is the same type as the root — `WithSpan<MatchedBrackets>` in both positions, the root's span being the whole literal — so no level is special. Matched brackets are structure; unmatched brackets are the same token types sitting raw in a level, so a consumer reads matched or unmatched from the path (or from whether the token is raw vs a group's own opening/closing).

The matcher's errors are derived from the tree, in source order:

```rust
// from crates/isograph_parser/src/matched_brackets.rs
#[derive(Debug, PartialEq, Eq)]
pub enum BracketError {
    UnmatchedOpen(WithSpan<OpenBracket>),
    UnmatchedClose(WithSpan<CloseBracket>),
}

impl MatchedBrackets {
    /// Every unmatched bracket under this level, in source order.
    pub fn errors(&self) -> Vec<BracketError>;
}
```

Positions marked below use `^` under the character. A position is on an unmatched token when the node it resolves to is an `OpenBracket` or `CloseBracket` whose parent is `Unmatched`; the pass ships no collapsed answer, so a consumer reads this off the path. A position on whitespace the tokenizer skipped resolves to the enclosing level.

## Case: text outside any bracket

```
field Query.Foo
      ^ raw non-bracket items
```

Generates: four raw `NonBracket` items (`Identifier`, `Identifier`, `Period`, `Identifier`). No errors.

An unbracketed token cannot be malformed at this stage, so it is a raw item on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ matched      ^ matched
```

Generates: `Bracketed` items nested as typed, every opening and closing real, with the non-bracket tokens between them as raw items. No errors.

Every close is its group's own; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
                     ^ unmatched open (the `(` alone, demoted to raw)
```

Generates: the brace group with its real closing; among its children, a raw `OpenBracket(Parenthesis)` for the `(`. One error: `UnmatchedOpen` for the parenthesis.

Reason: the `}` is strong evidence the author considers the brace section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and later passes keep working everywhere else in the group. Taking the group apart (rather than forcing it shut with a `None` closing) means content after the open sits at the enclosing level, not inside an unbalanced group that never closed.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ unmatched open
    ^ unmatched open
```

Generates: the brace group with its real closing; inside it two raw `OpenBracket` items for `(` and `[`, in source order. Two `UnmatchedOpen` errors.

Same reason as above, applied twice. Groups that never closed do not nest as unbalanced structure; they come apart into raw opens at the level that owns the close.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
      ^ unmatched close (the `)` alone)
```

Generates: the brace group with its real closing, whose children are raw non-bracket items around a raw `CloseBracket(Parenthesis)`. One error: `UnmatchedClose`.

The `)` does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The unmatched-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ unmatched close (the `}` alone)
```

Generates: the parenthesis group with its real closing, holding a raw `CloseBracket(Brace)`. One error: `UnmatchedClose`.

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one unmatched token.

## Case: crossing pairs

```
( { ) }
  ^ unmatched open (the `{`)
      ^ unmatched close (the trailing `}`, whose `{` was already demoted)
```

Generates: the parenthesis group with its real closing, holding a raw `OpenBracket(Brace)` for the `{`; after the parenthesis group, a top-level raw `CloseBracket(Brace)`. Two errors: `UnmatchedOpen` for the brace open, then `UnmatchedClose` for the trailing `}`.

One crossing produces two unmatched tokens even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires looking past the `)` an unbounded distance, and any such rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single, left-to-right, one token ahead.

A related shape, `foo { (}) }`, takes the paren group apart inside the brace, then the brace also comes apart when the first `}` is consumed as the brace's own close and the leftover `)` and `}` sit as raw closes at the top:

```
foo { (}) }
      ^ unmatched open
       ^ unmatched close (paren)
         ^ unmatched close (brace)
```

## Case: adjacent same-kind opens, one close

```
a { b { c }
```

Generates: b's brace group with its real closing, holding the raw token `c`; a's open becomes a raw `OpenBracket` at the top level, with `b` as a sibling raw token between the two opens. One error: `UnmatchedOpen` for a's group.

The `}` pairs with the nearest `{` (b's), because same-kind matching is always nearest-first: nesting is the common intent, and "nearest of its kind" is the rule everywhere else. Content after an unmatched open sits in the enclosing level; there is no unbalanced group left in the tree.

## Brackets inside strings

Resolved by running the matcher over the tokenizer's output: the tokenizer lexes string and block-string literals whole, so a bracket inside a string is part of a `StringLiteral` token and never structural.

```
{ name: "a}" }
          ^ raw non-bracket (inside a StringLiteral token, in the brace group's interior)
             ^ this closes the brace group
```

A malformed string lexes as whatever the tokenizer produces for it (an `Error` raw token); that is an inner error for a later pass to report, and the matcher just sees a non-bracket token.

## End of the tokens

Every group still open at the end of the tokens comes apart: each opening becomes a raw `OpenBracket` at the enclosing level, and children move up. There is no open question about validity of content after an unmatched open: that content sits in the enclosing level as ordinary siblings, and the unmatched open is the one error token.

When a close is present that an outer group owns, only the inner unclosed groups come apart; the outer group matches as usual:

```
foo { bar(a: }
         ^ unmatched open (the `(` alone)
             ^ this closes the brace group
```

Generates: the brace group with its real closing; among its children, raw `bar`, a raw `OpenBracket(Parenthesis)`, then raw `a` and `:`. One error: `UnmatchedOpen` for the parenthesis.
