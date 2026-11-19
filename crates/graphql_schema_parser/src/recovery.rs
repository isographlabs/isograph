use std::str::FromStr;

use span::WithSpan;

use crate::GraphQLTokenKind;
use crate::keyword::DefinitionKeyword;
use crate::token_cursor::{TokenCursor, TokenIndex};

/// How many brackets of any kind are open. Never below zero: a close with no open leaves
/// the depth at the top level.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
struct BracketDepth(u32);

impl BracketDepth {
    const TOP_LEVEL: BracketDepth = BracketDepth(0);

    fn after(self, kind: GraphQLTokenKind) -> BracketDepth {
        match kind {
            GraphQLTokenKind::OpenBrace
            | GraphQLTokenKind::OpenParenthesis
            | GraphQLTokenKind::OpenBracket => BracketDepth(self.0 + 1),
            GraphQLTokenKind::CloseBrace
            | GraphQLTokenKind::CloseParenthesis
            | GraphQLTokenKind::CloseBracket => BracketDepth(self.0.saturating_sub(1)),
            _ => self,
        }
    }
}

/// Moves the cursor from a failed definition, which began at `start`, to the next token
/// that starts a definition at the top level, or to the end of the file.
pub(crate) fn skip_failed_definition(cursor: &mut TokenCursor<'_>, start: TokenIndex) {
    if cursor.position() == start {
        cursor.advance();
    }
    let mut depth = cursor
        .consumed_since(start)
        .iter()
        .fold(BracketDepth::TOP_LEVEL, |depth, token| {
            depth.after(token.item)
        });
    while let Some(token) = cursor.peek() {
        if depth == BracketDepth::TOP_LEVEL && starts_definition(cursor) {
            break;
        }
        depth = depth.after(token.item);
        cursor.advance();
    }
}

/// Whether the next tokens are a definition keyword or `extend`, or a description followed
/// by one.
fn starts_definition(cursor: &TokenCursor<'_>) -> bool {
    match cursor.peek().map(|token| token.item) {
        Some(GraphQLTokenKind::StringLiteral | GraphQLTokenKind::BlockStringLiteral) => cursor
            .peek_nth(1)
            .is_some_and(|token| is_top_level_keyword(cursor, token)),
        Some(GraphQLTokenKind::Identifier) => cursor
            .peek()
            .is_some_and(|token| is_top_level_keyword(cursor, token)),
        _ => false,
    }
}

fn is_top_level_keyword(cursor: &TokenCursor<'_>, token: WithSpan<GraphQLTokenKind>) -> bool {
    let text = &cursor.text()[token.location.as_usize_range()];
    token.item == GraphQLTokenKind::Identifier
        && (text == "extend" || DefinitionKeyword::from_str(text).is_ok())
}
