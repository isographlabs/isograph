# Collapse the error token variants of `NonBracketTokenKind`

`NonBracketTokenKind` mirrors `IsographLangTokenKind` with the six bracket tokens
unrepresentable. Its six error variants — `ErrorUnterminatedString`,
`ErrorUnsupportedStringCharacter`, `ErrorUnterminatedBlockString`,
`ErrorNumberLiteralLeadingZero`, `ErrorNumberLiteralTrailingInvalid`,
`ErrorFloatLiteralMissingZero` — are collapsed into the single existing `Error`
variant. The `IsographLangTokenKind` lexer enum and its regexes are not touched:
the collapse happens entirely at the split boundary.

The parser does not carry the original text of a failed token, and it does not
distinguish one failed token from another. A found token that is any of these
reports as one thing: an invalid token.

## The type

Before:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
/// `IsographLangTokenKind` with the six bracket tokens unrepresentable: what the runs between
/// brackets hold once the bracket matcher has consumed the brackets. It has one variant per
/// non-bracket token, under the tokenizer's names.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error,
    ErrorUnterminatedString,
    ErrorUnsupportedStringCharacter,
    ErrorUnterminatedBlockString,
    At,
    Colon,
    Dollar,
    EndOfFile,
    Equals,
    Exclamation,
    Identifier,
    IntegerLiteral,
    LineBreak,
    ErrorNumberLiteralLeadingZero,
    ErrorNumberLiteralTrailingInvalid,
    ErrorFloatLiteralMissingZero,
    Period,
    Comma,
    StringLiteral,
    BlockStringLiteral,
}
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
/// `IsographLangTokenKind` with the six bracket tokens unrepresentable: what the runs between
/// brackets hold once the bracket matcher has consumed the brackets. Every malformed or
/// unlexable token is one variant: the parser does not carry its original text or
/// distinguish one from another.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error,
    At,
    Colon,
    Dollar,
    EndOfFile,
    Equals,
    Exclamation,
    Identifier,
    IntegerLiteral,
    LineBreak,
    Period,
    Comma,
    StringLiteral,
    BlockStringLiteral,
}
```

The doc comment's dropped lines claimed one variant per tokenizer token under its
name; the fold makes that false, so the comment names the fold instead. No comment
in the crate claimed the variants kept the original text.

## The forward split

The seven `IsographLangTokenKind` error kinds — `#[error]`'s `Error` plus the six
named ones — all land on `NonBracketTokenKind::Error`. The forward `From` joins
them into one arm.

Before:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::Error => SplitToken::NonBracket(NonBracketTokenKind::Error),
            IsographLangTokenKind::ErrorUnterminatedString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedString)
            }
            IsographLangTokenKind::ErrorUnsupportedStringCharacter => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnsupportedStringCharacter)
            }
            IsographLangTokenKind::ErrorUnterminatedBlockString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedBlockString)
            }
            ...
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralLeadingZero)
            }
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid)
            }
            IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorFloatLiteralMissingZero)
            }
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
            IsographLangTokenKind::Error
            | IsographLangTokenKind::ErrorUnterminatedString
            | IsographLangTokenKind::ErrorUnsupportedStringCharacter
            | IsographLangTokenKind::ErrorUnterminatedBlockString
            | IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            | IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            | IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::Error)
            }
```

The other arms of the forward `From` are unchanged.

## The reverse split and the display

`impl From<NonBracketTokenKind> for IsographLangTokenKind` exists only to feed the
`Display for NonBracketTokenKind` delegation, plus one round-trip test. It is not
a bijection after the collapse: seven `IsographLangTokenKind` variants share one
`NonBracketTokenKind::Error`, so a single return value cannot invert them. It is
deleted.

`Display for NonBracketTokenKind` is the live found-token surface: `Found::Token`
is `#[error("{0}")]`, and the parser reports a mismatched token through it. It
matches `NonBracketTokenKind` directly and gives the error the vague message the
change calls for, instead of delegating back to the granularity we just
discarded.

