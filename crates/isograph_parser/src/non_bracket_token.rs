use std::fmt;

use crate::IsographLangTokenKind;

/// `IsographLangTokenKind` with the six bracket tokens unrepresentable: what the runs between
/// brackets hold once the bracket matcher has consumed the brackets. It has one variant per
/// non-bracket token, under the tokenizer's names.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum NonBracketTokenKind {
    Error,
    ErrorUnterminatedString,
    ErrorUnsupportedStringCharacter,
    ErrorUnterminatedBlockString,
    At,
    Colon,
    Dollar,
    EndOfFile,
    Equals,
    Exclamation,
    Identifier,
    IntegerLiteral,
    LineBreak,
    ErrorNumberLiteralLeadingZero,
    ErrorNumberLiteralTrailingInvalid,
    ErrorFloatLiteralMissingZero,
    Period,
    Comma,
    StringLiteral,
    BlockStringLiteral,
}

/// The bracket kinds, named as isograph names its tokens: paren `()`, brace `{}`,
/// bracket `[]`.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BracketKind {
    Paren,
    Brace,
    Bracket,
}

/// One of the six bracket tokens.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum BracketToken {
    Open(BracketKind),
    Close(BracketKind),
}

/// Every token is a bracket or it is not. This enum is the one place that split is defined,
/// so the bracket matcher and the runs it produces cannot disagree about what counts as a
/// bracket.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SplitToken {
    Bracket(BracketToken),
    NonBracket(NonBracketTokenKind),
}

impl From<IsographLangTokenKind> for SplitToken {
    fn from(kind: IsographLangTokenKind) -> Self {
        match kind {
            IsographLangTokenKind::OpenParen => {
                SplitToken::Bracket(BracketToken::Open(BracketKind::Paren))
            }
            IsographLangTokenKind::CloseParen => {
                SplitToken::Bracket(BracketToken::Close(BracketKind::Paren))
            }
            IsographLangTokenKind::OpenBrace => {
                SplitToken::Bracket(BracketToken::Open(BracketKind::Brace))
            }
            IsographLangTokenKind::CloseBrace => {
                SplitToken::Bracket(BracketToken::Close(BracketKind::Brace))
            }
            IsographLangTokenKind::OpenBracket => {
                SplitToken::Bracket(BracketToken::Open(BracketKind::Bracket))
            }
            IsographLangTokenKind::CloseBracket => {
                SplitToken::Bracket(BracketToken::Close(BracketKind::Bracket))
            }
            IsographLangTokenKind::Error => {
                SplitToken::NonBracket(NonBracketTokenKind::Error)
            }
            IsographLangTokenKind::ErrorUnterminatedString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedString)
            }
            IsographLangTokenKind::ErrorUnsupportedStringCharacter => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnsupportedStringCharacter)
            }
            IsographLangTokenKind::ErrorUnterminatedBlockString => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorUnterminatedBlockString)
            }
            IsographLangTokenKind::At => SplitToken::NonBracket(NonBracketTokenKind::At),
            IsographLangTokenKind::Colon => SplitToken::NonBracket(NonBracketTokenKind::Colon),
            IsographLangTokenKind::Dollar => SplitToken::NonBracket(NonBracketTokenKind::Dollar),
            IsographLangTokenKind::EndOfFile => {
                SplitToken::NonBracket(NonBracketTokenKind::EndOfFile)
            }
            IsographLangTokenKind::Equals => SplitToken::NonBracket(NonBracketTokenKind::Equals),
            IsographLangTokenKind::Exclamation => {
                SplitToken::NonBracket(NonBracketTokenKind::Exclamation)
            }
            IsographLangTokenKind::Identifier => {
                SplitToken::NonBracket(NonBracketTokenKind::Identifier)
            }
            IsographLangTokenKind::IntegerLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::IntegerLiteral)
            }
            IsographLangTokenKind::LineBreak => {
                SplitToken::NonBracket(NonBracketTokenKind::LineBreak)
            }
            IsographLangTokenKind::ErrorNumberLiteralLeadingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralLeadingZero)
            }
            IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid)
            }
            IsographLangTokenKind::ErrorFloatLiteralMissingZero => {
                SplitToken::NonBracket(NonBracketTokenKind::ErrorFloatLiteralMissingZero)
            }
            IsographLangTokenKind::Period => SplitToken::NonBracket(NonBracketTokenKind::Period),
            IsographLangTokenKind::Comma => SplitToken::NonBracket(NonBracketTokenKind::Comma),
            IsographLangTokenKind::StringLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::StringLiteral)
            }
            IsographLangTokenKind::BlockStringLiteral => {
                SplitToken::NonBracket(NonBracketTokenKind::BlockStringLiteral)
            }
        }
    }
}

