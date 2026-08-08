use crate::{
    BracketedPath, CloseBracketPath, MatchedBracketsPath, NonBracketTokenPath, OpenBracketPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the bracket tree's.
#[derive(Debug)]
pub enum IsographResolutionNode<'a> {
    MatchedBrackets(MatchedBracketsPath<'a>),
    /// This will be resolved for spans that contains one of the opening/closing brace
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    Bracketed(BracketedPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
