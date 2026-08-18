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
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
#[error("Expected {expected}, found {found}.")]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a comma or line break")]
    Separator,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, Error)]
pub enum Found {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("a group opened by {}", opening_bracket_text(*.0))]
    Group(BracketKind),
    #[error("nothing more")]
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

fn opening_bracket_text(kind: BracketKind) -> &'static str {
    match kind {
        BracketKind::Parenthesis => "'('",
        BracketKind::Brace => "'{'",
        BracketKind::Bracket => "'['",
    }
}