impl From<NonBracketTokenKind> for IsographLangTokenKind {
    fn from(kind: NonBracketTokenKind) -> Self {
        match kind {
            NonBracketTokenKind::Error => IsographLangTokenKind::Error,
            NonBracketTokenKind::ErrorUnterminatedString => {
                IsographLangTokenKind::ErrorUnterminatedString
            }
            NonBracketTokenKind::ErrorUnsupportedStringCharacter => {
                IsographLangTokenKind::ErrorUnsupportedStringCharacter
            }
            NonBracketTokenKind::ErrorUnterminatedBlockString => {
                IsographLangTokenKind::ErrorUnterminatedBlockString
            }
            NonBracketTokenKind::At => IsographLangTokenKind::At,
            NonBracketTokenKind::Colon => IsographLangTokenKind::Colon,
            NonBracketTokenKind::Dollar => IsographLangTokenKind::Dollar,
            NonBracketTokenKind::EndOfFile => IsographLangTokenKind::EndOfFile,
            NonBracketTokenKind::Equals => IsographLangTokenKind::Equals,
            NonBracketTokenKind::Exclamation => IsographLangTokenKind::Exclamation,
            NonBracketTokenKind::Identifier => IsographLangTokenKind::Identifier,
            NonBracketTokenKind::IntegerLiteral => IsographLangTokenKind::IntegerLiteral,
            NonBracketTokenKind::LineBreak => IsographLangTokenKind::LineBreak,
            NonBracketTokenKind::ErrorNumberLiteralLeadingZero => {
                IsographLangTokenKind::ErrorNumberLiteralLeadingZero
            }
            NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid => {
                IsographLangTokenKind::ErrorNumberLiteralTrailingInvalid
            }
            NonBracketTokenKind::ErrorFloatLiteralMissingZero => {
                IsographLangTokenKind::ErrorFloatLiteralMissingZero
            }
            NonBracketTokenKind::Period => IsographLangTokenKind::Period,
            NonBracketTokenKind::Comma => IsographLangTokenKind::Comma,
            NonBracketTokenKind::StringLiteral => IsographLangTokenKind::StringLiteral,
            NonBracketTokenKind::BlockStringLiteral => IsographLangTokenKind::BlockStringLiteral,
        }
    }
}

impl fmt::Display for NonBracketTokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        IsographLangTokenKind::from(*self).fmt(f)
    }
}

#[cfg(test)]
mod tests {
    use super::{BracketKind, BracketToken, NonBracketTokenKind, SplitToken};
    use crate::IsographLangTokenKind;

    #[test]
    fn the_six_brackets_split_as_brackets() {
        for (kind, expected) in [
            (
                IsographLangTokenKind::OpenParen,
                BracketToken::Open(BracketKind::Paren),
            ),
            (
                IsographLangTokenKind::CloseParen,
                BracketToken::Close(BracketKind::Paren),
            ),
            (
                IsographLangTokenKind::OpenBrace,
                BracketToken::Open(BracketKind::Brace),
            ),
            (
                IsographLangTokenKind::CloseBrace,
                BracketToken::Close(BracketKind::Brace),
            ),
            (
                IsographLangTokenKind::OpenBracket,
                BracketToken::Open(BracketKind::Bracket),
            ),
            (
                IsographLangTokenKind::CloseBracket,
                BracketToken::Close(BracketKind::Bracket),
            ),
        ] {
            assert_eq!(SplitToken::from(kind), SplitToken::Bracket(expected));
        }
    }

    #[test]
    fn a_non_bracket_token_round_trips_through_the_split() {
        for kind in [
            IsographLangTokenKind::At,
            IsographLangTokenKind::Identifier,
            IsographLangTokenKind::StringLiteral,
            IsographLangTokenKind::Error,
        ] {
            match SplitToken::from(kind) {
                SplitToken::NonBracket(non_bracket) => {
                    assert_eq!(IsographLangTokenKind::from(non_bracket), kind);
                }
                SplitToken::Bracket(bracket) => {
                    panic!("{kind:?} split as a bracket: {bracket:?}")
                }
            }
        }
    }
}
