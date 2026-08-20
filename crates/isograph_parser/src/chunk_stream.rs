use intern::string_key::Intern;
use nonempty::NonEmpty;
use prelude::Postfix;
use safe_peekable::{IntoSafePeekable, Peek, SafePeekable};
use span::{Span, WithSpan, WithSpanPostfix};

use crate::{
    BracketKind, Chunk, ChunkContentItem, ChunkedLevel, Expectation, ExtraChunks, Found,
    NonBracketTokenKind, ParseError, SemanticToken, Singleton, Slot, UnparsedChunkItems,
    parse_singleton,
};

/// Sequential reader of one chunk. Parameter of a parse function.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last item this cursor advanced past (the chunk's start before
    /// any). An `Expected(_, EndOfChunk)` error uses this offset.
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    errors: &'a mut Vec<WithSpan<ParseError>>,
}

pub(crate) struct CursorPeek<'c, 'a> {
    peek: Peek<'c, &'a WithSpan<ChunkContentItem>>,
    previous_end: &'c mut u32,
    tokens: &'c mut Vec<WithSpan<SemanticToken>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) struct TokenText<'a> {
    pub location: Span,
    // Whole literal. A Span plus `cursor.text()` at the call site copies 16 fewer bytes per consume.
    text: &'a str,
}

impl<'a> TokenText<'a> {
    pub(crate) fn text(self) -> &'a str {
        &self.text[self.location.as_usize_range()]
    }

    pub(crate) fn interned<T: From<intern::string_key::StringKey>>(self) -> WithSpan<T> {
        self.text().intern().to::<T>().with_span(self.location)
    }
}

/// Records `token` at `closing` when dropped, however the parse function exits, a panic included.
struct RecordGroupClose<'c, 'a> {
    cursor: &'c mut ItemCursor<'a>,
    closing: Span,
    token: SemanticToken,
}

impl<'c, 'a> RecordGroupClose<'c, 'a> {
    fn cursor(&mut self) -> &mut ItemCursor<'a> {
        self.cursor
    }
}

impl Drop for RecordGroupClose<'_, '_> {
    fn drop(&mut self) {
        self.cursor.record(self.token, self.closing);
    }
}

/// Sequential reader of one chunk, plus `require_end`.
pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a NonEmpty<WithSpan<ChunkContentItem>>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
            tokens,
            errors,
        })
    }

    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a> {
        &mut self.0
    }

    #[cfg_attr(not(test), expect(dead_code))]
    pub(crate) fn require_end(&mut self) -> Result<(), ()> {
        self.0.items.peek().map_or(().wrap_ok(), |_| ().wrap_err())
    }

    /// Unread content items. `None` when the cursor is at end.
    pub(crate) fn remaining_contents(&mut self) -> Option<NonEmpty<WithSpan<ChunkContentItem>>> {
        let first = self.0.items.next()?;
        let mut tail = Vec::new();
        for item in self.0.items.by_ref() {
            tail.push(item.clone());
        }
        NonEmpty {
            head: first.clone(),
            tail,
        }
        .wrap_some()
    }
}

impl<'a> ItemCursor<'a> {
    pub(crate) fn peek(&mut self) -> Option<CursorPeek<'_, 'a>> {
        CursorPeek {
            peek: self.items.peek()?,
            previous_end: &mut self.previous_end,
            tokens: self.tokens,
        }
        .wrap_some()
    }

