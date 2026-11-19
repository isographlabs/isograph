use thiserror::Error;

use crate::GraphQLTokenKind;

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
    #[error("This float does not parse as a 64-bit float.")]
    InvalidFloat,
    #[error("Enum values cannot be true, false, or null.")]
    EnumValueCannotBeTrueFalseOrNull,
    #[error("This escape sequence does not encode a Unicode scalar value.")]
    InvalidUnicodeEscape,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Display)]
#[display("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Expectation {
    #[strum(to_string = "{0}")]
    Token(GraphQLTokenKind),
    #[strum(to_string = "the keyword `{0}`")]
    Keyword(&'static str),
    #[strum(
        to_string = "a definition (schema, scalar, type, interface, union, enum, input, or directive)"
    )]
    DefinitionKeyword,
    #[strum(
        to_string = "a definition (schema, scalar, type, interface, union, enum, input, or directive) or extend"
    )]
    DefinitionOrExtensionKeyword,
    #[strum(to_string = "schema, scalar, type, interface, union, enum, or input")]
    ExtensionKeyword,
    #[strum(to_string = "a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[strum(
        to_string = "a constant value, like 42, 1.5, \"bar\", true, null, an enum value, a list, or an object"
    )]
    ConstantValue,
    #[strum(to_string = "query, mutation, or subscription")]
    OperationType,
    #[strum(to_string = "a directive location, like FIELD_DEFINITION")]
    DirectiveLocation,
    #[strum(to_string = "a directive, like '@deprecated'")]
    Directive,
    #[strum(to_string = "`implements`, a directive, or '{{'")]
    ImplementsDirectiveOrOpenBrace,
    #[strum(to_string = "a directive or '='")]
    DirectiveOrEquals,
    #[strum(to_string = "a directive or '{{'")]
    DirectiveOrOpenBrace,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Found {
    #[strum(to_string = "{0}")]
    Token(GraphQLTokenKind),
    #[strum(to_string = "end of file")]
    EndOfFile,
}

impl ParseError {
    pub fn expected(expected: Expectation, found: Found) -> Self {
        ParseError::Expected(ExpectedFound { expected, found })
    }
}
