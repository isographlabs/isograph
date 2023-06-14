use thiserror::Error;

use crate::IsographLangTokenKind;

use super::peekable_lexer::LowLevelParseError;

pub(crate) type ParseResult<T> = Result<T, IsographLiteralParseError>;

/// Errors tha make semantic sense when referring to parsing a Isograph literal
#[derive(Error, Debug)]
pub enum IsographLiteralParseError {
    #[error("{error}")]
    ParseError { error: LowLevelParseError },

    #[error("Expected scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\"")]
    TopLevelSchemaDeclarationExpected { found_text: String },

    #[error("Unable to parse constant value")]
    UnableToParseConstantValue,

    #[error("Invalid integer value. Received {text}")]
    InvalidIntValue { text: String },

    #[error("Invalid float value. Received {text}")]
    InvalidFloatValue { text: String },

    #[error("Expected a type (e.g. String, [String], or String!)")]
    ExpectedTypeAnnotation,

    #[error("Leftover tokens. Next token: {token}")]
    LeftoverTokens { token: IsographLangTokenKind },
}

impl From<LowLevelParseError> for IsographLiteralParseError {
    fn from(error: LowLevelParseError) -> Self {
        IsographLiteralParseError::ParseError { error }
    }
}