    pub(crate) fn consume_token_if(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Option<TokenText<'a>> {
        let peek = self.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(found) if found.0 == kind => {}
            _ => return None,
        }
        let location = peek.commit(token).location;
        TokenText {
            location,
            text: self.text,
        }
        .wrap_some()
    }

    pub(crate) fn consume_group_if<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Option<WithSpan<R>> {
        let peek = self.peek()?;
        let item = peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                let location = item.location;
                let closing = group.closing.location;
                let children = group.children.reference();
                peek.commit(token);
                let mut close = RecordGroupClose {
                    cursor: self,
                    closing,
                    token,
                };
                parse_inside(close.cursor(), children)
                    .with_span(location)
                    .wrap_some()
            }
            _ => None,
        }
    }

    pub(crate) fn report_error(&mut self, error: WithSpan<ParseError>) {
        self.errors.push(error);
    }

    pub(crate) fn stream_chunk<'c>(&'c mut self, chunk: &'c Chunk) -> ChunkStream<'c> {
        chunk.stream(self.text, self.tokens, self.errors)
    }

    fn record(&mut self, token: SemanticToken, span: Span) {
        self.tokens.push(token.with_span(span));
    }

    pub(crate) fn expected(&mut self, expected: Expectation) -> WithSpan<ParseError> {
        match self.items.peek() {
            None => ParseError::expected(expected, Found::EndOfChunk).with_span(self.end_span()),
            Some(peek) => {
                let item = peek.view();
                ParseError::expected(expected, Found::from(item.item.reference()))
                    .with_span(item.location)
            }
        }
    }

    pub(crate) fn require_token(
        &mut self,
        kind: NonBracketTokenKind,
        token: SemanticToken,
    ) -> Result<TokenText<'a>, ()> {
        self.consume_token_if(kind, token).ok_or(())
    }

    pub(crate) fn require_group<R>(
        &mut self,
        kind: BracketKind,
        token: SemanticToken,
        parse_inside: impl FnOnce(&mut Self, &'a WithSpan<ChunkedLevel>) -> R,
    ) -> Result<WithSpan<R>, ()> {
        self.consume_group_if(kind, token, parse_inside).ok_or(())
    }

    pub(crate) fn parse_nested_singleton<T>(
        &mut self,
        level: &WithSpan<ChunkedLevel>,
        end: Expectation,
        extra_chunks: impl FnOnce(&WithSpan<Chunk>) -> WithSpan<ParseError>,
        parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
    ) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks> {
        parse_singleton(
            level,
            self.text,
            self.tokens,
            self.errors,
            end,
            extra_chunks,
            parse,
        )
    }

    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn spanning<T>(
        &mut self,
        parse: impl FnOnce(&mut Self) -> Result<T, WithSpan<ParseError>>,
    ) -> Result<WithSpan<T>, WithSpan<ParseError>> {
        let start = match self.items.peek() {
            Some(peek) => peek.view().location.start,
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

    fn end_span(&self) -> Span {
        Span::new(self.previous_end, self.previous_end)
    }
}

impl<'c, 'a> CursorPeek<'c, 'a> {
    pub(crate) fn view(&self) -> &'a WithSpan<ChunkContentItem> {
        self.peek.view().dereference()
    }

    pub(crate) fn commit(self, token: SemanticToken) -> &'a WithSpan<ChunkContentItem> {
        let item = self.peek.commit();
        *self.previous_end = item.location.end;
        let span = match item.item.reference() {
            ChunkContentItem::NonBracket(_) => item.location,
            ChunkContentItem::Group(group) => group.opening.location,
        };
        self.tokens.push(token.with_span(span));
        item
    }
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{ChunkStream, TokenText};
    use crate::{
        BracketKind, Chunk, ChunkedLevel, Expectation, Found, NonBracketTokenKind, ParseError,
        SemanticToken, chunk, match_brackets, tokenize,
    };

    fn chunked(text: &str) -> WithSpan<ChunkedLevel> {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        tree
    }

    fn first_chunk(tree: &WithSpan<ChunkedLevel>) -> &Chunk {
        tree.item.0[0].item.reference()
    }

    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    fn token_text<'a>(text: &'a str, pattern: &str) -> TokenText<'a> {
        TokenText {
            location: span_of(text, pattern),
            text,
        }
    }

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
    }

    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> ChunkStream<'a> {
        first_chunk(tree).stream(text, tokens, errors)
    }

    #[test]
    fn consume_token_if_matches_the_next_identifier_and_skips_the_wrong_kind() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Period, SemanticToken::Period),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "bar").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            None
        );
    }

    #[test]
    fn consume_group_if_matches_a_brace_group_and_skips_a_token_or_the_wrong_bracket() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            None
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
        assert_eq!(
            cursor.consume_group_if(
                BracketKind::Parenthesis,
                SemanticToken::Parenthesis,
                |_, _| ()
            ),
            None
        );
        let group = cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the next item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(
            cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            None
        );
    }

    #[test]
    fn require_token_and_require_group_are_consume_or_err() {
        let text = "{ bar } foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            ().wrap_err(),
        );
        let group = cursor
            .require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the first item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(
            cursor.require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
            ().wrap_err(),
        );
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_ok(),
        );
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            ().wrap_err(),
        );
    }

    #[test]
    fn expected_names_the_next_item_or_end_of_chunk() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.expected(token(NonBracketTokenKind::Period)),
            expected(
                token(NonBracketTokenKind::Period),
                Found::Token(NonBracketTokenKind::Identifier),
            )
            .with_span(span_of(text, "foo")),
        );
        cursor
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(
            cursor.expected(token(NonBracketTokenKind::Identifier)),
            expected(
                token(NonBracketTokenKind::Identifier),
                Found::Group(BracketKind::Brace),
            )
            .with_span(span_of(text, "{ bar }")),
        );
        cursor
            .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the group is present");
        let group_end = span_of(text, "{ bar }").end;
        assert_eq!(
            cursor.expected(token(NonBracketTokenKind::Identifier)),
            expected(token(NonBracketTokenKind::Identifier), Found::EndOfChunk)
                .with_span(Span::new(group_end, group_end)),
        );
    }

    #[test]
    fn expected_at_the_start_of_an_unconsumed_chunk_uses_the_first_item_start() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        cursor
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(
            cursor.expected(Expectation::Separator(BracketKind::Parenthesis)),
            expected(
                Expectation::Separator(BracketKind::Parenthesis),
                Found::EndOfChunk,
            )
            .with_span(Span::new(
                span_of(text, "foo").end,
                span_of(text, "foo").end,
            )),
        );
    }

    #[test]
    fn require_end_is_ok_only_on_an_empty_remainder() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("bar is present");
        assert_eq!(stream.require_end(), ().wrap_ok());
    }

    #[test]
    fn remaining_contents_is_none_at_end_and_clones_the_unread_items() {
        let text = "foo bar baz";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        assert_eq!(
            stream.remaining_contents().map(|items| items.len()),
            3.wrap_some()
        );
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        match stream.remaining_contents() {
            Some(remaining) => {
                assert_eq!(remaining.len(), 2);
                assert_eq!(remaining.first().location, span_of(text, "bar"));
                assert_eq!(remaining.last().location, span_of(text, "baz"));
            }
            None => panic!("expected unread items after foo"),
        }
        assert_eq!(stream.remaining_contents(), None);
    }

    #[test]
    fn spanning_covers_what_the_closure_advanced_past() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let spanned = stream
            .cursor()
            .spanning(|cursor| {
                cursor
                    .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                    .map_err(|()| cursor.expected(token(NonBracketTokenKind::Identifier)))?;
                cursor
                    .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                    .map_err(|()| cursor.expected(token(NonBracketTokenKind::Identifier)))?;
                ().wrap_ok()
            })
            .expect("both identifiers are present");
        assert_eq!(
            spanned.location,
            Span::join(span_of(text, "foo"), span_of(text, "bar")),
        );
    }

    #[test]
    fn spanning_on_an_empty_cursor_is_zero_width_at_previous_end() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        let spanned = stream
            .cursor()
            .spanning(|_cursor| ().wrap_ok())
            .expect("an empty spanning succeeds");
        let foo_end = span_of(text, "foo").end;
        assert_eq!(spanned.location, Span::new(foo_end, foo_end));
    }

    #[test]
    fn spanning_does_not_advance_when_the_closure_errors_without_consuming() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let error = stream
            .cursor()
            .spanning::<()>(|cursor| {
                cursor
                    .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
                    .map_err(|()| cursor.expected(token(NonBracketTokenKind::Period)))?;
                ().wrap_ok()
            })
            .expect_err("period is not present");
        assert_eq!(
            error,
            expected(
                token(NonBracketTokenKind::Period),
                Found::Token(NonBracketTokenKind::Identifier),
            )
            .with_span(span_of(text, "foo")),
        );
        assert_eq!(
            stream
                .cursor()
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
            token_text(text, "foo").wrap_some(),
        );
    }

    #[test]
    fn token_text_is_the_source_slice() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        let foo = cursor
            .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
            .expect("foo is present");
        assert_eq!(foo.text(), "foo");
        assert_eq!(foo.location, span_of(text, "foo"));
        assert_eq!(cursor.text(), text);
    }

    #[test]
    fn a_lone_group_is_the_whole_chunk() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
            .expect("the chunk is a brace group");
        assert_eq!(stream.require_end(), ().wrap_ok());
        assert_eq!(stream.remaining_contents(), None);
    }

    #[test]
    fn require_token_records_the_role_the_caller_passed() {
        for (text, token) in [
            ("entrypoint", SemanticToken::Keyword),
            ("Query", SemanticToken::Type),
            ("foo", SemanticToken::FieldName),
            ("id", SemanticToken::ObjectKey),
            ("Foo", SemanticToken::GraphQLTypeName),
        ] {
            let tree = chunked(text);
            let mut tokens = Vec::new();
            let mut errors = Vec::new();
            {
                let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
                assert_eq!(
                    stream
                        .cursor()
                        .require_token(NonBracketTokenKind::Identifier, token),
                    token_text(text, text).wrap_ok(),
                    "for literal {text:?}",
                );
            }
            assert_eq!(tokens, token.with_span(span_of(text, text)).wrap_vec());
        }
    }

    #[test]
    fn require_token_records_nothing_when_the_kind_does_not_match() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(
                stream
                    .cursor()
                    .require_token(NonBracketTokenKind::Period, SemanticToken::Period),
                ().wrap_err(),
            );
        }
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn consume_token_if_records_nothing_when_the_kind_does_not_match() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(
                stream
                    .cursor()
                    .consume_token_if(NonBracketTokenKind::Period, SemanticToken::Period),
                None,
            );
        }
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn alias_colon_name_records_field_name_colon_field_name() {
        let text = "alias: name";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            let cursor = stream.cursor();
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                token_text(text, "alias").wrap_some(),
            );
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon),
                token_text(text, ":").wrap_some(),
            );
            assert_eq!(
                cursor.consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                token_text(text, "name").wrap_some(),
            );
        }
        assert_eq!(
            tokens,
            vec![
                SemanticToken::FieldName.with_span(span_of(text, "alias")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::FieldName.with_span(span_of(text, "name")),
            ],
        );
    }

    #[test]
    fn consume_group_if_records_the_open_and_the_close() {
        for text in ["{ bar }", "{}"] {
            let tree = chunked(text);
            let mut tokens = Vec::new();
            let mut errors = Vec::new();
            {
                let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
                stream
                    .cursor()
                    .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
                    .expect("the chunk is a brace group");
            }
            assert_eq!(
                tokens,
                vec![
                    SemanticToken::Brace.with_span(span_of(text, "{")),
                    SemanticToken::Brace.with_span(span_of(text, "}")),
                ],
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn require_group_err_records_nothing() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(
                stream
                    .cursor()
                    .require_group(BracketKind::Brace, SemanticToken::Brace, |_, _| ()),
                ().wrap_err(),
            );
        }
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn expected_does_not_record() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            stream.cursor().expected(token(NonBracketTokenKind::Period));
        }
        assert_eq!(tokens, vec![]);
    }

    #[test]
    fn peek_without_commit_records_nothing() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            let peek = stream.cursor().peek().expect("foo is present");
            assert_eq!(peek.view().location, span_of(text, "foo"));
        }
        assert_eq!(tokens, vec![]);
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(
                stream
                    .cursor()
                    .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName),
                token_text(text, "foo").wrap_some(),
            );
        }
        assert_eq!(
            tokens,
            SemanticToken::FieldName
                .with_span(span_of(text, "foo"))
                .wrap_vec(),
        );
    }

    #[test]
    fn report_error_appends_to_the_vec() {
        let text = "foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let error = ParseError::EmptyLiteral.with_span(span_of(text, "foo"));
        stream.cursor().report_error(error);
        assert_eq!(errors, error.wrap_vec());
        assert_eq!(tokens, vec![]);
    }
}
