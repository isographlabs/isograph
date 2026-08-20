# Future improvements

These are not in the grammar-stage order. They are language-shape and leftover lexer grammar. Do not mix them into type-annotation-null.md, parse-iso-literal-entry.md, or parser-minor-improvements.md.

## Line break and comma are the same chunk separator

These tests pin the language. They are not accidents:

- `field Query.Foo\n{ bar }` is a field with no selection set plus `MultipleDeclarations` on the brace. `a_selection_set_on_its_own_line_is_a_second_declaration` in `parse_iso_literal.rs`.
- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`. `a_directive_on_the_next_line_is_its_own_failed_selection`.
- `[Pet\n!]` does not attach the bang to `Pet`. `a_line_break_inside_a_list_type_does_not_attach_bang`.

Spaces do not split chunks. Newlines do. Anyone who formats a selection set or a `to` clause onto the next line gets a second declaration.

The diagnostic for `field Query.Foo\n{ bar }` is `Expected nothing after the declaration. Each literal holds exactly one declaration.` If line breaks remain chunk separators, the diagnostic names the line-break rule: `expected the selection set on the same line`.

## `#` comments

`#` comments are an `Error` token for `#` plus identifiers for the rest of the line. There is no comment token. The skip regex skips only `[ \t\f\ufeff]+`.

```rust
// from crates/isograph_parser/src/token_kind.rs
    #[regex(r"[ \t\f\ufeff]+", logos::skip)]
    #[error]
    Error,
```

A comment token is a language-shape change. It is not mixed into leftover-semantic-tokens.md leftover fill-in.

## Commented-out grammar in `token_kind.rs`

Spread, comments, `Pipe`, and `PeriodPeriod` sit as comments in the token enum, plus `TODO don't skip comments and spaces`. The crate rule is that a comment must not describe what was not done. Display has the matching commented-out arms (`Ampersand`, `PeriodPeriod`, `Pipe`, `Spread`).

```rust
// from crates/isograph_parser/src/token_kind.rs
    // #[token("..")]
    // PeriodPeriod,

    // #[token("|")]
    // Pipe,

    // #[token("...")]
    // Spread,

    // Comments
    // #[regex("#[^\n\r]*")]
    // SingleLineComment,
```

Delete those commented-out variants, the TODO on the skip regex, and the commented-out Display arms.
