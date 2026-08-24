# Future improvements

These are not in the grammar-stage order. They are language-shape and leftover lexer grammar. Do not mix them into type-annotation-null.md, parse-iso-literal-entry.md, or parser-minor-improvements.md.

## Line break and comma are the same chunk separator

These tests pin the language. They are not accidents:

- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`. `a_directive_on_the_next_line_is_its_own_failed_selection`.
- `[Pet\n!]` does not attach the bang to `Pet`. `a_line_break_inside_a_list_type_does_not_attach_bang`.

Spaces do not split chunks. Newlines do.

## `#` comments

comments.md. A comment token is a language-shape change. It is not mixed into leftover-semantic-tokens.md leftover fill-in.

## Commented-out grammar in `token_kind.rs`

Spread, `Pipe`, and `PeriodPeriod` sit as comments in the token enum, plus `TODO don't skip comments and spaces`. `SingleLineComment` is comments.md (that regex is uncommented there, not deleted). The crate rule is that a comment must not describe what was not done.

```rust
// from crates/isograph_parser/src/token_kind.rs
    // #[token("..")]
    // PeriodPeriod,

    // #[token("|")]
    // Pipe,

    // #[token("...")]
    // Spread,
```

Delete those commented-out variants and the TODO on the skip regex. The commented-out `SingleLineComment` regex is comments.md, not this deletion. `token_kind.rs` uses `strum::Display`; there are no handwritten Display arms.