Before:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
impl From<NonBracketTokenKind> for IsographLangTokenKind {
    fn from(kind: NonBracketTokenKind) -> Self {
        match kind {
            NonBracketTokenKind::Error => IsographLangTokenKind::Error,
            NonBracketTokenKind::ErrorUnterminatedString => {
                IsographLangTokenKind::ErrorUnterminatedString
            }
            NonBracketTokenKind::ErrorUnsupportedStringCharacter => {
                IsographLangTokenKind::ErrorUnsupportedStringCharacter
            }
            NonBracketTokenKind::ErrorUnterminatedBlockString => {
                IsographLangTokenKind::ErrorUnterminatedBlockString
            }
            NonBracketTokenKind::At => IsographLangTokenKind::At,
            ...
            NonBracketTokenKind::ErrorNumberLiteralLeadingZero => {
                IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            }
            NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid => {
                IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            }
            NonBracketTokenKind::ErrorFloatLiteralMissingZero => {
                IsographLangTokenKind::ErrorFloatLiteralMissingZero
            }
            NonBracketTokenKind::Period => IsographLangTokenKind::Period,
            NonBracketTokenKind::Comma => IsographLangTokenKind::Comma,
            NonBracketTokenKind::StringLiteral => {
                IsographLangTokenKind::StringLiteral
            }
            NonBracketTokenKind::BlockStringLiteral => {
                IsographLangTokenKind::BlockStringLiteral
            }
        }
    }
}

impl fmt::Display for NonBracketTokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        IsographLangTokenKind::from(*self).fmt(f)
    }
}
```

After: the reverse `From` is gone; the display matches `Error` to a vague message
and the remaining arms carry the messages the forward side already produced
(verbatim from `IsographLangTokenKind`'s own `Display`, the one that no one
reads).

```rust
// from crates/isograph_parser/src/non_bracket_token.rs
impl fmt::Display for NonBracketTokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            NonBracketTokenKind::Error => "an invalid token",
            NonBracketTokenKind::At => "at symbol ('@')",
            NonBracketTokenKind::Colon => "colon (':')",
            NonBracketTokenKind::Dollar => "dollar ('$')",
            NonBracketTokenKind::EndOfFile => "end of file",
            NonBracketTokenKind::Equals => "equals ('=')",
            NonBracketTokenKind::Exclamation => "exclamation mark ('!')",
            NonBracketTokenKind::Identifier => {
                "non-variable identifier (e.g. 'x' or 'Foo')"
            }
            NonBracketTokenKind::IntegerLiteral => "integer value (e.g. '0' or '42')",
            NonBracketTokenKind::LineBreak => "line break",
            NonBracketTokenKind::Period => "period ('.')",
            NonBracketTokenKind::Comma => "comma (',')",
            NonBracketTokenKind::StringLiteral => "string literal (e.g. '\"...\"')",
            NonBracketTokenKind::BlockStringLiteral => "block string (e.g. '\"\"\"hi\"\"\"')",
        };
        f.write_str(message)
    }
}
```

`IsographLangTokenKind` and its `Display` in `token_kind.rs` stay as they are.
The collapse is at the split, so the lexer keeps all seven error variants; its
match stays exhaustive and raises no warning. Nothing displays an
`IsographLangTokenKind`, so its table is not kept in step with
`NonBracketTokenKind`'s.

The `use crate::IsographLangTokenKind;` at the top of `non_bracket_token.rs`
stays: the forward `From` and the `SplitToken` type still name it.

## The found-token surface

`Found::Token(NonBracketTokenKind)` and `Expectation::Token(NonBracketTokenKind)`
in `parse_error.rs` are unchanged: they hold a `NonBracketTokenKind`, and its
`Error` now renders as "an invalid token" where it once rendered "unterminated
string" and the like.

## The grammar test

`parse_iso_literal.rs` names one collapsed variant, in the "numeric where an
identifier was expected" case. The import loses `ErrorNumberLiteralTrailingInvalid`
and the assertion uses `Error`.

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use NonBracketTokenKind::{
        At, Comma, Dollar, ErrorNumberLiteralTrailingInvalid, Identifier, Period,
    };
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (mod tests)
        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(
                token(Identifier),
                Found::Token(ErrorNumberLiteralTrailingInvalid),
            ),
            span_of(numeric, "42."),
        );
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    use NonBracketTokenKind::{At, Comma, Dollar, Error, Identifier, Period};
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (mod tests)
        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(
                token(Identifier),
                Found::Token(Error),
            ),
            span_of(numeric, "42."),
        );
```

