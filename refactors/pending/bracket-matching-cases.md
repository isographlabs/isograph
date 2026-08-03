# Bracket matching: the cases and why

The behavior of stage 2 of `resilient-parser.md`, pattern by pattern, with the reason for each. The governing goal: the rule must be easy to reason about. A single left-to-right pass, no lookahead, no heuristics; we accept a worse tree on a rare edge case to keep every case predictable from the rule alone.

The rule:

- `()`, `{}`, and `[]` are all matched.
- A close bracket pairs with the nearest open bracket of its kind. Open brackets of other kinds above that one become invalid sections, ending just before the close.
- A close bracket whose kind is open nowhere is itself an invalid section, one character wide. It consumes nothing.
- Invalidity never spreads outward: not to siblings, not to the enclosing group. A position is in an invalid section iff the node it resolves to, or an ancestor, is unmatched.
- Text outside any bracket is a valid section by itself.

Positions marked below use `^` under the character; `valid`/`invalid` states what `validity()` returns there.

## Case: text outside any bracket

```
field Query.Foo
      ^ valid
```

Unbracketed text cannot be malformed at this stage, so it is a valid section on its own. This is what keeps a literal useful while it is mostly prose and the user has not typed a bracket yet.

## Case: balanced, mixed kinds

```
field Query.Foo { bar(arg: [1, 2]) { id } }
                        ^ valid      ^ valid
```

Every close finds its kind at the top of the stack; the rule degenerates to ordinary matching. The recovery machinery costs nothing on well-formed input.

## Case: wrong-kind close with a same-kind open below

```
field Query.Foo { bar( }
       ^ valid        ^
                 (paren section: invalid)
```

The `}` pairs with `{`; the open `(` above it becomes an invalid section ending just before the `}`. The curly group is valid; only the paren section is not.

Reason: the `}` is strong evidence the author considers the curly section finished. Blaming the one bracket that provably never got its partner confines the damage to it, so hover, completion, and stage 3 keep working everywhere else in the group.

## Case: several wrong-kind opens

```
{ ( [ }
  ^ invalid (paren section)
    ^ invalid (square section, nested inside the paren section)
```

The `}` ends `[` and then `(` as invalid sections, nested in the order they were opened, and pairs with `{`. Same reason as above, applied twice; nesting is preserved so a position resolves through the same ancestry the author typed.

## Case: close of a kind that is open nowhere

```
{ foo ) bar }
  ^ valid
      ^ invalid (the `)` alone)
        ^ valid
```

The `)` is a one-character invalid section. It does not end the `{` group and does not consume anything.

Reason: consuming an open of a different kind would destroy a pair that may still complete. The stray-close rule is what makes this work:

```
( } )
^ matched pair ^
  ^ invalid (the `}` alone)
```

If the `}` had ended the `(` group, the `)` that was coming would have become a second error. One typo, one invalid section.

## Case: crossing pairs

```
( { ) }
  ^ invalid (curly section, ends before the `)`)
      ^ invalid (the trailing `}`, whose `{` was already consumed)
```

The `)` pairs with `(` and ends the `{` as an invalid section; the later `}` then finds no open curly and is a stray close. One crossing produces two invalid sections even though a smarter matcher could have paired `{` with `}`.

Reason to accept this: pairing them requires lookahead past the `)`, and any lookahead rule reintroduces the hard-to-predict behavior this design exists to avoid. Crossing brackets are rare in real literals; the pass stays single and left-to-right.

## Case: adjacent same-kind opens, one close

```
a {
  b {
    c
}
```

The `}` pairs with the nearest `{` (b's), because same-kind matching is always nearest-first: nesting is the common intent, and "nearest of its kind" is the rule everywhere else. That leaves a's `{` open at the end of the literal, which is the next case.

## Open question: the end of the literal

What happens to brackets still open when the literal ends. `resilient-parser.md` provisionally implements option A; deciding this doc's question updates both docs.

The dominant real-world input is a literal being typed: the user has just written `{` and everything that follows is momentarily "after an unclosed open". Whatever we pick is the LSP experience during typing.

### Option A: unmatched to the end

Every still-open bracket becomes an invalid section from its open to the end of the literal, nested.

```
field Query.Foo {
  id
  ^ invalid
```

Simple, and symmetric with every other unmatched-open case. The cost: while the user types inside a new `{`, the entire rest of the literal is invalid, so stage 3 has nothing to say about the content most likely to be under the cursor.

### Option B: only the open bracket is invalid

The unclosed open becomes a one-character invalid section (like a stray close); its would-be children are matched as if it were absent, attached to the enclosing level.

```
field Query.Foo {
  id
  ^ valid (but at the top level, not inside any group)
```

Content stays valid while typing. The cost: the tree lies about nesting — `id` reads as a sibling of `field Query.Foo` rather than a child — and the whole tree restructures the moment the close is typed. "As if it were absent" also breaks the symmetry with mid-literal recovery, where an unclosed open keeps its children.

### Option C: the end of the literal closes everything

Still-open brackets are matched by an implied zero-width close at the end; the groups count as valid.

```
field Query.Foo {
  id
  ^ valid (inside the curly group)
```

Correct nesting and valid content while typing, and no restructuring when the real close arrives. The cost: an unclosed bracket at the end is no longer representable as an error, so `MatchedGroup.close` would need to admit an absent close (an `Option`, or a third item variant), and a genuinely forgotten close brace at the end of a finished literal is reported by nothing at this stage.

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
