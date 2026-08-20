# Future improvements

Not in the grammar-stage order. Language-shape and leftover lexer grammar. Do not mix these into type-annotation-null.md, parse-iso-literal-entry.md, or parser-minor-improvements.md.

## Line break and comma are the same chunk separator

These are tests, not accidents:

- `field Query.Foo\n{ bar }` is a field with no selection set plus `MultipleDeclarations` on the brace.
- `bar\n{ baz }` inside a set is a scalar plus a failed selection.
- `bar\n@loadable` is a selection plus a failed selection on `@`.
- `[Pet\n!]` does not attach the bang to `Pet`.

Spaces do not split. Newlines do. Anyone who formats a selection set or a `to` clause onto the next line gets a second declaration. If that is the language, the diagnostic should say so (`expected the selection set on the same line`). It currently says `Expected nothing after the declaration`.

## `#` comments

`#` comments are `Error` plus identifiers. There is no comment token. The skip regex skips only `[ \t\f\ufeff]+`.

## Commented-out grammar in `token_kind.rs`

Spread, comments, `Pipe`, `PeriodPeriod` sit as comments, plus `TODO don't skip comments and spaces`. The crate rule is that a comment must not describe what was not done. `observe_kinds` is the same residue in test form. Display has the matching commented-out arms (`Ampersand`, `PeriodPeriod`, `Pipe`, `Spread`, `Empty`).
