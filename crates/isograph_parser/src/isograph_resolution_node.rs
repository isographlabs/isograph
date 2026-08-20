use crate::{
    ArgumentListPath, BooleanValuePath, ChunkPath, ChunkSeparatorPath, ChunkedGroupPath,
    ChunkedLevelPath, ClientFieldNamePath, CloseBracketPath, EntityNamePath,
    EntrypointDeclarationPath, ExtraChunksPath, FieldArgumentNameWrapperPath, IntegerValuePath,
    IsoLiteralParsePath, IsoLiteralSlotPath, NamedArgumentPath, NamedArgumentSlotPath,
    NonBracketTokenPath, NullValuePath, ObjectEntryPath, ObjectEntrySlotPath, ObjectLiteralPath,
    OpenBracketPath, StringLiteralValueWrapperPath, UnparsedChunkItemsPath,
    VariableNameWrapperPath, VariableUsePath,
};

/// What a position resolves to: the leaves of the newest tree. Each parsing stage
/// modifies these variants in place; today they are the grammar tree's, with the chunk
/// tree's still surfacing inside unparsed regions.
#[derive(Debug)]
#[non_exhaustive]
pub enum IsographResolutionNode<'a> {
    Singleton(IsoLiteralParsePath<'a>),
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    EntityName(EntityNamePath<'a>),
    ClientFieldName(ClientFieldNamePath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
    ExtraChunks(ExtraChunksPath<'a>),
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
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    NamedArgument(NamedArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
    FieldArgumentNameWrapper(FieldArgumentNameWrapperPath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableNameWrapper(VariableNameWrapperPath<'a>),
    StringLiteralValueWrapper(StringLiteralValueWrapperPath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
}
