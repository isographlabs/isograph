use crate::IsographLangTokenKind;
use common_lang_types::{EmbeddedLocation, Location, Span, TextSource, WithLocation, WithSpan};
use intern::string_key::{Intern, StringKey};
use logos::Logos;
use thiserror::Error;

pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    /// the byte offset of the *end* of the previous token
    end_index_of_last_parsed_token: u32,
    offset: u32,
    text_source: TextSource,
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
        let dummy = WithSpan::new(IsographLangTokenKind::EndOfFile, Span::todo_generated());

        let mut parser = PeekableLexer {
            current: dummy,
            lexer,
            source,
            end_index_of_last_parsed_token: 0,
            offset: 0,
            text_source,
        };

        // Advance to the first real token before doing any work
        parser.parse_token();
        parser
    }

    /// Get the next token (and advance)
    pub fn parse_token(&mut self) -> WithSpan<IsographLangTokenKind> {
        // Skip over (and record) any invalid tokens until either a valid token or an EOF is encountered
        //
        // Remove this allow after logic changed.
        #[allow(clippy::never_loop)]
        loop {
            let kind = self
                .lexer
                .next()
                .unwrap_or(IsographLangTokenKind::EndOfFile);
            match kind {
                IsographLangTokenKind::Error => {
                    // HACK we print out the location here, but we should return
                    // the error. In particular, we can show multiple such errors
                    // if they occur in different iso literals.
                    let span = self.lexer_span();
                    // TODO propagate? continue?
                    panic!(
                        "Encountered an error. \
                        This can occur if you commented out an iso literal, \
                        or if an iso literal contains \
                        an invalid token. \n{}",
                        WithLocation::new(
                            "",
                            Location::Embedded(EmbeddedLocation {
                                text_source: self.text_source,
                                span
                            })
                        )
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

    pub fn peek(&self) -> WithSpan<IsographLangTokenKind> {
        self.current
    }

    pub fn lexer_span(&self) -> Span {
        let span: Span = self.lexer.span().into();
        span.with_offset(self.offset)
    }

    pub fn remaining_token_span(&mut self) -> Option<Span> {
        if self.reached_eof() {
            None
        } else {
            let next_token = self.parse_token();
            Some(Span::new(next_token.span.start, self.source.len() as u32))
        }
    }

    pub fn reached_eof(&self) -> bool {
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
    ) -> LowLevelParseResult<WithSpan<IsographLangTokenKind>> {
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

    /// Advances the parser iff the IsographLangTokenKind, so this is safe
    /// to call to see if the next token matches.
    pub fn parse_source_of_kind(
        &mut self,
        expected_kind: IsographLangTokenKind,
    ) -> LowLevelParseResult<WithSpan<&'source str>> {
        let kind = self.parse_token_of_kind(expected_kind)?;

        Ok(WithSpan::new(self.source(kind.span), kind.span))
    }

    pub fn parse_string_key_type<T: From<StringKey>>(
        &mut self,
        expected_kind: IsographLangTokenKind,
    ) -> LowLevelParseResult<WithSpan<T>> {
        let kind = self.parse_token_of_kind(expected_kind)?;
        let source = self.source(kind.span).intern();
        Ok(WithSpan::new(source.into(), kind.span))
    }

    #[allow(dead_code)]
    pub fn parse_matching_identifier(
        &mut self,
        identifier: &'static str,
    ) -> LowLevelParseResult<WithSpan<IsographLangTokenKind>> {
        let peeked = self.peek();
        if peeked.item == IsographLangTokenKind::Identifier {
            let source = self.source(peeked.span);
            if source == identifier {
                Ok(self.parse_token())
            } else {
                Err(WithSpan::new(
                    LowLevelParseError::ParseMatchingIdentifierError {
                        expected_identifier: identifier,
                        found_text: source.to_string(),
                    },
                    peeked.span,
                ))
            }
        } else {
            Err(WithSpan::new(
                LowLevelParseError::ParseTokenKindError {
                    expected_kind: IsographLangTokenKind::Identifier,
                    found_kind: peeked.item,
                },
                peeked.span,
            ))
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

    pub fn white_space_span(&self) -> Span {
        Span::new(self.end_index_of_last_parsed_token, self.peek().span.start)
    }
}

type LowLevelParseResult<T> = Result<T, WithSpan<LowLevelParseError>>;

/// Low-level errors. If peekable_lexer could be made generic (it can't because it needs to know
/// about EOF), these would belong in a different crate than the parser itself.
#[derive(Error, Debug)]
pub enum LowLevelParseError {
    #[error("Expected {expected_kind}, found {found_kind}.")]
    ParseTokenKindError {
        expected_kind: IsographLangTokenKind,
        found_kind: IsographLangTokenKind,
    },

    #[error("Expected {expected_identifier}, found \"{found_text}\"")]
    ParseMatchingIdentifierError {
        expected_identifier: &'static str,
        found_text: String,
    },
}
