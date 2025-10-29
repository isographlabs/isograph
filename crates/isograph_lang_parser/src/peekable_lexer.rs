use crate::IsographLangTokenKind;
use common_lang_types::{Span, WithSpan};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{IsographSemanticToken, semantic_token_legend};
use logos::Logos;
use thiserror::Error;

pub(crate) struct PeekableLexer<'source> {
    current: WithSpan<IsographLangTokenKind>,
    lexer: logos::Lexer<'source, IsographLangTokenKind>,
    source: &'source str,
    /// the byte offset of the *end* of the previous token
    end_index_of_last_parsed_token: u32,
    offset: u32,
    semantic_tokens: Vec<WithSpan<IsographSemanticToken>>,
}

impl<'source> PeekableLexer<'source> {
    pub fn new(source: &'source str) -> Self {
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
            semantic_tokens: vec![],
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
        let parsed_token = std::mem::replace(&mut self.current, WithSpan::new(kind, span));

        self.semantic_tokens
            .push(WithSpan::new(isograph_semantic_token, parsed_token.span));

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
            Some(Span::new(next_token.span.start, self.source.len() as u32))
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
    ) -> LowLevelParseResult<WithSpan<IsographLangTokenKind>> {
        let found = self.peek();
        if found.item == expected_kind {
            Ok(self.parse_token(isograph_semantic_token))
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
        isograph_semantic_token: IsographSemanticToken,
    ) -> LowLevelParseResult<WithSpan<&'source str>> {
        let kind = self.parse_token_of_kind(expected_kind, isograph_semantic_token)?;

        Ok(WithSpan::new(self.source(kind.span), kind.span))
    }

    pub fn parse_string_key_type<T: From<StringKey>>(
        &mut self,
        expected_kind: IsographLangTokenKind,
        isograph_semantic_token: IsographSemanticToken,
    ) -> LowLevelParseResult<WithSpan<T>> {
        let kind = self.parse_token_of_kind(expected_kind, isograph_semantic_token)?;
        let source = self.source(kind.span).intern();
        Ok(WithSpan::new(source.into(), kind.span))
    }

    #[expect(dead_code)]
    pub fn parse_matching_identifier(
        &mut self,
        identifier: &'static str,
        isograph_semantic_token: IsographSemanticToken,
    ) -> LowLevelParseResult<WithSpan<IsographLangTokenKind>> {
        let peeked = self.peek();
        if peeked.item == IsographLangTokenKind::Identifier {
            let source = self.source(peeked.span);
            if source == identifier {
                Ok(self.parse_token(isograph_semantic_token))
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

    // TODO we should instead have a .complete method that takes self, checks that
    // there are no remaining unparsed tokens, and returns the semantic token vec,
    // so that we can avoid a clone.
    pub fn semantic_tokens(&self) -> Vec<WithSpan<IsographSemanticToken>> {
        self.semantic_tokens.clone()
    }
}

type LowLevelParseResult<T> = Result<T, WithSpan<LowLevelParseError>>;

/// Low-level errors. If peekable_lexer could be made generic (it can't because it needs to know
/// about EOF), these would belong in a different crate than the parser itself.
#[derive(Error, Clone, Eq, PartialEq, Debug)]
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
