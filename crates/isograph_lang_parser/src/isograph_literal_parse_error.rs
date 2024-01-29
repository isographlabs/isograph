use common_lang_types::{ScalarFieldName, WithLocation, WithSpan};
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

    #[error("Isograph literals must be immediately called, and passed a function")]
    ExpectedAssociatedJsFunction,

    #[error("Isograph literals must start with field or entrypoint")]
    ExpectedFieldOrEntrypoint,

    #[error(
        "This isograph literal must be exported as `export const {expected_const_export_name}`"
    )]
    ExpectedLiteralToBeExported {
        expected_const_export_name: ScalarFieldName,
    },
}

impl From<LowLevelParseError> for IsographLiteralParseError {
    fn from(error: LowLevelParseError) -> Self {
        IsographLiteralParseError::ParseError { error }
    }
}
