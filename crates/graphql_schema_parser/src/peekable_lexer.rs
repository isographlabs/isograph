use graphql_syntax::TokenKind;
use intern::string_key::{Intern, StringKey};
use logos::Logos;
use thiserror::Error;

use common_lang_types::{Span, WithSpan};

pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<TokenKind>,
    lexer: logos::Lexer<'source, TokenKind>,
    source: &'source str,
    /// the byte offset of the *end* of the previous token
    end_index_of_last_parsed_token: u32,
    offset: u32,
}

type ParseResultWithSpan<T> = Result<T, WithSpan<LowLevelParseError>>;

impl<'source> PeekableLexer<'source> {
    pub fn new(source: &'source str) -> Self {
        // To enable fast lookahead the parser needs to store at least the 'kind' (TokenKind)
        // of the next token: the simplest option is to store the full current token, but
        // the Parser requires an initial value. Rather than incur runtime/code overhead
        // of dealing with an Option or UnsafeCell, the constructor uses a dummy token
        // value to construct the Parser, then immediately advance()s to move to the
        // first real token.
        let lexer = TokenKind::lexer(source);
        let dummy = WithSpan::new(TokenKind::EndOfFile, Span::todo_generated());

        let mut parser = PeekableLexer {
            current: dummy,
            lexer,
            source,
            end_index_of_last_parsed_token: 0,
            offset: 0,
        };

        // Advance to the first real token before doing any work
        parser.parse_token();
        parser
    }

    /// Get the next token (and advance)
    pub fn parse_token(&mut self) -> WithSpan<TokenKind> {
        // Skip over (and record) any invalid tokens until either a valid token or an EOF is encountered
        //
        // TODO: remove this allow after logic changed.
        #[allow(clippy::never_loop)]
        loop {
            let kind = self.lexer.next().unwrap_or(TokenKind::EndOfFile);
            match kind {
                TokenKind::Error => {
                    // TODO propagate? continue?
                    panic!(
                        "Encountered an error; this probably means you have an invalid character."
                    )
                }
                _ => {
                    self.end_index_of_last_parsed_token = self.current.span.end;
                    let span = self.lexer_span();
                    // TODO why does self.current = ... not work here?
                    return std::mem::replace(&mut self.current, WithSpan::new(kind, span));
                }
            }
        }
    }

    pub fn peek(&self) -> WithSpan<TokenKind> {
        self.current
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
    ) -> ParseResultWithSpan<WithSpan<TokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            Ok(self.parse_token())
        } else {
            Err(WithSpan::new(
                LowLevelParseError::ParseTokenKindError {
                    expected_kind,
                    found_kind: found.item,
                },
                found.span,
            ))
        }
    }

    /// Advances the parser iff the TokenKind, so this is safe
    /// to call to see if the next token matches.
    pub fn parse_source_of_kind(
        &mut self,
        expected_kind: TokenKind,
    ) -> ParseResultWithSpan<WithSpan<&'source str>> {
        let kind = self.parse_token_of_kind(expected_kind)?;

        Ok(WithSpan::new(self.source(kind.span), kind.span))
    }

    pub fn parse_string_key_type<T: From<StringKey>>(
        &mut self,
        expected_kind: TokenKind,
    ) -> ParseResultWithSpan<WithSpan<T>> {
        let kind = self.parse_token_of_kind(expected_kind)?;
        let source = self.source(kind.span).intern();
        Ok(WithSpan::new(source.into(), kind.span))
    }

    pub fn parse_matching_identifier(
        &mut self,
        identifier: &'static str,
    ) -> Result<WithSpan<TokenKind>, LowLevelParseError> {
        let peeked = self.peek();
        if peeked.item == TokenKind::Identifier {
            let source = self.source(peeked.span);
            if source == identifier {
                Ok(self.parse_token())
            } else {
                Err(LowLevelParseError::ParseMatchingIdentifierError {
                    expected_identifier: identifier,
                    found_text: source.to_string(),
                })
            }
        } else {
            Err(LowLevelParseError::ParseTokenKindError {
                expected_kind: TokenKind::Identifier,
                found_kind: peeked.item,
            })
        }
    }

    pub fn with_span<T, E>(
        &mut self,
        do_stuff: impl FnOnce(&mut Self) -> Result<T, E>,
    ) -> Result<WithSpan<T>, E> {
        let start = self.current.span.start;
        let result = do_stuff(self)?;
        let end = self.end_index_of_last_parsed_token;
        Ok(WithSpan::new(result, Span::new(start, end)))
    }
}

/// Low-level errors. If peekable_lexer could be made generic (it can't because it needs to know
/// about EOF), these would belong in a different crate than the parser itself.
#[derive(Error, Debug)]
pub enum LowLevelParseError {
    #[error("Expected {expected_kind}, found {found_kind}")]
    ParseTokenKindError {
        expected_kind: TokenKind,
        found_kind: TokenKind,
    },

    #[error("Expected {expected_identifier}, found \"{found_text}\"")]
    ParseMatchingIdentifierError {
        expected_identifier: &'static str,
        found_text: String,
    },
}
