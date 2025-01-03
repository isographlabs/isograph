use thiserror::Error;

use super::token::Token;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum Error {
    #[error("expected next token to be {expected:?}, got {peek:?}")]
    UnexpectedToken { peek: Token, expected: Token },

    #[error("could not parse token as integer: {0}")]
    UnableToParseInteger(#[from] std::num::ParseIntError),

    #[error("unexpected token {token:?} in expression")]
    UnexpectedExpressionToken { token: Token },

    #[error("unexpected prefix operator: {token:?}")]
    UnexpectedPrefixOperator { token: Token },

    #[error("unexpected infix operator: {token:?}")]
    UnexpectedInfixOperator { token: Token },
}
