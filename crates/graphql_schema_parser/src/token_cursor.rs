use std::ops::Range;
use std::str::FromStr;

use intern::string_key::Intern;
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::{Expectation, Found, GraphQLTokenKind, ParseError};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    // Whole file. A Span plus `cursor.text()` at the call site copies 16 fewer bytes per consume.
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn text(self) -> &'a str {
        &self.text[self.location.as_usize_range()]
    }

    pub(crate) fn exclude_ends(self, n: u32) -> TokenText<'a> {
        TokenText {
            location: Span::new(self.location.start + n, self.location.end - n),
            text: self.text,
        }
    }

    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T> {
        self.text().intern().to::<T>().with_span(self.location)
    }

    /// A byte range inside `text()` as a span in the file.
    pub(crate) fn sub_span(self, range: Range<usize>) -> Span {
        Span::new(
            self.location.start + range.start as u32,
            self.location.start + range.end as u32,
        )
    }
}

/// A position in the token slice a `TokenCursor` reads.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenIndex(usize);

/// Sequential reader of a file's tokens. Parameter of a parse function.
pub(crate) struct TokenCursor<'a> {
    items: &'a [WithSpan<GraphQLTokenKind>],
    position: TokenIndex,
    /// The end of the last token this cursor advanced past (the first token's start before
    /// any). An `Expected(_, EndOfFile)` error uses this offset.
    previous_end: u32,
    text: &'a str,
}

impl<'a> TokenCursor<'a> {
    pub(crate) fn new(items: &'a [WithSpan<GraphQLTokenKind>], text: &'a str) -> Self {
        TokenCursor {
            items,
            position: TokenIndex(0),
            previous_end: match items.first() {
                Some(item) => item.location.start,
                None => 0,
            },
            text,
        }
    }

    pub(crate) fn peek(&self) -> Option<WithSpan<GraphQLTokenKind>> {
        self.peek_nth(0)
    }

    pub(crate) fn peek_nth(&self, n: usize) -> Option<WithSpan<GraphQLTokenKind>> {
        self.items.get(self.position.0 + n).copied()
    }

    pub(crate) fn at_end(&self) -> bool {
        self.peek().is_none()
    }

    pub(crate) fn position(&self) -> TokenIndex {
        self.position
    }

