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

#[derive(Copy, Clone, Debug, PartialEq, Eq, derive_more::Display)]
pub enum Expectation {
    #[display("{_0}")]
    Token(GraphQLTokenKind),
    #[display("the keyword `{_0}`")]
    Keyword(&'static str),
    #[display("a definition (schema, scalar, type, interface, union, enum, input, or directive)")]
    DefinitionKeyword,
    #[display(
        "a definition (schema, scalar, type, interface, union, enum, input, or directive) or extend"
    )]
    DefinitionOrExtensionKeyword,
    #[display("schema, scalar, type, interface, union, enum, or input")]
    ExtensionKeyword,
    #[display("a type, like 'String', 'String!', or '[String]'")]
    TypeAnnotation,
    #[display(
        "a constant value, like 42, 1.5, \"bar\", true, null, an enum value, a list, or an object"
    )]
    ConstantValue,
    #[display("query, mutation, or subscription")]
    OperationType,
    #[display("a directive location, like FIELD_DEFINITION")]
    DirectiveLocation,
    #[display("a directive, like '@deprecated'")]
    Directive,
    #[display("`implements`, a directive, or '{{'")]
    ImplementsDirectiveOrOpenBrace,
    #[display("a directive or '='")]
    DirectiveOrEquals,
    #[display("a directive or '{{'")]
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
