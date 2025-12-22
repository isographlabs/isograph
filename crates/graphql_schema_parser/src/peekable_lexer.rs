use graphql_syntax::TokenKind;
use intern::string_key::{Intern, StringKey};
use logos::Logos;
use prelude::Postfix;

use common_lang_types::{
    Diagnostic, DiagnosticResult, Location, Span, TextSource, WithEmbeddedLocation,
    WithLocationPostfix, WithSpan, WithSpanPostfix,
};

pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<TokenKind>,
    lexer: logos::Lexer<'source, TokenKind>,
    source: &'source str,
    /// the byte offset of the *end* of the previous token
    end_index_of_last_parsed_token: u32,
    offset: u32,

    pub text_source: TextSource,
}

impl<'source> PeekableLexer<'source> {
    pub fn new(source_content: &'source str, text_source: TextSource) -> Self {
        // To enable fast lookahead the parser needs to store at least the 'kind' (TokenKind)
        // of the next token: the simplest option is to store the full current token, but
        // the Parser requires an initial value. Rather than incur runtime/code overhead
        // of dealing with an Option or UnsafeCell, the constructor uses a dummy token
        // value to construct the Parser, then immediately advance()s to move to the
        // first real token.
        let lexer = TokenKind::lexer(source_content);
        let dummy = TokenKind::EndOfFile.with_generated_span();

        let mut parser = PeekableLexer {
            current: dummy,
            lexer,
            source: source_content,
            end_index_of_last_parsed_token: 0,
            offset: 0,

            text_source,
        };

        // Advance to the first real token before doing any work
        parser.parse_token();
        parser
    }

    /// Get the next token (and advance)
    pub fn parse_token(&mut self) -> WithEmbeddedLocation<TokenKind> {
        let kind = self.lexer.next().unwrap_or(TokenKind::EndOfFile);
        self.end_index_of_last_parsed_token = self.current.span.end;
        let span = self.lexer_span();
        // TODO why does self.current = ... not work here?
        std::mem::replace(&mut self.current, kind.with_span(span))
            .to_with_embedded_location(self.text_source)
    }

    pub fn peek(&self) -> WithEmbeddedLocation<TokenKind> {
        self.current.to_with_embedded_location(self.text_source)
    }

    pub fn lexer_span(&self) -> Span {
        let span: Span = self.lexer.span().into();
        span.with_offset(self.offset)
    }

    pub fn reached_eof(&self) -> bool {
        self.current.item == TokenKind::EndOfFile
    }

    /// A &str for the source of the given span
    pub fn source(&self, span: Span) -> &'source str {
        let (raw_start, raw_end) = span.as_usize();
        let start = raw_start - self.offset as usize;
        let end = raw_end - self.offset as usize;

        &self.source[start..end]
    }

    /// Advances the parser iff the TokenKind, so this is safe
    /// to call to see if the next token matches.
    pub fn parse_token_of_kind(
        &mut self,
        expected_kind: TokenKind,
    ) -> DiagnosticResult<WithEmbeddedLocation<TokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            self.parse_token().wrap_ok()
        } else {
            Diagnostic::new(
                format!("Expected {expected_kind}, found {}.", found.item),
                found.location.to::<Location>().wrap_some(),
            )
            .wrap_err()
        }
    }

    /// Advances the parser iff the TokenKind, so this is safe
    /// to call to see if the next token matches.
    pub fn parse_source_of_kind(
        &mut self,
        expected_kind: TokenKind,
    ) -> DiagnosticResult<WithEmbeddedLocation<&'source str>> {
        let token = self.parse_token_of_kind(expected_kind)?;

        self.source(token.location.span)
            .with_generic_location(token.location)
            .wrap_ok()
    }

    pub fn parse_string_key_type<T: From<StringKey>>(
        &mut self,
        expected_kind: TokenKind,
    ) -> DiagnosticResult<WithEmbeddedLocation<T>> {
        let kind = self.parse_token_of_kind(expected_kind)?;
        let source = self.source(kind.location.span).intern();
        source
            .to::<T>()
            .with_generic_location(kind.location)
            .wrap_ok()
    }

    pub fn parse_matching_identifier(
        &mut self,
        identifier: &'static str,
    ) -> DiagnosticResult<WithEmbeddedLocation<TokenKind>> {
        let peeked = self.peek();
        if peeked.item == TokenKind::Identifier {
            let source = self.source(peeked.location.span);
            if source == identifier {
                self.parse_token().wrap_ok()
            } else {
                Diagnostic::new(
                    format!("Expected {identifier}, found {source}"),
                    peeked.location.to::<Location>().wrap_some(),
                )
                .wrap_err()
            }
        } else {
            Diagnostic::new(
                format!("Expected identifier, found {}", peeked.item),
                peeked.location.to::<Location>().wrap_some(),
            )
            .wrap_err()
        }
    }

    pub fn with_embedded_location_result<T, E>(
        &mut self,
        do_stuff: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<WithEmbeddedLocation<T>, E> {
        let start = self.current.span.start;
        let result = do_stuff(self)?;
        let end = self.end_index_of_last_parsed_token;
        result
            .with_span(Span::new(start, end))
            .to_with_embedded_location(self.text_source)
            .wrap_ok()
    }
}
