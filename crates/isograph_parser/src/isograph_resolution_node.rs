use crate::{
    ChunkPath, ChunkSeparatorPath, ChunkedGroupPath, ChunkedLevelPath, CloseBracketPath,
    NonBracketTokenPath, OpenBracketPath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the chunk tree's.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    ChunkedLevel(ChunkedLevelPath<'a>),
    /// This will be resolved for spans that contain one of the opening/closing brackets
    /// and part of the inside, e.g. "{ ba" in "foo { bar }". Single-character spans
    /// will never resolve to this.
    ChunkedGroup(ChunkedGroupPath<'a>),
    Chunk(ChunkPath<'a>),
    ChunkSeparator(ChunkSeparatorPath<'a>),
    NonBracketToken(NonBracketTokenPath<'a>),
    OpenBracket(OpenBracketPath<'a>),
    CloseBracket(CloseBracketPath<'a>),
}
