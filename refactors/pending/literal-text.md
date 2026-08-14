# literal-text: the only reader of the literal's text

A prefactor to the parsing series: the grammar stage's one gate onto the literal's text, landed ahead of the series so parse-entrypoint.md carries only grammar. Its first production caller is parse-entrypoint.md's keyword dispatch, the next doc to land; shipping one doc ahead of that caller is a deliberate exception to parsing-standards.md's ship-with-first-caller rule, made because the type couples to nothing undecided (no error types, no chunk types, only `Span` and the text).

## The module

```rust
// from crates/isograph_parser/src/literal_text.rs
use span::Span;

/// The literal's text, admitting only the reads the grammar performs. Raw slicing is
/// unavailable outside this impl.
pub(crate) struct LiteralText<'a>(&'a str);

impl<'a> LiteralText<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        LiteralText(text)
    }

    /// The text of an identifier token, for keyword dispatch by string match. The span
    /// is one an identifier-accepting stream method returned.
    pub(crate) fn identifier(&self, span: Span) -> &'a str {
        &self.0[span.as_usize_range()]
    }
}
```

lib.rs registers the module (`mod literal_text;`) without re-exporting it: the type is `pub(crate)` and stays so. parse-arguments.md amends the impl with `integer` (`None` on out of range); no other read exists, and string-literal contents and every other span stay unreadable.

## Tests

```rust
// from crates/isograph_parser/src/literal_text.rs
#[cfg(test)]
mod tests {
    use super::LiteralText;
    use span::Span;

    #[test]
    fn identifier_returns_the_spanned_text() {
        let text = LiteralText::new("entrypoint Query.foo");
        assert_eq!(text.identifier(Span::new(0, 10)), "entrypoint");
        assert_eq!(text.identifier(Span::new(11, 16)), "Query");
        assert_eq!(text.identifier(Span::new(17, 20)), "foo");
    }
}
```

## Landing checklist

1. The module, its lib.rs registration, and the test; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
