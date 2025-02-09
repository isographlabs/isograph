use std::mem;

use super::{
    ast::{Expression, Program},
    error::{Error, Result},
    lexer::Lexer,
    token::Token,
};

pub struct Parser<'a> {
    lexer: &'a mut Lexer,
    pub current_token: Token,
    pub peek_token: Token,
}

impl<'a> Parser<'a> {
    pub fn new(lexer: &'a mut Lexer) -> Result<Self> {
        let current_token = lexer.next_token()?;
        let peek_token = lexer.next_token()?;

        Ok(Self {
            lexer,
            current_token,
            peek_token,
        })
    }

    pub fn next_token(&mut self) -> Result<()> {
        self.current_token = self.lexer.next_token()?;
        mem::swap(&mut self.current_token, &mut self.peek_token);
        Ok(())
    }

    pub fn parse_program(&mut self) -> Result<Program> {
        Ok(Program {
            expression: Expression::parse(self, Precedence::Lowest)?,
        })
    }

    pub fn peek_token_is(&self, token: &Token) -> bool {
        &self.peek_token == token
    }

    pub fn expect_peek(&mut self, token: Token) -> Result<()> {
        if self.peek_token_is(&token) {
            self.next_token()
        } else {
            Err(Error::UnexpectedToken {
                peek: self.peek_token.clone(),
                expected: token,
            })
        }
    }
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest,
    Sum,
    Product,
    Prefix,
    Call,
}

impl From<&Token> for Precedence {
    fn from(value: &Token) -> Self {
        match value {
            Token::Plus | Token::Minus => Self::Sum,
            Token::ForwardSlash | Token::Asterisk => Self::Product,
            Token::Lparen => Self::Call,
            _ => Self::Lowest,
        }
    }
}
