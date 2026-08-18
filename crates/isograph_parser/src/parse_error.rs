use crate::{BracketKind, ChunkContentItem, NonBracketTokenKind};

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParseError {
    Expected(ExpectedFound),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct ExpectedFound {
    pub expected: Expectation,
    pub found: Found,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Expectation {
    Token(NonBracketTokenKind),
    Separator,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Found {
    Token(NonBracketTokenKind),
    Group(BracketKind),
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
