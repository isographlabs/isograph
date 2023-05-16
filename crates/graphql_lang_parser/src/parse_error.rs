use thiserror::Error;

use super::peekable_lexer::LowLevelParseError;

/// Errors tha make semantic sense when referring to parsing a GraphQL schema file
#[derive(Error, Debug)]
pub enum SchemaParseError {
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
}

impl From<LowLevelParseError> for SchemaParseError {
    fn from(error: LowLevelParseError) -> Self {
        SchemaParseError::ParseError { error }
    }
}
