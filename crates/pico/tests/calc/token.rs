use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum Token {
    Int(String),
    Eof,
    Minus,
    ForwardSlash,
    Asterisk,
    Plus,
    Lparen,
    Rparen,
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Token::Int(x) => write!(f, "Int({})", x),
            Token::Eof => write!(f, "Eof"),
            Token::Minus => write!(f, "Minus"),
            Token::ForwardSlash => write!(f, "ForwardSlash"),
            Token::Asterisk => write!(f, "Asterisk"),
            Token::Plus => write!(f, "Plus"),
            Token::Lparen => write!(f, "Lparen"),
            Token::Rparen => write!(f, "Rparen"),
        }
    }
}
