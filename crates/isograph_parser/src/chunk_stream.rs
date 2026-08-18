use nonempty::NonEmpty;
use prelude::Postfix;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan, WithSpanPostfix};

use crate::{
    BracketKind, ChunkContentItem, ChunkedGroup, Expectation, Found, NonBracketTokenKind,
    ParseError,
};

/// Sequential reader of one chunk. Parameter of a parse function.
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<nonempty::Iter<'a, WithSpan<ChunkContentItem>>>,
    /// The end of the last item this cursor advanced past (the chunk's start before
    /// any). An `Expected(_, EndOfChunk)` error uses this offset.
    previous_end: u32,
    text: &'a str,
}

/// Sequential reader of one chunk, plus `require_end`.
pub(crate) struct ChunkStream<'a>(ItemCursor<'a>);

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(contents: &'a NonEmpty<WithSpan<ChunkContentItem>>, text: &'a str) -> Self {
        ChunkStream(ItemCursor {
            previous_end: contents.first().location.start,
            items: contents.iter().safe_peekable(),
            text,
        })
    }

    pub(crate) fn cursor(&mut self) -> &mut ItemCursor<'a> {
        &mut self.0
    }

    #[allow(dead_code)]
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
    pub(crate) fn consume_token_if(&mut self, kind: NonBracketTokenKind) -> Option<Span> {
        let peek = self.items.peek()?;
        match peek.view().item.reference() {
            ChunkContentItem::NonBracket(token) if token.0 == kind => {}
            _ => return None,
        }
        let item = peek.commit();
        self.previous_end = item.location.end;
        item.location.wrap_some()
    }

    #[allow(dead_code)]
    pub(crate) fn consume_group_if(
        &mut self,
        kind: BracketKind,
    ) -> Option<WithSpan<&'a ChunkedGroup>> {
        let peek = self.items.peek()?;
        let item = *peek.view();
        match item.item.reference() {
            ChunkContentItem::Group(group) if group.opening.item.0 == kind => {
                peek.commit();
                self.previous_end = item.location.end;
                group.with_span(item.location).wrap_some()
            }
            _ => None,
        }
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

    pub(crate) fn require_token(&mut self, kind: NonBracketTokenKind) -> Result<Span, ()> {
        self.consume_token_if(kind).ok_or(())
    }

    #[allow(dead_code)]
    pub(crate) fn require_group(
        &mut self,
        kind: BracketKind,
    ) -> Result<WithSpan<&'a ChunkedGroup>, ()> {
        self.consume_group_if(kind).ok_or(())
    }

    #[allow(dead_code)]
    pub(crate) fn text(&self) -> &'a str {
        self.text
    }

    pub(crate) fn token_text(&self, span: Span) -> &'a str {
        &self.text[span.as_usize_range()]
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

#[cfg(test)]
mod tests {
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::ChunkStream;
    use crate::{
        BracketKind, Chunk, ChunkedLevel, Expectation, Found, NonBracketTokenKind, ParseError,
        chunk, match_brackets, tokenize,
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

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
    }

    fn stream_of<'a>(tree: &'a WithSpan<ChunkedLevel>, text: &'a str) -> ChunkStream<'a> {
        first_chunk(tree).stream(text)
    }

