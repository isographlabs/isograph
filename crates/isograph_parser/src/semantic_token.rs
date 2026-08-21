use prelude::Postfix;

use crate::{NonBracketTokenKind, SplitToken};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum SemanticToken {
    Keyword,
    Type,
    FieldName,
    ObjectKey,
    GraphQLTypeName,
    DirectiveName,
    Variable,
    Argument,
    Integer,
    String,
    BooleanOrNull,
    Period,
    Colon,
    Equals,
    Parenthesis,
    Brace,
    Content,
    Bracket,
    Error,
}

// Only leftover fill-in: extra, extra_chunks, the matcher's cut.
pub(crate) fn leftover_token(kind: SplitToken) -> Option<SemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            SemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => SemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => SemanticToken::Error.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => SemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => SemanticToken::Bracket.wrap_some(),
    }
}
