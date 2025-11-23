use common_lang_types::{Diagnostic, WithSpan};
use thiserror::Error;

pub(crate) type ParseResult<T> = Result<T, WithSpan<SchemaParseError>>;

/// Errors tha make semantic sense when referring to parsing a GraphQL schema file
#[derive(Error, Clone, Eq, PartialEq, Debug, PartialOrd, Ord)]
pub enum SchemaParseError {
    #[error("{0}")]
    ParseError(#[from] Diagnostic),

    #[error(
        "Expected scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\""
    )]
    TopLevelSchemaDeclarationExpected { found_text: String },

    #[error(
        "Expected extend, scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\""
    )]
    TopLevelSchemaDeclarationOrExtensionExpected { found_text: String },

    #[error("Unable to parse constant value")]
    UnableToParseConstantValue,

    #[error("Invalid integer value. Received {text}")]
    InvalidIntValue { text: String },

    #[error("Invalid float value. Received {text}")]
    InvalidFloatValue { text: String },

    #[error("Expected a type (e.g. String, [String], or String!)")]
    ExpectedTypeAnnotation,

    #[error("Enum values cannot be true, false or null.")]
    EnumValueTrueFalseNull,

    #[error("Expected schema, mutation or subscription")]
    ExpectedRootOperationType,

    #[error(
        "Root operation types (query, subscription and mutation) cannot be defined twice in a schema definition"
    )]
    RootOperationTypeRedefined,
}