    #[test]
    fn consume_token_if_matches_the_next_identifier_and_skips_the_wrong_kind() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier),
            span_of(text, "foo").wrap_some(),
        );
        assert_eq!(cursor.consume_token_if(NonBracketTokenKind::Period), None);
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier),
            span_of(text, "bar").wrap_some(),
        );
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier),
            None
        );
    }

    #[test]
    fn consume_group_if_matches_a_brace_group_and_skips_a_token_or_the_wrong_bracket() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        let cursor = stream.cursor();
        assert_eq!(cursor.consume_group_if(BracketKind::Brace), None);
        assert_eq!(
            cursor.consume_token_if(NonBracketTokenKind::Identifier),
            span_of(text, "foo").wrap_some(),
        );
        assert_eq!(cursor.consume_group_if(BracketKind::Parenthesis), None);
        let group = cursor
            .consume_group_if(BracketKind::Brace)
            .expect("the next item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(group.item.opening.item.0, BracketKind::Brace);
        assert_eq!(cursor.consume_group_if(BracketKind::Brace), None);
    }

    #[test]
    fn require_token_and_require_group_are_consume_or_err() {
        let text = "{ bar } foo";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        let cursor = stream.cursor();
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier),
            ().wrap_err(),
        );
        let group = cursor
            .require_group(BracketKind::Brace)
            .expect("the first item is a brace group");
        assert_eq!(group.location, span_of(text, "{ bar }"));
        assert_eq!(cursor.require_group(BracketKind::Brace), ().wrap_err(),);
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier),
            span_of(text, "foo").wrap_ok(),
        );
        assert_eq!(
            cursor.require_token(NonBracketTokenKind::Identifier),
            ().wrap_err(),
        );
    }

    #[test]
    fn expected_names_the_next_item_or_end_of_chunk() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
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
            .consume_token_if(NonBracketTokenKind::Identifier)
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
            .consume_group_if(BracketKind::Brace)
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
        let mut stream = stream_of(tree.reference(), text);
        let cursor = stream.cursor();
        cursor
            .consume_token_if(NonBracketTokenKind::Identifier)
            .expect("foo is present");
        assert_eq!(
            cursor.expected(Expectation::Separator),
            expected(Expectation::Separator, Found::EndOfChunk).with_span(Span::new(
                span_of(text, "foo").end,
                span_of(text, "foo").end,
            )),
        );
    }

    #[test]
    fn require_end_is_ok_only_on_an_empty_remainder() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier)
            .expect("foo is present");
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier)
            .expect("bar is present");
        assert_eq!(stream.require_end(), ().wrap_ok());
    }

    #[test]
    fn remaining_contents_is_none_at_end_and_clones_the_unread_items() {
        let text = "foo bar baz";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        assert_eq!(
            stream.remaining_contents().map(|items| items.len()),
            3.wrap_some()
        );
        let mut stream = stream_of(tree.reference(), text);
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier)
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
        let mut stream = stream_of(tree.reference(), text);
        let spanned = stream
            .cursor()
            .spanning(|cursor| {
                cursor
                    .require_token(NonBracketTokenKind::Identifier)
                    .map_err(|()| cursor.expected(token(NonBracketTokenKind::Identifier)))?;
                cursor
                    .require_token(NonBracketTokenKind::Identifier)
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
        let mut stream = stream_of(tree.reference(), text);
        stream
            .cursor()
            .consume_token_if(NonBracketTokenKind::Identifier)
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
        let mut stream = stream_of(tree.reference(), text);
        let error = stream
            .cursor()
            .spanning::<()>(|cursor| {
                cursor
                    .require_token(NonBracketTokenKind::Period)
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
                .consume_token_if(NonBracketTokenKind::Identifier),
            span_of(text, "foo").wrap_some(),
        );
    }

    #[test]
    fn token_text_is_the_source_slice() {
        let text = "foo bar";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        let cursor = stream.cursor();
        let foo = cursor
            .consume_token_if(NonBracketTokenKind::Identifier)
            .expect("foo is present");
        assert_eq!(cursor.token_text(foo), "foo");
        assert_eq!(cursor.text(), text);
    }

    #[test]
    fn a_lone_group_is_the_whole_chunk() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut stream = stream_of(tree.reference(), text);
        assert_eq!(stream.require_end(), ().wrap_err());
        stream
            .cursor()
            .require_group(BracketKind::Brace)
            .expect("the chunk is a brace group");
        assert_eq!(stream.require_end(), ().wrap_ok());
        assert_eq!(stream.remaining_contents(), None);
    }
}
