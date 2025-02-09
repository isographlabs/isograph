use super::error::{Error, Result};
use super::parser::{Parser, Precedence};
use super::token::Token;

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Program {
    pub expression: Expression,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Expression {
    IntegerLiteral(i64),
    PrefixOperator(PrefixOperator),
    InfixOperator(InfixOperator),
}

impl Expression {
    pub fn parse(parser: &mut Parser<'_>, precedence: Precedence) -> Result<Self> {
        let mut left = match &parser.current_token {
            Token::Int(value) => Self::IntegerLiteral(value.parse()?),
            Token::Minus => Self::PrefixOperator(PrefixOperator::parse(parser)?),
            Token::Lparen => {
                parser.next_token()?;
                let exp = Expression::parse(parser, Precedence::Lowest)?;
                parser.expect_peek(Token::Rparen)?;
                exp
            }
            _ => {
                return Err(Error::UnexpectedExpressionToken {
                    token: parser.current_token.clone(),
                })
            }
        };
        while !parser.peek_token_is(&Token::Eof)
            && precedence < Precedence::from(&parser.peek_token)
        {
            left = match &parser.peek_token {
                Token::Minus | Token::Plus | Token::ForwardSlash | Token::Asterisk => {
                    parser.next_token()?;
                    Self::InfixOperator(InfixOperator::parse(parser, left)?)
                }
                _ => return Ok(left),
            }
        }

        Ok(left)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct PrefixOperator {
    pub operator: PrefixOperatorKind,
    pub right: Box<Expression>,
}

impl PrefixOperator {
    fn parse(parser: &mut Parser<'_>) -> Result<Self> {
        let operator = match parser.current_token {
            Token::Minus => PrefixOperatorKind::Negative,
            _ => {
                return Err(Error::UnexpectedPrefixOperator {
                    token: parser.current_token.clone(),
                })
            }
        };
        parser.next_token()?;
        let right = Expression::parse(parser, Precedence::Prefix)?;
        Ok(Self {
            operator,
            right: Box::new(right),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct InfixOperator {
    pub left: Box<Expression>,
    pub operator: InfixOperatorKind,
    pub right: Box<Expression>,
}

impl InfixOperator {
    fn parse(parser: &mut Parser<'_>, left: Expression) -> Result<Self> {
        let operator = match parser.current_token {
            Token::Plus => InfixOperatorKind::Add,
            Token::Minus => InfixOperatorKind::Subtract,
            Token::Asterisk => InfixOperatorKind::Multiply,
            Token::ForwardSlash => InfixOperatorKind::Divide,
            _ => {
                return Err(Error::UnexpectedInfixOperator {
                    token: parser.current_token.clone(),
                })
            }
        };
        let precedence = Precedence::from(&parser.current_token);
        parser.next_token()?;
        let right = Expression::parse(parser, precedence)?;
        Ok(Self {
            left: Box::new(left),
            operator,
            right: Box::new(right),
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum PrefixOperatorKind {
    Negative,
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum InfixOperatorKind {
    Add,
    Subtract,
    Multiply,
    Divide,
}