    /// The tokens from `start` up to the cursor; empty when `start` is not behind it.
    pub(crate) fn consumed_since(&self, start: TokenIndex) -> &'a [WithSpan<GraphQLTokenKind>] {
        self.items.get(start.0..self.position.0).unwrap_or_default()
    }

    /// From the start of the token at `start` to the end of the last consumed token;
    /// zero-width at `previous_end` when nothing was consumed since `start`.
    pub(crate) fn span_since(&self, start: TokenIndex) -> Span {
        match self.consumed_since(start).first() {
            Some(first) => Span::new(first.location.start, self.previous_end),
            None => Span::new(self.previous_end, self.previous_end),
        }
    }
    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn advance(&mut self) {
        if let Some(item) = self.peek() {
            self.previous_end = item.location.end;
            self.position = TokenIndex(self.position.0 + 1);
        }
    }

    pub(crate) fn consume_token_if(&mut self, kind: GraphQLTokenKind) -> Option<TokenText<'a>> {
        let item = self.peek().filter(|item| item.item == kind)?;
        self.advance();
        TokenText {
            location: item.location,
            text: self.text,
        }
        .wrap_some()
    }

    pub(crate) fn require_token(&mut self, kind: GraphQLTokenKind) -> Result<TokenText<'a>, ()> {
        self.consume_token_if(kind).ok_or(())
    }

    /// The next token as `K` when it is an identifier whose text parses as `K`. Consumes
    /// nothing.
    pub(crate) fn peek_identifier_of<K: FromStr>(&self) -> Option<WithSpan<K>> {
        let item = self
            .peek()
            .filter(|item| item.item == GraphQLTokenKind::Identifier)?;
        K::from_str(&self.text[item.location.as_usize_range()])
            .ok()
            .map(|keyword| keyword.with_span(item.location))
    }

    pub(crate) fn consume_identifier_of<K: FromStr>(&mut self) -> Option<WithSpan<K>> {
        let keyword = self.peek_identifier_of::<K>()?;
        self.advance();
        keyword.wrap_some()
    }

    pub(crate) fn consume_keyword_if(&mut self, word: &'static str) -> Option<Span> {
        let item = self.peek().filter(|item| {
            item.item == GraphQLTokenKind::Identifier
                && &self.text[item.location.as_usize_range()] == word
        })?;
        self.advance();
        item.location.wrap_some()
    }

    pub(crate) fn require_keyword(&mut self, word: &'static str) -> Result<Span, ()> {
        self.consume_keyword_if(word).ok_or(())
    }

    pub(crate) fn expected(&self, expected: Expectation) -> WithSpan<ParseError> {
        match self.peek() {
            None => ParseError::expected(expected, Found::EndOfFile)
                .with_span(Span::new(self.previous_end, self.previous_end)),
            Some(item) => {
                ParseError::expected(expected, Found::Token(item.item)).with_span(item.location)
            }
        }
    }

    pub(crate) fn spanning<T, E>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<WithSpan<T>, E> {
        let start = match self.peek() {
            Some(item) => item.location.start,
            None => self.previous_end,
        };
        let before = self.previous_end;
        let value = parse(self)?;
        let span = if self.previous_end == before {
            Span::new(before, before)
        } else {
            Span::new(start, self.previous_end)
        };
        value.with_span(span).wrap_ok()
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::StringLiteralValue;
    use prelude::Postfix;
    use span::Span;

    use crate::keyword::DefinitionKeyword;
    use crate::string_value::consume_string_value_if;
    use crate::test_cursor::with_cursor;
    use crate::{Expectation, Found, GraphQLTokenKind, ParseError};

    #[test]
    fn require_token_consumes_the_matching_token() {
        with_cursor("Pet", |cursor| {
            let name = cursor
                .require_token(GraphQLTokenKind::Identifier)
                .expect("the fixture is one identifier");
            assert_eq!(name.text(), "Pet");
            assert_eq!(name.location, Span::new(0, 3));
            assert!(cursor.at_end());
        });
    }

    #[test]
    fn a_failed_require_consumes_nothing_and_reports_the_found_token() {
        with_cursor("Pet", |cursor| {
            assert_eq!(cursor.require_token(GraphQLTokenKind::Colon), ().wrap_err());
            assert_eq!(
                cursor.peek().map(|token| token.item),
                GraphQLTokenKind::Identifier.wrap_some()
            );
            let error = cursor.expected(Expectation::Token(GraphQLTokenKind::Colon));
            assert_eq!(
                error.item.to_string(),
                "Expected colon (':'), found non-variable identifier (e.g. 'x' or 'Foo')."
            );
            assert_eq!(error.location, Span::new(0, 3));
        });
    }

    #[test]
    fn expected_at_the_end_of_the_file_is_zero_width_after_the_last_token() {
        with_cursor("", |cursor| {
            let error = cursor.expected(Expectation::TypeAnnotation);
            assert_eq!(
                error.item,
                ParseError::expected(Expectation::TypeAnnotation, Found::EndOfFile)
            );
            assert_eq!(error.location, Span::new(0, 0));
        });
        with_cursor("Pet ", |cursor| {
            cursor.advance();
            let error = cursor.expected(Expectation::TypeAnnotation);
            assert_eq!(
                error.item,
                ParseError::expected(Expectation::TypeAnnotation, Found::EndOfFile)
            );
            assert_eq!(error.location, Span::new(3, 3));
        });
    }

    #[test]
    fn consume_identifier_of_parses_the_keyword_or_consumes_nothing() {
        with_cursor("type", |cursor| {
            let keyword = cursor
                .consume_identifier_of::<DefinitionKeyword>()
                .expect("the fixture is the keyword type");
            assert_eq!(keyword.item, DefinitionKeyword::Type);
            assert_eq!(keyword.location, Span::new(0, 4));
        });
        with_cursor("typo", |cursor| {
            assert_eq!(cursor.consume_identifier_of::<DefinitionKeyword>(), None);
            assert_eq!(
                cursor.peek().map(|token| token.location),
                Span::new(0, 4).wrap_some()
            );
        });
    }

    #[test]
    fn consume_keyword_if_matches_the_whole_word() {
        with_cursor("extended", |cursor| {
            assert_eq!(cursor.consume_keyword_if("extend"), None);
            assert!(!cursor.at_end());
        });
    }

    #[test]
    fn spanning_covers_the_consumed_tokens() {
        with_cursor("a b", |cursor| {
            let span = cursor.spanning(|cursor| {
                cursor.require_token(GraphQLTokenKind::Identifier)?;
                cursor.require_token(GraphQLTokenKind::Identifier)
            });
            assert_eq!(span.map(|value| value.location), Span::new(0, 3).wrap_ok());
        });
    }

    #[test]
    fn spanning_that_consumes_nothing_is_zero_width_at_the_previous_end() {
        with_cursor("a", |cursor| {
            let span = cursor.spanning(|_| ().wrap_ok::<()>());
            assert_eq!(span.map(|value| value.location), Span::new(0, 0).wrap_ok());
        });
        with_cursor("a b", |cursor| {
            cursor.advance();
            let span = cursor.spanning(|_| ().wrap_ok::<()>());
            assert_eq!(span.map(|value| value.location), Span::new(1, 1).wrap_ok());
        });
    }

    #[test]
    fn consumed_since_and_span_since_cover_the_advanced_tokens() {
        with_cursor("a b c", |cursor| {
            let start = cursor.position();
            cursor.advance();
            cursor.advance();
            let consumed: Vec<_> = cursor
                .consumed_since(start)
                .iter()
                .map(|token| token.location)
                .collect();
            assert_eq!(consumed, vec![Span::new(0, 1), Span::new(2, 3)]);
            assert_eq!(cursor.span_since(start), Span::new(0, 3));
        });
    }

    #[test]
    fn consume_string_value_if_decodes_strings_and_block_strings() {
        with_cursor(r#""a\nb""#, |cursor| {
            let value = consume_string_value_if::<StringLiteralValue>(cursor)
                .expect("the fixture decodes")
                .expect("the fixture is a string");
            assert_eq!(value.item, "a\nb");
            assert_eq!(value.location, Span::new(0, 6));
        });
        with_cursor(r#""""  x""""#, |cursor| {
            let value = consume_string_value_if::<StringLiteralValue>(cursor)
                .expect("the fixture decodes")
                .expect("the fixture is a block string");
            assert_eq!(value.item, "  x");
        });
        with_cursor("x", |cursor| {
            assert_eq!(
                consume_string_value_if::<StringLiteralValue>(cursor),
                None.wrap_ok()
            );
            assert!(!cursor.at_end());
        });
        with_cursor(r#""\uDEAD""#, |cursor| {
            assert_eq!(
                consume_string_value_if::<StringLiteralValue>(cursor).map_err(|error| error.item),
                ParseError::InvalidUnicodeEscape.wrap_err()
            );
        });
    }

    #[test]
    fn expectations_and_errors_display_their_messages() {
        assert_eq!(
            Expectation::DefinitionOrExtensionKeyword.to_string(),
            "a definition (schema, scalar, type, interface, union, enum, input, or directive) or extend"
        );
        assert_eq!(Expectation::Keyword("on").to_string(), "the keyword `on`");
        assert_eq!(Found::EndOfFile.to_string(), "end of file");
        assert_eq!(
            ParseError::InvalidUnicodeEscape.to_string(),
            "This escape sequence does not encode a Unicode scalar value."
        );
    }
}
