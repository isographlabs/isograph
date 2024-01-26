use common_lang_types::WithSpan;
use thiserror::Error;

use super::peekable_lexer::LowLevelParseError;

pub(crate) type ParseResult<T> = Result<T, WithSpan<SchemaParseError>>;

/// Errors tha make semantic sense when referring to parsing a GraphQL schema file
#[derive(Error, Debug)]
pub enum SchemaParseError {
    #[error("{error}")]
    ParseError { error: LowLevelParseError },

    #[error("Expected scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\"")]
    TopLevelSchemaDeclarationExpected { found_text: String },

    #[error("Expected extend, scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\"")]
    TopLevelSchemaDeclarationOrExtensionExpected { found_text: String },

    #[error("Unable to parse constant value")]
    UnableToParseConstantValue,

    #[error("Invalid integer value. Received {text}")]
    InvalidIntValue { text: String },

    #[error("Invalid float value. Received {text}")]
    InvalidFloatValue { text: String },

    #[error("Expected a type (e.g. String, [String], or String!)")]
    ExpectedTypeAnnotation,

    #[error("Expected directive location. Found {text}")]
    ExpectedDirectiveLocation { text: String },

    #[error("Enum values cannot be true, false or null.")]
    EnumValueTrueFalseNull,

    #[error("Expected schema, mutation or subscription")]
    ExpectedRootOperationType,

    #[error("Root operation types (query, subscription and mutation) cannot be defined twice in a schema definition")]
    RootOperationTypeRedefined,
}

impl From<LowLevelParseError> for SchemaParseError {
    fn from(error: LowLevelParseError) -> Self {
        SchemaParseError::ParseError { error }
    }
}
