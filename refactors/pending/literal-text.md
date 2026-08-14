# literal-text: the only reader of the literal's text

A prefactor to the parsing series: the grammar stage's one gate onto the literal's text, landed ahead of the series so parse-entrypoint.md carries only grammar. Its first production caller is parse-entrypoint.md's keyword dispatch, the next doc to land; shipping one doc ahead of that caller is a deliberate exception to parsing-standards.md's ship-with-first-caller rule, made because the type couples to nothing undecided (no error types, no chunk types, only `Span` and the text).

The method is parse-entrypoint.md's free function `token_text`, moved onto the type unchanged; the delta this doc exists for is the gate alone, that the raw `&str` is unreachable outside this impl. parse-entrypoint.md's revision deletes its free function in favor of this method.

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

    /// The literal text a span covers. The parser reads it only to recognize keywords.
    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.0[span.as_usize_range()]
    }
}
```

lib.rs registers the module (`mod literal_text;`) without re-exporting it: the type is `pub(crate)` and stays so. Until its caller lands, the impl carries `#[expect(dead_code)]`; parse-entrypoint.md's implementation removes the attribute, and the `expect` (unlike `allow`) fails the build if it is ever redundant, so it cannot outlive its reason. parse-arguments.md amends the impl with `integer` (`None` on out of range); no other read exists, and string-literal contents and every other span stay unreadable.

## Tests

```rust
// from crates/isograph_parser/src/literal_text.rs
#[cfg(test)]
mod tests {
    use super::LiteralText;
    use span::Span;

    #[test]
    fn token_text_returns_the_spanned_text() {
        let text = LiteralText::new("entrypoint Query.foo");
        assert_eq!(text.token_text(Span::new(0, 10)), "entrypoint");
        assert_eq!(text.token_text(Span::new(11, 16)), "Query");
        assert_eq!(text.token_text(Span::new(17, 20)), "foo");
    }
}
```

## Landing checklist

1. The module, its lib.rs registration, and the test; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
