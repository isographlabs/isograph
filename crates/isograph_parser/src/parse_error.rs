use std::fmt;

use thiserror::Error;

use crate::{BracketKind, ChunkContentItem, NonBracketTokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum ParseError {
    #[error("{0}")]
    Expected(ExpectedFound),
    #[error("Expected a declaration. An isograph literal cannot be empty.")]
    EmptyLiteral,
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
    #[error("This declaration type is not supported yet.")]
    UnsupportedDeclarationType,
    #[error("This integer does not fit in a 64-bit signed integer.")]
    IntegerDoesNotFitI64,
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a comma, a line break, or {}", .0.closing())]
    Separator(BracketKind),
    #[error("an argument, like 'id: $id'")]
    Argument,
    #[error("a value, like $foo, 42, \"bar\", true, false, null, or an object literal")]
    Value,
    #[error("an object entry, like 'id: 4'")]
    ObjectEntry,
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::Display)]
pub enum Found {
    #[strum(to_string = "{0}")]
    Token(NonBracketTokenKind),
    #[strum(to_string = "a group opened by {0}")]
    Group(BracketKind),
    #[strum(to_string = "nothing more")]
    EndOfChunk,
}

impl ParseError {
    pub fn expected(expected: Expectation, found: Found) -> Self {
        ParseError::Expected(ExpectedFound { expected, found })
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
    use super::{Expectation, ExpectedFound, Found, ParseError};
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
            Expectation::DeclarationKeyword.to_string(),
            "one of `entrypoint`, `field`, or `pointer`",
        );
        assert_eq!(
            Expectation::EndOfDeclaration.to_string(),
            "the end of the declaration",
        );
        assert_eq!(
            Expectation::Separator(BracketKind::Parenthesis).to_string(),
            "a comma, a line break, or ')'",
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
    fn parse_error_expected_uses_expected_found() {
        assert_eq!(
            ParseError::expected(
                Expectation::Separator(BracketKind::Parenthesis),
                Found::EndOfChunk,
            )
            .to_string(),
            "Expected a comma, a line break, or ')', found nothing more.",
        );
    }
}
