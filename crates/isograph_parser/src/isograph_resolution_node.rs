use crate::{
    ArgumentListPath, BooleanValuePath, ChunkPath, ChunkSeparatorPath, ChunkedGroupPath,
    ChunkedLevelPath, CloseBracketPath, DescriptionPath, EntityNameWrapperPath,
    EntrypointDeclarationPath, ExtraChunksPath, FieldArgumentNameWrapperPath, FieldDeclarationPath,
    IntegerValuePath, IsoLiteralParsePath, IsoLiteralSlotPath, IsographDirectiveNameWrapperPath,
    IsographFieldDirectiveListPath, IsographFieldDirectivePath, ListTypeAnnotationPath,
    NamedTypeAnnotationPath, NonBracketTokenPath, NullValuePath, ObjectEntryPath,
    ObjectEntrySlotPath, ObjectLiteralPath, OpenBracketPath, SelectableNameWrapperPath,
    SelectionFieldArgumentPath, SelectionFieldArgumentSlotPath, SelectionNameWrapperPath,
    SelectionPath, SelectionSetPath, SelectionSlotPath, StringLiteralValueWrapperPath,
    UnparsedChunkItemsPath, ValueKeyNameWrapperPath, VariableDeclarationOrUsageListPath,
    VariableDeclarationOrUsagePath, VariableDeclarationOrUsageSlotPath, VariableNameWrapperPath,
    VariableUsePath,
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
    FieldDeclaration(FieldDeclarationPath<'a>),
    Description(DescriptionPath<'a>),
    EntityNameWrapper(EntityNameWrapperPath<'a>),
    SelectableNameWrapper(SelectableNameWrapperPath<'a>),
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
    SelectionFieldArgumentSlot(SelectionFieldArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    SelectionFieldArgument(SelectionFieldArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
    FieldArgumentNameWrapper(FieldArgumentNameWrapperPath<'a>),
    ValueKeyNameWrapper(ValueKeyNameWrapperPath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableNameWrapper(VariableNameWrapperPath<'a>),
    StringLiteralValueWrapper(StringLiteralValueWrapperPath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    Selection(SelectionPath<'a>),
    SelectionNameWrapper(SelectionNameWrapperPath<'a>),
    VariableDeclarationOrUsageSlot(VariableDeclarationOrUsageSlotPath<'a>),
    VariableDeclarationOrUsageList(VariableDeclarationOrUsageListPath<'a>),
    VariableDeclarationOrUsage(VariableDeclarationOrUsagePath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    IsographFieldDirectiveList(IsographFieldDirectiveListPath<'a>),
    IsographFieldDirective(IsographFieldDirectivePath<'a>),
    IsographDirectiveNameWrapper(IsographDirectiveNameWrapperPath<'a>),
}
