use crate::IsographLangTokenKind;
use common_lang_types::{
    Diagnostic, DiagnosticResult, Location, Span, TextSource, WithSpan, WithSpanPostfix,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{IsographSemanticToken, semantic_token_legend};
use logos::Logos;
use prelude::Postfix;

pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    /// the byte offset of the *end* of the previous token
    end_index_of_last_parsed_token: u32,
    offset: u32,
    semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,

    pub text_source: TextSource,
}

impl<'source> PeekableLexer<'source> {
    pub fn new(source: &'source str, text_source: TextSource) -> Self {
        // To enable fast lookahead the parser needs to store at least the 'kind' (IsographLangTokenKind)
        // of the next token: the simplest option is to store the full current token, but
        // the Parser requires an initial value. Rather than incur runtime/code overhead
        // of dealing with an Option or UnsafeCell, the constructor uses a dummy token
        // value to construct the Parser, then immediately advance()s to move to the
        // first real token.
        let lexer = IsographLangTokenKind::lexer(source);
        let dummy = IsographLangTokenKind::EndOfFile.with_generated_span();

        let mut parser = PeekableLexer {
            current: dummy,
            lexer,
            source,
            end_index_of_last_parsed_token: 0,
            offset: 0,
            semantic_tokens: vec![],

            text_source,
        };

        // Advance to the first real token before doing any work
        parser.parse_token(semantic_token_legend::ST_COMMENT);
        // When parsing, we placed a semantic token on the stack.
        parser.semantic_tokens.pop();
        parser
    }

    /// Get the next token (and advance)
    fn parse_token(
        &mut self,
        isograph_semantic_token: IsographSemanticToken,
    ) -> WithSpan<IsographLangTokenKind> {
        let kind = self
            .lexer
            .next()
            .unwrap_or(IsographLangTokenKind::EndOfFile);

        self.end_index_of_last_parsed_token = self.current.span.end;
        // TODO this seems buggy — the span is inclusive on both sides,
        // but it should be exclusive on the right, so we should probably
        // do self.end_index_of_last_parsed_token = self.current.span.end + 1
        // or something!
        let span = self.lexer_span();

        // TODO why does self.current = ... not work here?
        let parsed_token = std::mem::replace(&mut self.current, kind.with_span(span));

        self.semantic_tokens
            .push(isograph_semantic_token.with_span(parsed_token.span));

        parsed_token
    }

    pub fn peek(&self) -> WithSpan<IsographLangTokenKind> {
        self.current
    }

    fn lexer_span(&self) -> Span {
        let span: Span = self.lexer.span().into();
        span.with_offset(self.offset)
    }

    pub fn remaining_token_span(&mut self) -> Option<Span> {
        if self.reached_eof() {
            None
        } else {
            let next_token = self.parse_token(semantic_token_legend::ST_COMMENT);
            Span::new(next_token.span.start, self.source.len() as u32).wrap_some()
        }
    }

    fn reached_eof(&self) -> bool {
        self.current.item == IsographLangTokenKind::EndOfFile
    }

    /// A &str for the source of the given span
    pub fn source(&self, span: Span) -> &'source str {
        let (raw_start, raw_end) = span.as_usize();
        let start = raw_start - self.offset as usize;
        let end = raw_end - self.offset as usize;

        &self.source[start..end]
    }

    /// If the next token doesn't match expected_kind, we don't advance
    /// the parser, so this is safe to use without peeking.
    pub fn parse_token_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithSpan<IsographLangTokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            self.parse_token(isograph_semantic_token).wrap_ok()
        } else {
            parse_token_kind_diagnostic(
                expected_kind,
                found.item,
                Location::new(self.text_source, found.span),
            )
            .wrap_err()
        }
    }

    /// Advances the parser iff the IsographLangTokenKind, so this is safe
    /// to call to see if the next token matches.
    pub fn parse_source_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithSpan<&'source str>> {
        let kind = self.parse_token_of_kind(expected_kind, isograph_semantic_token)?;

        self.source(kind.span).with_span(kind.span).wrap_ok()
    }

    pub fn parse_string_key_type<T: From<StringKey>>(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithSpan<T>> {
        let kind = self.parse_token_of_kind(expected_kind, isograph_semantic_token)?;
        let source = self.source(kind.span).intern();
        WithSpan::new(source.into(), kind.span).wrap_ok()
    }

    #[expect(dead_code)]
    pub fn parse_matching_identifier(
        &mut self,
        identifier: &'static str,
        isograph_semantic_token: IsographSemanticToken,
    ) -> DiagnosticResult<WithSpan<IsographLangTokenKind>> {
        let peeked = self.peek();
        if peeked.item == IsographLangTokenKind::Identifier {
            let source = self.source(peeked.span);
            if source == identifier {
                self.parse_token(isograph_semantic_token).wrap_ok()
            } else {
                parse_matching_identifier_diagnostic(
                    identifier,
                    source,
                    Location::new(self.text_source, peeked.span),
                )
                .wrap_err()
            }
        } else {
            parse_token_kind_diagnostic(
                IsographLangTokenKind::Identifier,
                peeked.item,
                Location::new(self.text_source, peeked.span),
            )
            .wrap_err()
        }
    }

    pub fn with_span_result<T, E>(
        &mut self,
        do_stuff: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<WithSpan<T>, E> {
        let start = self.current.span.start;
        let result = do_stuff(self)?;
        let end = self.end_index_of_last_parsed_token;

        // If `do_stuff` parses nothing, e.g. we call `with_span_result(parse_something_optional)`
        // and nothing is parsed, then end < start, and we will panic.
        //
        // In situations like that, call with_span_optional_result!

        result.with_span(Span::new(start, end)).wrap_ok()
    }

    pub fn with_span_optional_result<T, E>(
        &mut self,
        do_stuff: impl FnOnce(&mut Self) -> Result<Option<T>, E>,
    ) -> Result<Option<WithSpan<T>>, E> {
        let start = self.current.span.start;
        let result = do_stuff(self)?;
        let end = self.end_index_of_last_parsed_token;

        // Here, if do_stuff parses nothing, then it had better not advanced the cursor.
        debug_assert!(
            result.is_some() || (start == self.current.span.start),
            "We should either parse something and advance the cursor, or parse nothing and \
            not advance it."
        );

        result
            .map(|value| value.with_span(Span::new(start, end)))
            .wrap_ok()
    }

    pub fn white_space_span(&self) -> Span {
        Span::new(self.end_index_of_last_parsed_token, self.peek().span.start)
    }

    // TODO we should instead have a .complete method that takes self, checks that
    // there are no remaining unparsed tokens, and returns the semantic token vec,
    // so that we can avoid a clone.
    pub fn semantic_tokens(&self) -> Vec<WithSpan<IsographSemanticToken>> {
        self.semantic_tokens.clone()
    }
}

fn parse_token_kind_diagnostic(
    expected: IsographLangTokenKind,
    found: IsographLangTokenKind,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!("Expected {expected}, but found {found}"),
        location.wrap_some(),
    )
}

fn parse_matching_identifier_diagnostic(
    expected: &str,
    found: &str,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!("Expected {expected}, but found {found}"),
        location.wrap_some(),
    )
}