The import drops `ErrorNumberLiteralTrailingInvalid` and takes `Error` in its
place; the names stay alphabetical.

## The split test

The round-trip test inverted the split through the deleted reverse `From`, so it
is replaced by two forward tests: the non-error kinds split to themselves, and the
seven error kinds fold to `Error`.

Before:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs (mod tests)
    fn non_bracket_token_round_trips_through_the_split() {
        for kind in [
            IsographLangTokenKind::At,
            IsographLangTokenKind::Identifier,
            IsographLangTokenKind::StringLiteral,
            IsographLangTokenKind::Error,
        ] {
            match SplitToken::from(kind) {
                SplitToken::NonBracket(non_bracket) => {
                    assert_eq!(IsographLangTokenKind::from(non_bracket), kind);
                }
                SplitToken::Bracket(bracket) => {
                    panic!("{kind:?} split as a bracket: {bracket:?}")
                }
            }
        }
    }
```

After:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs (mod tests)
    #[test]
    fn non_error_tokens_split_to_their_own_kind() {
        for (kind, expected) in [
            (IsographLangTokenKind::At, NonBracketTokenKind::At),
            (IsographLangTokenKind::Identifier, NonBracketTokenKind::Identifier),
            (IsographLangTokenKind::StringLiteral, NonBracketTokenKind::StringLiteral),
            (IsographLangTokenKind::IntegerLiteral, NonBracketTokenKind::IntegerLiteral),
        ] {
            assert_eq!(SplitToken::from(kind), SplitToken::NonBracket(expected));
        }
    }

    #[test]
    fn every_error_token_folds_to_one_variant() {
        for kind in [
            IsographLangTokenKind::Error,
            IsographLangTokenKind::ErrorUnterminatedString,
            IsographLangTokenKind::ErrorUnsupportedStringCharacter,
            IsographLangTokenKind::ErrorUnterminatedBlockString,
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero,
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid,
            IsographLangTokenKind::ErrorFloatLiteralMissingZero,
        ] {
            assert_eq!(
                SplitToken::from(kind),
                SplitToken::NonBracket(NonBracketTokenKind::Error)
            );
        }
    }
```

The test module's import gains `NonBracketTokenKind`:

```rust
// from crates/isograph_parser/src/non_bracket_token.rs (mod tests)
    use super::{BracketKind, BracketToken, NonBracketTokenKind, SplitToken};
```

## Docs this change amends

`refactors/pending/semantic-tokens.md` lists all six `NonBracketTokenKind::Error*`
variants in the `from_non_bracket` match (the `=> None` group with `Comma`,
`LineBreak`, `EndOfFile`, `Error`). When that doc lands, the six named variants do
not exist: the group reads `NonBracketTokenKind::Comma | NonBracketTokenKind::LineBreak
| NonBracketTokenKind::EndOfFile | NonBracketTokenKind::Error => None`. The
`from_non_bracket` arm for the single `Error` already returns `None`, so the fold
needs no behavior change there beyond dropping the six names.

## Landing checklist

1. `non_bracket_token.rs`: drop the six variants; fold the forward `From` arms;
   delete the reverse `From`; match `Display` directly. `cargo test -p
   isograph_parser` passes.
2. `parse_iso_literal.rs`: drop the import name; assert `Found::Token(Error)`.
3. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
4. Note the `semantic-tokens.md` delta there; leave that doc otherwise as it is.
5. Move this doc to refactors/past.
