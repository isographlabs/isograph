use prelude::Postfix;

use crate::{NonBracketTokenKind, SplitToken};

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
pub enum IsographSemanticToken {
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

// Only leftover fill-in: extra, leftover chunks, the matcher's cut.
pub(crate) fn leftover_token(kind: SplitToken) -> Option<IsographSemanticToken> {
    match kind {
        SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral) => {
            IsographSemanticToken::Integer.wrap_some()
        }
        SplitToken::NonBracket(
            NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
        ) => IsographSemanticToken::String.wrap_some(),
        SplitToken::NonBracket(NonBracketTokenKind::Error) => {
            IsographSemanticToken::Error.wrap_some()
        }
        SplitToken::NonBracket(NonBracketTokenKind::LineBreak | NonBracketTokenKind::EndOfFile) => {
            None
        }
        SplitToken::NonBracket(_) => IsographSemanticToken::Content.wrap_some(),
        SplitToken::Bracket(_) => IsographSemanticToken::Bracket.wrap_some(),
    }
}
