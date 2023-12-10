use common_lang_types::{WithLocation, WithSpan};
use thiserror::Error;

use super::peekable_lexer::LowLevelParseError;

pub(crate) type ParseResultWithLocation<T> = Result<T, WithLocation<IsographLiteralParseError>>;
pub(crate) type ParseResultWithSpan<T> = Result<T, WithSpan<IsographLiteralParseError>>;

/// Errors tha make semantic sense when referring to parsing a Isograph literal
#[derive(Error, Debug)]
pub enum IsographLiteralParseError {
    #[error("{error}")]
    ParseError { error: LowLevelParseError },

    #[error("Expected a type (e.g. String, [String], or String!)")]
    ExpectedTypeAnnotation,

    #[error("Unparsed tokens remaining")]
    LeftoverTokens,

    #[error("isograph literals must be immediately called, and passed a function")]
    ExpectedAssociatedJsFunction,
}

impl From<LowLevelParseError> for IsographLiteralParseError {
    fn from(error: LowLevelParseError) -> Self {
        IsographLiteralParseError::ParseError { error }
    }
}
