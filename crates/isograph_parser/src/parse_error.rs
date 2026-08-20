use std::fmt;

use thiserror::Error;

use crate::{BracketError, BracketKind, ChunkContentItem, CommaWithoutItem, NonBracketTokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum AstError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
}

#[derive(Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Ast(#[from] AstError),
    #[error("{0}")]
    Bracket(#[from] BracketError),
    #[error("{0}")]
    Comma(#[from] CommaWithoutItem),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

impl fmt::Display for ExpectedFound {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Expected {}, found {}.", self.expected, self.found)
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(NonBracketTokenKind),
    Keyword(&'static str),
    Description,
    OneOf(&'static [Expectation]),
    EndOfDeclaration,
    SelectionSet,
    Selection,
    Separator(BracketKind),
    Argument,
    Value,
    ObjectEntry,
    VariableDeclaration,
    TypeAnnotation,
    EndOfType,
}

impl fmt::Display for Expectation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Expectation::Token(kind) => write!(f, "{kind}"),
            Expectation::Keyword(word) => write!(f, "the keyword `{word}`"),
            Expectation::Description => write!(f, "a description"),
            Expectation::OneOf(items) => write_one_of(f, items),
            Expectation::EndOfDeclaration => write!(f, "the end of the declaration"),
            Expectation::SelectionSet => write!(f, "a selection set, like '{{ id, name }}'"),
            Expectation::Selection => write!(f, "a selection"),
            Expectation::Separator(kind) => {
                write!(f, "a comma, a line break, or {}", kind.closing())
            }
            Expectation::Argument => write!(f, "an argument, like 'id: $id'"),
            Expectation::Value => write!(
                f,
                "a value, like $foo, 42, \"bar\", true, false, null, or an object literal"
            ),
            Expectation::ObjectEntry => write!(f, "an object entry, like 'id: 4'"),
            Expectation::VariableDeclaration => {
                write!(f, "a variable declaration, like '$id: ID!'")
            }
            Expectation::TypeAnnotation => {
                write!(f, "a type, like 'String', 'String!', or '[String]'")
            }
            Expectation::EndOfType => write!(f, "the end of the type"),
        }
    }
}

impl std::error::Error for Expectation {}

fn write_one_of(f: &mut fmt::Formatter<'_>, items: &[Expectation]) -> fmt::Result {
    match items {
        [] => write!(f, "one of"),
        [item] => write!(f, "{item}"),
        [first, second] => write!(f, "{first} or {second}"),
        [start @ .., last] => {
            for (i, item) in start.iter().enumerate() {
                if i > 0 {
                    write!(f, ", ")?;
                }
                write!(f, "{item}")?;
            }
            write!(f, ", or {last}")
        }
    }
}

pub const DECLARATION_KEYWORD: Expectation = Expectation::OneOf(&[
    Expectation::Keyword("entrypoint"),
    Expectation::Keyword("field"),
]);

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Found {
    #[strum(to_string = "{0}")]
    Token(NonBracketTokenKind),
    #[strum(to_string = "a group opened by {0}")]
    Group(BracketKind),
    #[strum(to_string = "nothing more")]
    EndOfChunk,
}

impl AstError {
    pub fn expected(expected: Expectation, found: Found) -> Self {
        AstError::Expected(ExpectedFound { expected, found })
    }
}

impl From<&ChunkContentItem> for Found {
    fn from(item: &ChunkContentItem) -> Self {
        match item {
            ChunkContentItem::NonBracket(token) => Found::Token(token.0),
            ChunkContentItem::Group(group) => Found::Group(group.opening.item.0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AstError, DECLARATION_KEYWORD, Expectation, ExpectedFound, Found};
    use crate::{BracketKind, NonBracketTokenKind};

    #[test]
    fn expectation_token_forwards_the_token_kind() {
        assert_eq!(
            Expectation::Token(NonBracketTokenKind::Identifier).to_string(),
            "non-variable identifier (e.g. 'x' or 'Foo')",
        );
    }

    #[test]
    fn expectation_unit_variants_use_their_messages() {
        assert_eq!(
            DECLARATION_KEYWORD.to_string(),
            "the keyword `entrypoint` or the keyword `field`",
        );
        assert_eq!(Expectation::Keyword("to").to_string(), "the keyword `to`",);
        assert_eq!(
            Expectation::EndOfDeclaration.to_string(),
            "the end of the declaration",
        );
        assert_eq!(
            Expectation::Separator(BracketKind::Parenthesis).to_string(),
            "a comma, a line break, or ')'",
        );
        assert_eq!(
            Expectation::OneOf(&[
                Expectation::Keyword("to"),
                Expectation::Description,
                Expectation::SelectionSet,
            ])
            .to_string(),
            "the keyword `to`, a description, or a selection set, like '{ id, name }'",
        );
        assert_eq!(Expectation::OneOf(&[]).to_string(), "one of");
        assert_eq!(
            Expectation::OneOf(&[Expectation::Description]).to_string(),
            "a description",
        );
        assert_eq!(Expectation::Description.to_string(), "a description");
        assert_eq!(
            Expectation::SelectionSet.to_string(),
            "a selection set, like '{ id, name }'",
        );
        assert_eq!(Expectation::Selection.to_string(), "a selection");
        assert_eq!(
            Expectation::Argument.to_string(),
            "an argument, like 'id: $id'",
        );
        assert_eq!(
            Expectation::Value.to_string(),
            "a value, like $foo, 42, \"bar\", true, false, null, or an object literal",
        );
        assert_eq!(
            Expectation::ObjectEntry.to_string(),
            "an object entry, like 'id: 4'",
        );
        assert_eq!(
            Expectation::VariableDeclaration.to_string(),
            "a variable declaration, like '$id: ID!'",
        );
        assert_eq!(
            Expectation::TypeAnnotation.to_string(),
            "a type, like 'String', 'String!', or '[String]'",
        );
        assert_eq!(Expectation::EndOfType.to_string(), "the end of the type");
        assert_eq!(
            Expectation::Separator(BracketKind::Brace).to_string(),
            "a comma, a line break, or '}'",
        );
        assert_eq!(
            Expectation::Separator(BracketKind::Bracket).to_string(),
            "a comma, a line break, or ']'",
        );
    }

    #[test]
    fn ast_error_unit_variants_use_their_messages() {
        assert_eq!(
            AstError::EmptyLiteral.to_string(),
            "Expected a declaration. An isograph literal cannot be empty.",
        );
        assert_eq!(
            AstError::MultipleDeclarations.to_string(),
            "Expected nothing after the declaration. Each literal holds exactly one declaration.",
        );
        assert_eq!(
            AstError::IntegerDoesNotFitI64.to_string(),
            "This integer does not fit in a 64-bit signed integer.",
        );
    }

    #[test]
    fn found_token_forwards_the_token_kind() {
        assert_eq!(
            Found::Token(NonBracketTokenKind::Dollar).to_string(),
            "dollar ('$')",
        );
    }

    #[test]
    fn found_group_includes_the_bracket() {
        assert_eq!(
            Found::Group(BracketKind::Brace).to_string(),
            "a group opened by '{'",
        );
    }

    #[test]
    fn found_end_of_chunk() {
        assert_eq!(Found::EndOfChunk.to_string(), "nothing more");
    }

    #[test]
    fn expected_found_joins_both_sides() {
        assert_eq!(
            ExpectedFound {
                expected: Expectation::EndOfDeclaration,
                found: Found::EndOfChunk,
            }
            .to_string(),
            "Expected the end of the declaration, found nothing more.",
        );
    }

    #[test]
    fn ast_error_expected_uses_expected_found() {
        assert_eq!(
            AstError::expected(
                Expectation::Separator(BracketKind::Parenthesis),
                Found::EndOfChunk,
            )
            .to_string(),
            "Expected a comma, a line break, or ')', found nothing more.",
        );
    }
}
