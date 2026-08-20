use nonempty::NonEmpty;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::{IntoSafePeekable, SafePeekable};
use span::{Span, WithSpan, WithSpanPostfix};

use crate::{
    Argument, ArgumentListPath, ArgumentSlotPath, BracketItem, Bracketed, CloseBracket,
    Expectation, ExtraChunksPath, Found, IsoLiteralItem, IsoLiteralParsePath, IsoLiteralSlotPath,
    IsographResolutionNode, ListLiteralPath, ListLiteralValue, ListLiteralValueSlotPath,
    ListTypeAnnotationPath, MatchedBrackets, NonBracketToken, NonBracketTokenKind, ObjectEntry,
    ObjectEntrySlotPath, ObjectLiteralPath, OpenBracket, ParseError, Selection, SelectionSetPath,
    SelectionSlotPath, SemanticToken, VariableDeclaration, VariableDeclarationListPath,
    VariableDeclarationSlotPath,
    chunk_stream::{ChunkStream, ItemCursor},
};

/// One level of the chunk tree: the whole literal at the root, a group's interior
/// below — the same role `MatchedBrackets` has on the bracket tree. Its chunks, in
/// order, and nothing else: line breaks at a level's start were captured by the opening
/// bracket (or the literal's start), and a comma no item precedes is a
/// `CommaWithoutItem` error beside the tree, its boundary dropped.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel(
    #[resolve_field]
    #[parent_variant(Level)]
    pub Vec<WithSpan<Chunk>>,
);

/// A maximal separator-free run of a level's items — tokens and groups — plus the
/// boundary that ended it when one did: line breaks and at most one comma, a second
/// comma ending the boundary as well. The chunk-parsing pass consumes it as one unit.
/// Every chunk but a level's last has a trailing separator by construction; the last's
/// is the optional trailing delimiter.
/// The wrapping `WithSpan`'s span runs from the first part's start to the last part's
/// end.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    #[parent_variant(Chunk)]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}

/// What a chunk holds: every non-separator item of its level, groups included.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum ChunkContentItem {
    NonBracket(NonBracketToken),
    Group(ChunkedGroup),
}

/// A matched group re-chunked: the bracket tree's opening and closing are kept, and the
/// interior is a `ChunkedLevel` — same layout as `Bracketed` / `MatchedBrackets`.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkContentItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedGroup {
    #[resolve_field]
    pub opening: WithSpan<OpenBracket>,
    /// The wrapping `WithSpan`'s span runs from the opening's end to the closing's start.
    #[resolve_field]
    #[parent_variant(Interior)]
    pub children: WithSpan<ChunkedLevel>,
    #[resolve_field]
    pub closing: WithSpan<CloseBracket>,
}

/// The boundary that ended its chunk: its line-break tokens and at most one comma, in
/// order. A second comma is never absorbed; it opens the next chunk's boundary. Its
/// tokens are not resolution leaves; a position on any of them answers the separator.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkSeparator(pub NonEmpty<WithSpan<SeparatorToken>>);

/// The two token kinds a separator boundary can hold.
#[derive(Copy, Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum SeparatorToken {
    Comma,
    LineBreak,
}

/// A chunking error: a comma no item precedes, at the comma's span. The comma and its
/// boundary have no chunk; positions on them answer their level.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct CommaWithoutItem(pub Span);

#[derive(Debug)]
pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}

#[derive(Debug)]
pub enum ChunkParent<'a> {
    Level(ChunkedLevelPath<'a>),
    Extra(ExtraChunksPath<'a>),
}

#[derive(Debug)]
pub enum ChunkContentItemParent<'a> {
    Chunk(ChunkPath<'a>),
    Unparsed(UnparsedChunkItemsPath<'a>),
}

pub type ChunkedLevelPath<'a> = PositionResolutionPath<&'a ChunkedLevel, ChunkedLevelParent<'a>>;

pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkParent<'a>>;

pub type ChunkedGroupPath<'a> =
    PositionResolutionPath<&'a ChunkedGroup, ChunkContentItemParent<'a>>;

pub type ChunkSeparatorPath<'a> = PositionResolutionPath<&'a ChunkSeparator, ChunkPath<'a>>;

pub type NonBracketTokenPath<'a> =
    PositionResolutionPath<&'a NonBracketToken, ChunkContentItemParent<'a>>;

pub type OpenBracketPath<'a> = PositionResolutionPath<&'a OpenBracket, ChunkedGroupPath<'a>>;
pub type CloseBracketPath<'a> = PositionResolutionPath<&'a CloseBracket, ChunkedGroupPath<'a>>;

type LevelItems<'a> = SafePeekable<std::slice::Iter<'a, WithSpan<BracketItem>>>;

impl Chunk {
    pub(crate) fn stream<'a>(
        &'a self,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
    ) -> ChunkStream<'a> {
        ChunkStream::new(self.contents.reference(), text, tokens, errors)
    }

    /// First content item through last content item. `WithSpan<Chunk>` also covers
    /// the trailing separator.
    pub fn contents_span(&self) -> Span {
        Span::join(
            self.contents.first().location,
            self.contents.last().location,
        )
    }

    pub(crate) fn first_item(&self) -> &WithSpan<ChunkContentItem> {
        self.contents.first()
    }

    pub fn boundary_comma(&self) -> Option<Span> {
        let separator = self.trailing_separator.as_ref()?;
        separator
            .item
            .0
            .iter()
            .find(|token| token.item == SeparatorToken::Comma)
            .map(|token| token.location)
    }
}

impl ChunkedLevel {
    pub(crate) fn len(&self) -> usize {
        self.0.len()
    }

    pub(crate) fn parse_each_chunk<P>(
        &self,
        parent: &mut ItemCursor<'_>,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>> {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_chunk(
                    chunk,
                    parent.stream_chunk(&chunk.item),
                    leftover,
                    &parse_item,
                )
            })
            .collect()
    }
}

/// Unread or failed items from the chunk under parse.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnparsedChunkItemsParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedChunkItems(
    #[resolve_field]
    #[parent_variant(Unparsed)]
    pub NonEmpty<WithSpan<ChunkContentItem>>,
);

#[derive(Debug)]
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    ArgumentSlot(ArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    ListLiteralValueSlot(ListLiteralValueSlotPath<'a>),
}

pub type UnparsedChunkItemsPath<'a> =
    PositionResolutionPath<&'a UnparsedChunkItems, UnparsedChunkItemsParent<'a>>;

impl<'a> From<IsoLiteralSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::IsoLiteralSlot(path)
    }
}

impl<'a> From<ArgumentSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ArgumentSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ObjectEntrySlot(path)
    }
}

impl<'a> From<SelectionSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: SelectionSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::SelectionSlot(path)
    }
}

impl<'a> From<VariableDeclarationSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: VariableDeclarationSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::VariableDeclarationSlot(path)
    }
}

impl<'a> From<ListTypeAnnotationPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ListTypeAnnotationPath<'a>) -> Self {
        UnparsedChunkItemsParent::ListTypeAnnotation(path)
    }
}

impl<'a> From<ListLiteralValueSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: ListLiteralValueSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::ListLiteralValueSlot(path)
    }
}

/// Extra root chunks after the first.
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ExtraChunks(
    #[resolve_field]
    #[parent_variant(Extra)]
    pub NonEmpty<WithSpan<Chunk>>,
);

/// One parse attempt: an item plus leftover tokens in the same chunk.
///
/// `resolve` receives the list parent (`IsoLiteralParsePath` at the root, later
/// `SelectionSetPath` from a second pin).
/// Walk, given that parent:
/// - Position in `item`: bare `#[resolve_field]` passes `self.path(parent)`, a path
///   to this `Slot`. `T::Parent` is that path. `Slot` is a path segment.
/// - Position in `extra`: `#[parent_from]` converts the slot path
///   into leftover's parent enum.
/// - Position in the slot span but in neither field: `on_unmatched_span = from_path`
///   returns `self.path(parent).to()`. Each pin’s `From` builds that pin’s
///   `ResolvedNode` variant (`IsoLiteralSlot`, `ArgumentSlot`, `ObjectEntrySlot`).
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<Argument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
        (<VariableDeclaration, UnparsedChunkItems>, VariableDeclarationListPath<'a>),
        (<ListLiteralValue, UnparsedChunkItems>, ListLiteralPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    /// `Some` when the form parsed.
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    /// Unread or failed tokens after the item. Span is tight to those tokens.
    #[resolve_field]
    #[parent_from]
    pub extra: Option<WithSpan<E>>,
}

/// One-item level.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    pins = [
        (<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>, ()),
    ]
)]
pub struct Singleton<T, E> {
    #[resolve_field]
    pub item: WithSpan<T>,
    #[resolve_field]
    pub extra_chunks: Option<WithSpan<E>>,
}

fn parse_one_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    mut stream: ChunkStream<'a>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
) -> WithSpan<Slot<P, UnparsedChunkItems>> {
    let result = stream.cursor().spanning(parse);
    match result {
        Ok(item) => match stream.remaining_contents() {
            None => {
                let location = item.location;
                Slot {
                    item: item.wrap_some(),
                    extra: None,
                }
                .with_span(location)
            }
            Some(remaining) => {
                stream.cursor().report_error(
                    ParseError::expected(leftover, Found::from(remaining.first().item.reference()))
                        .with_span(remaining.first().location),
                );
                let leftover_span =
                    Span::join(remaining.first().location, remaining.last().location);
                let location = Span::join(item.location, leftover_span);
                Slot {
                    item: item.wrap_some(),
                    extra: UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some(),
                }
                .with_span(location)
            }
        },
        Err(reason) => {
            stream.cursor().report_error(reason);
            let location = chunk.item.contents_span();
            Slot {
                item: None,
                extra: UnparsedChunkItems(chunk.item.contents.clone())
                    .with_span(location)
                    .wrap_some(),
            }
            .with_span(location)
        }
    }
}

pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<SemanticToken>>,
    errors: &'a mut Vec<WithSpan<ParseError>>,
    end: Expectation,
    extra_chunks: impl FnOnce(&'a WithSpan<Chunk>) -> WithSpan<ParseError>,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<ParseError>>,
) -> Singleton<Slot<T, UnparsedChunkItems>, ExtraChunks> {
    let item = parse_one_chunk(
        &level.item.0[0],
        level.item.0[0].item.stream(text, tokens, errors),
        end,
        parse,
    );
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        errors.push(
            ParseError::expected(end, Found::Token(NonBracketTokenKind::Comma)).with_span(comma),
        );
    }
    let extra_chunks = (level.item.len() > 1).then(|| {
        errors.push(extra_chunks(&level.item.0[1]));
        let rest = NonEmpty {
            head: level.item.0[1].clone(),
            tail: level.item.0[2..].to_vec(),
        };
        let location = Span::join(rest.head.location, rest.last().location);
        ExtraChunks(rest).with_span(location)
    });
    Singleton { item, extra_chunks }
}

/// Chunk a matched-brackets tree. Every non-separator token lands in a chunk; a comma
/// no item precedes is the pass's one error, returned beside the tree; no grammar is
/// checked.
pub fn chunk(tree: &WithSpan<MatchedBrackets>) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let level = chunk_level(tree.item.reference(), &mut errors);
    (level.with_span(tree.location), errors)
}

fn chunk_level(level: &MatchedBrackets, errors: &mut Vec<CommaWithoutItem>) -> ChunkedLevel {
    let mut items = level.0.iter().safe_peekable();
    let mut out = Vec::new();
    while let Some(absorbed) = absorb_chunk(&mut items, errors) {
        match absorbed {
            Absorbed::Chunk(chunk) => out.push(chunk),
            Absorbed::CommaWithoutItem(comma) => errors.push(CommaWithoutItem(comma)),
        }
    }
    ChunkedLevel(out)
}

fn separator_token(kind: NonBracketTokenKind) -> Option<SeparatorToken> {
    match kind {
        NonBracketTokenKind::Comma => SeparatorToken::Comma.wrap_some(),
        NonBracketTokenKind::LineBreak => SeparatorToken::LineBreak.wrap_some(),
        _ => None,
    }
}

fn separator_of(item: &WithSpan<BracketItem>) -> Option<SeparatorToken> {
    match item.item.reference() {
        BracketItem::Raw(token) => separator_token(token.0),
        _ => None,
    }
}

/// One absorption: a chunk, or a comma no item precedes, whose boundary is consumed
/// and dropped.
enum Absorbed {
    Chunk(WithSpan<Chunk>),
    CommaWithoutItem(Span),
}

/// One absorption, or `None` at the stream's end. The first item is classified by
/// kind: a comma is the error, its following line breaks drained; a line break is
/// dropped and classification retries (the matcher already captured a level's leading
/// line breaks; this arm names the other separator so a leak is not reported as
/// `CommaWithoutItem`); otherwise the content phase runs from that item, then the
/// boundary phase, which stops before a second comma. Group interiors recurse in the
/// content phase's `Bracketed` arm.
fn absorb_chunk(
    items: &mut LevelItems<'_>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<Absorbed> {
    let peek = loop {
        let peek = items.peek()?;
        match separator_of(peek.view()) {
            Some(SeparatorToken::Comma) => {
                let comma = peek.commit().location;
                drain_dropped_boundary(items);
                return Absorbed::CommaWithoutItem(comma).wrap_some();
            }
            Some(SeparatorToken::LineBreak) => {
                peek.commit();
            }
            None => break peek,
        }
    };
    let first = match peek.view().item.reference() {
        BracketItem::Raw(token) => ChunkContentItem::NonBracket(*token),
        BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group, errors)),
    };
    let first_location = peek.commit().location;
    let mut span = first_location;
    let mut contents = NonEmpty::new(first.with_span(first_location));
    while let Some(peek) = items.peek() {
        let Some(content_item) = as_content(peek.view(), errors) else {
            break;
        };
        let item = peek.commit();
        span = Span::join(span, item.location);
        contents.push(content_item.with_span(item.location));
    }

    let mut separators: Option<NonEmpty<WithSpan<SeparatorToken>>> = None;
    while let Some(peek) = items.peek() {
        let Some(separator) = separator_of(peek.view()) else {
            break;
        };
        if separator == SeparatorToken::Comma
            && separators.as_ref().is_some_and(|absorbed| {
                absorbed
                    .iter()
                    .any(|token| token.item == SeparatorToken::Comma)
            })
        {
            break;
        }
        let item = peek.commit();
        let token = separator.with_span(item.location);
        match &mut separators {
            None => separators = NonEmpty::new(token).wrap_some(),
            Some(absorbed) => absorbed.push(token),
        }
    }

    let trailing_separator = separators.map(|separators| {
        let location = Span::join(separators.first().location, separators.last().location);
        ChunkSeparator(separators).with_span(location)
    });
    let span = match trailing_separator.reference() {
        Some(separator) => Span::join(span, separator.location),
        None => span,
    };
    Absorbed::Chunk(
        Chunk {
            contents,
            trailing_separator,
        }
        .with_span(span),
    )
    .wrap_some()
}

/// The content this item contributes, `None` when it is a separator. Group interiors
/// chunk here.
fn as_content(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<ChunkContentItem> {
    match item.item.reference() {
        BracketItem::Raw(token) => match separator_token(token.0) {
            Some(_) => None,
            None => ChunkContentItem::NonBracket(*token).wrap_some(),
        },
        BracketItem::Bracketed(group) => {
            ChunkContentItem::Group(chunk_group(group, errors)).wrap_some()
        }
    }
}

/// The rest of a dropped boundary: the line breaks after its comma, dropped with it. A
/// further comma is not absorbed; it opens the next absorption and its own error.
fn drain_dropped_boundary(items: &mut LevelItems<'_>) {
    while let Some(peek) = items.peek() {
        if separator_of(peek.view()) != SeparatorToken::LineBreak.wrap_some() {
            break;
        }
        peek.commit();
    }
}

fn chunk_group(group: &Bracketed, errors: &mut Vec<CommaWithoutItem>) -> ChunkedGroup {
    ChunkedGroup {
        opening: group.opening,
        children: chunk_level(group.children.item.reference(), errors)
            .with_span(group.children.location),
        closing: group.closing,
    }
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;
    use resolve_position::ResolvePosition;

    use super::*;
    use crate::{
        BracketError, BracketKind, Expectation, Found, NonBracketTokenKind, ParseError,
        SemanticToken, chunk_stream::ItemCursor, match_brackets, tokenize,
    };
    use BracketKind::Brace;
    use Expectation::Separator;
    use NonBracketTokenKind::{Identifier, Period};

    fn tree(literal: &str) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
        match_brackets(tokenize(literal), literal.len() as u32)
    }

    fn chunked(literal: &str) -> WithSpan<ChunkedLevel> {
        let (brackets, bracket_errors) = tree(literal);
        assert_eq!(bracket_errors, vec![]);
        let (chunked, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        chunked
    }

    /// For fixtures whose bracket tree is clean but whose chunking errs.
    fn chunked_with_commas(literal: &str) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
        let (brackets, bracket_errors) = tree(literal);
        assert_eq!(bracket_errors, vec![]);
        chunk(brackets.reference())
    }

    /// The span of `pattern`, which must occur exactly once in `text`: an anchor an edit
    /// cannot silently shift, and one that fails loudly when it stops being unique.
    fn span_of(text: &str, pattern: &str) -> Span {
        let mut occurrences = text.match_indices(pattern);
        let (offset, _) = occurrences
            .next()
            .expect("the pattern the test anchors on occurs in the literal");
        assert!(
            occurrences.next().is_none(),
            "the pattern the test anchors on occurs exactly once in the literal"
        );
        Span::from_usize(offset, offset + pattern.len())
    }

    fn chunk_span(chunk: &Chunk) -> Span {
        let contents = Span::join(
            chunk.contents.first().location,
            chunk.contents.last().location,
        );
        match chunk.trailing_separator.reference() {
            Some(separator) => Span::join(contents, separator.location),
            None => contents,
        }
    }

    fn render_chunk<'a>(literal: &'a str, chunk: &Chunk) -> &'a str {
        let span = chunk_span(chunk);
        &literal[span.start as usize..span.end as usize]
    }

    fn content_item(chunk: &Chunk, index: usize) -> &ChunkContentItem {
        chunk.contents[index].item.reference()
    }

    fn as_group(item: &ChunkContentItem) -> &ChunkedGroup {
        match item {
            ChunkContentItem::Group(group) => group,
            other => panic!("expected a group, got {other:?}"),
        }
    }

    fn as_non_bracket(item: &ChunkContentItem) -> NonBracketToken {
        match item {
            ChunkContentItem::NonBracket(token) => *token,
            other => panic!("expected a non-bracket token, got {other:?}"),
        }
    }

    fn separator_kinds(separator: &ChunkSeparator) -> Vec<SeparatorToken> {
        separator.0.iter().map(|s| s.item).collect()
    }

    #[test]
    fn a_selection_set_splits_on_commas_and_line_breaks() {
        let text = "foo { bar, baz\nqux }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 2);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        let brace = as_group(content_item(top, 1));
        assert_eq!(brace.opening.item.0, Brace);
        assert!(top.trailing_separator.is_none());

        let interior = brace.children.item.0.reference();
        assert_eq!(interior.len(), 3);
        assert_eq!(
            as_non_bracket(content_item(interior[0].item.reference(), 0)).0,
            NonBracketTokenKind::Identifier
        );
        assert_eq!(
            separator_kinds(
                interior[0]
                    .item
                    .trailing_separator
                    .as_ref()
                    .unwrap()
                    .item
                    .reference()
            ),
            SeparatorToken::Comma.wrap_vec()
        );
        assert_eq!(
            separator_kinds(
                interior[1]
                    .item
                    .trailing_separator
                    .as_ref()
                    .unwrap()
                    .item
                    .reference()
            ),
            SeparatorToken::LineBreak.wrap_vec()
        );
        assert!(interior[2].item.trailing_separator.is_none());
        assert_eq!(render_chunk(text, interior[0].item.reference()), "bar,");
        assert_eq!(render_chunk(text, interior[1].item.reference()), "baz\n");
        assert_eq!(render_chunk(text, interior[2].item.reference()), "qux");
    }

    #[test]
    fn multiple_groups_share_one_chunk_and_empty_interiors_are_zero_chunks() {
        let text = "foo { } { }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 3);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        let first = as_group(content_item(top, 1));
        let second = as_group(content_item(top, 2));
        assert_eq!(first.children.item.0.len(), 0);
        assert_eq!(second.children.item.0.len(), 0);
        assert!(top.trailing_separator.is_none());
    }

    #[test]
    fn an_empty_brace_group_has_an_empty_interior_level() {
        let text = "{}";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 1);
        let brace = as_group(content_item(top, 0));
        assert_eq!(brace.children.item.0.len(), 0);
        assert_eq!(
            brace.children.location,
            Span::new(span_of(text, "{").end, span_of(text, "}").start)
        );
    }

    #[test]
    fn commas_and_line_breaks_are_equivalent_separators() {
        let comma = chunked("a, b");
        let linebreak = chunked("a\nb");
        assert_eq!(comma.item.0.len(), 2);
        assert_eq!(linebreak.item.0.len(), 2);
        assert_eq!(
            separator_kinds(
                comma.item.0[0]
                    .item
                    .trailing_separator
                    .as_ref()
                    .unwrap()
                    .item
                    .reference()
            ),
            SeparatorToken::Comma.wrap_vec()
        );
        assert_eq!(
            separator_kinds(
                linebreak.item.0[0]
                    .item
                    .trailing_separator
                    .as_ref()
                    .unwrap()
                    .item
                    .reference()
            ),
            SeparatorToken::LineBreak.wrap_vec()
        );
        assert!(comma.item.0[1].item.trailing_separator.is_none());
        assert!(linebreak.item.0[1].item.trailing_separator.is_none());
    }

    #[test]
    fn captured_line_breaks_make_no_chunk() {
        let text = "\n\na, b\n";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        assert_eq!(render_chunk(text, tree.item.0[0].item.reference()), "a,");
        assert_eq!(tree.location, Span::from_usize(0, text.len()));

        let interior = "foo {\n bar\n}";
        let tree = chunked(interior);
        let brace = as_group(content_item(tree.item.0[0].item.reference(), 1));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(
            render_chunk(interior, brace.children.item.0[0].item.reference()),
            "bar\n"
        );
    }

    #[test]
    fn a_comma_before_the_first_item_is_an_error() {
        let text = "\n, a";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, CommaWithoutItem(span_of(text, ",")).wrap_vec());
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, chunked.item.0[0].item.reference()), "a");
    }

    #[test]
    fn a_second_comma_after_line_breaks_is_an_error() {
        let text = "a,\n\n,b";
        let (chunked, errors) = chunked_with_commas(text);
        let second_comma = span_of(text, ",b");
        assert_eq!(
            errors,
            CommaWithoutItem(Span::new(second_comma.start, second_comma.start + 1)).wrap_vec()
        );
        assert_eq!(chunked.item.0.len(), 2);
        assert_eq!(
            render_chunk(text, chunked.item.0[0].item.reference()),
            "a,\n\n"
        );
        assert_eq!(render_chunk(text, chunked.item.0[1].item.reference()), "b");
    }

    #[test]
    fn a_dropped_boundarys_line_break_goes_with_its_comma() {
        let text = "a,,\nb";
        let (chunked, errors) = chunked_with_commas(text);
        let commas = span_of(text, ",,");
        assert_eq!(
            errors,
            CommaWithoutItem(Span::new(commas.start + 1, commas.end)).wrap_vec()
        );
        assert_eq!(chunked.item.0.len(), 2);
        assert_eq!(render_chunk(text, chunked.item.0[0].item.reference()), "a,");
        assert_eq!(render_chunk(text, chunked.item.0[1].item.reference()), "b");
        match chunked.resolve(ChunkedLevelParent::Root, span_of(text, "\n")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn doubled_leading_commas_are_two_errors_in_order() {
        let text = ",,a";
        let (chunked, errors) = chunked_with_commas(text);
        let commas = span_of(text, ",,");
        assert_eq!(
            errors,
            vec![
                CommaWithoutItem(Span::new(commas.start, commas.start + 1)),
                CommaWithoutItem(Span::new(commas.start + 1, commas.end)),
            ]
        );
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, chunked.item.0[0].item.reference()), "a");
    }

    #[test]
    fn a_trailing_doubled_comma_keeps_the_item() {
        let text = "a, ,";
        let (chunked, errors) = chunked_with_commas(text);
        let anchor = span_of(text, ", ,");
        assert_eq!(
            errors,
            CommaWithoutItem(Span::new(anchor.end - 1, anchor.end)).wrap_vec()
        );
        assert_eq!(chunked.item.0.len(), 1);
        assert_eq!(render_chunk(text, chunked.item.0[0].item.reference()), "a,");
    }

    #[test]
    fn an_interior_comma_without_item_resolves_to_its_level() {
        let text = "foo {,}";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, CommaWithoutItem(span_of(text, ",")).wrap_vec());
        let top = chunked.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 2);
        let brace = as_group(content_item(top, 1));
        assert_eq!(brace.children.item.0.len(), 0);
        match chunked.resolve(ChunkedLevelParent::Root, span_of(text, ",")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    #[test]
    fn an_interior_comma_then_an_item_keeps_the_item() {
        let text = "{, a }";
        let (chunked, errors) = chunked_with_commas(text);
        assert_eq!(errors, CommaWithoutItem(span_of(text, ",")).wrap_vec());
        assert_eq!(chunked.item.0.len(), 1);
        let brace = as_group(content_item(chunked.item.0[0].item.reference(), 0));
        assert_eq!(brace.children.item.0.len(), 1);
        assert_eq!(
            render_chunk(text, brace.children.item.0[0].item.reference()),
            "a"
        );
    }

    #[test]
    fn a_list_trailing_comma_is_not_a_chunking_error() {
        let text = "foo { a, }";
        let chunked = chunked(text);
        let brace = as_group(content_item(chunked.item.0[0].item.reference(), 1));
        assert_eq!(brace.children.item.0.len(), 1);
    }

    #[test]
    fn a_captured_line_break_resolves_to_its_level() {
        let text = "\nfoo {\n bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, Span::new(0, 1)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        let opening = span_of(text, "{\n");
        match tree.resolve(
            ChunkedLevelParent::Root,
            Span::new(opening.start + 1, opening.end),
        ) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    #[test]
    fn non_separator_tokens_stay_in_one_chunk() {
        let text = "bar, baz watttt, qux";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        let middle = tree.item.0[1].item.reference();
        assert_eq!(middle.contents.len(), 2);
        assert_eq!(render_chunk(text, middle), "baz watttt,");
    }

    #[test]
    fn a_line_break_before_a_group_splits_the_field_from_its_selection_set() {
        let text = "foo\n{ bar }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 2);
        let first = tree.item.0[0].item.reference();
        assert_eq!(first.contents.len(), 1);
        assert_eq!(
            separator_kinds(first.trailing_separator.as_ref().unwrap().item.reference()),
            SeparatorToken::LineBreak.wrap_vec()
        );
        let second = tree.item.0[1].item.reference();
        assert_eq!(second.contents.len(), 1);
        as_group(content_item(second, 0));
        assert!(second.trailing_separator.is_none());
    }

    #[test]
    fn an_unmatched_close_rides_inside_a_chunk_and_errors_stay_on_the_bracket_tree() {
        let text = "a ) b";
        let (brackets, errors) = tree(text);
        match errors.as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.item.0, BracketKind::Parenthesis);
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        assert_eq!(tree.item.0.len(), 1);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 1);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        assert_eq!(top.contents[0].location, span_of(text, "a"));
        assert_eq!(render_chunk(text, top), "a");
    }

    #[test]
    fn an_unclosed_brace_demotes_to_raw_items_at_the_top() {
        let text = "foo { bar";
        let (brackets, errors) = tree(text);
        match errors.as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.item.0, Brace);
                assert_eq!(open.location, span_of(text, "{"));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        assert_eq!(tree.item.0.len(), 1);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents.len(), 1);
        assert_eq!(
            as_non_bracket(content_item(top, 0)).0,
            NonBracketTokenKind::Identifier
        );
        assert_eq!(top.contents[0].location, span_of(text, "foo"));
        assert_eq!(render_chunk(text, top), "foo");
    }

    #[test]
    fn whitespace_only_and_empty_literals_are_empty_levels() {
        for text in ["   ", "", "\n\n"] {
            let tree = chunked(text);
            assert_eq!(tree.item.len(), 0);
            assert_eq!(tree.location, Span::from_usize(0, text.len()));
        }
    }

    #[test]
    fn contents_span_stops_at_the_last_content_item() {
        let text = "foo,";
        let tree = chunked(text);
        let top = tree.item.0[0].item.reference();
        assert_eq!(top.contents_span(), span_of(text, "foo"));
        assert_eq!(top.boundary_comma(), span_of(text, ",").wrap_some());
        assert_eq!(top.first_item().location, span_of(text, "foo"));
        match top.first_item().item.reference() {
            ChunkContentItem::NonBracket(token) => {
                assert_eq!(token.0, NonBracketTokenKind::Identifier);
            }
            other => panic!("expected the identifier, got {other:?}"),
        }
    }

    #[test]
    fn a_chunk_without_a_comma_has_no_boundary_comma() {
        let text = "foo\nbar";
        let tree = chunked(text);
        assert_eq!(tree.item.len(), 2);
        assert_eq!(tree.item.0[0].item.boundary_comma(), None);
        assert_eq!(tree.item.0[1].item.boundary_comma(), None);
    }

    #[test]
    fn chunk_spans_are_tight_to_their_parts() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);
        let top = tree.item.0[0].reference();
        assert_eq!(top.location, chunk_span(top.item.reference()));
        assert_eq!(
            top.location,
            Span::new(span_of(text, "foo").start, span_of(text, "}").end)
        );
        let interior = as_group(content_item(top.item.reference(), 1))
            .children
            .item
            .0
            .reference();
        assert_eq!(interior[0].location, span_of(text, "bar,"));
        assert_eq!(
            interior[1].location,
            Span::new(span_of(text, "baz").start, span_of(text, "baz").end)
        );
    }

    #[test]
    fn a_matched_pair_resolves_with_its_group_as_parent() {
        let text = "foo { bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => {
                assert_eq!(open.parent.inner.closing.item.0, Brace);
                match open.parent.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => {
                        assert_eq!(render_chunk(text, chunk.inner), "foo { bar }");
                    }
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "}")) {
            IsographResolutionNode::CloseBracket(close) => {
                assert_eq!(close.parent.inner.opening.item.0, Brace);
            }
            node => panic!("expected the close bracket leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_span_straddling_a_groups_own_parts_resolves_to_the_group() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let straddle = Span::new(span_of(text, "{").start, span_of(text, "bar").end);
        match tree.resolve(ChunkedLevelParent::Root, straddle) {
            IsographResolutionNode::ChunkedGroup(group) => {
                assert_eq!(group.inner.opening.item.0, Brace);
            }
            node => panic!("expected the group leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_ordinary_token_resolves_to_its_own_leaf() {
        let text = "foo { bar }";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                match token.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => match chunk.parent.reference() {
                        ChunkParent::Level(level) => {
                            assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
                        }
                        parent => panic!("expected a level parent, got {parent:?}"),
                    },
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_inside_a_chunk_resolves_to_the_chunk() {
        let text = "foo { bar }";
        let tree = chunked(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(ChunkedLevelParent::Root, gap) {
            IsographResolutionNode::Chunk(chunk_path) => {
                assert_eq!(render_chunk(text, chunk_path.inner), "foo { bar }");
            }
            node => panic!("expected the chunk, got {node:?}"),
        }
    }

    #[test]
    fn resolution_walks_ancestry_against_source_text() {
        let text = "foo { bar, baz }";
        let tree = chunked(text);

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
                match token.parent.reference() {
                    ChunkContentItemParent::Chunk(chunk) => {
                        assert_eq!(render_chunk(text, chunk.inner), "bar,");
                        match chunk.parent.reference() {
                            ChunkParent::Level(level) => match level.parent.reference() {
                                ChunkedLevelParent::Interior(group) => {
                                    match group.parent.reference() {
                                        ChunkContentItemParent::Chunk(outer) => {
                                            assert_eq!(
                                                render_chunk(text, outer.inner),
                                                "foo { bar, baz }"
                                            );
                                        }
                                        parent => panic!("expected a chunk parent, got {parent:?}"),
                                    }
                                }
                                parent => panic!("expected an interior level, got {parent:?}"),
                            },
                            parent => panic!("expected a level parent, got {parent:?}"),
                        }
                    }
                    parent => panic!("expected a chunk parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "{")) {
            IsographResolutionNode::OpenBracket(open) => match open.parent.parent.reference() {
                ChunkContentItemParent::Chunk(chunk) => {
                    assert_eq!(render_chunk(text, chunk.inner), "foo { bar, baz }");
                }
                parent => panic!("expected a chunk parent, got {parent:?}"),
            },
            node => panic!("expected the open bracket leaf, got {node:?}"),
        }

        match tree.resolve(ChunkedLevelParent::Root, span_of(text, ",")) {
            IsographResolutionNode::ChunkSeparator(separator) => {
                assert_eq!(render_chunk(text, separator.parent.inner), "bar,");
                assert_eq!(
                    as_non_bracket(content_item(separator.parent.inner, 0)).0,
                    NonBracketTokenKind::Identifier
                );
            }
            node => panic!("expected the separator, got {node:?}"),
        }

        let gap_in_chunk = Span::new(span_of(text, "foo").end, span_of(text, "{").start);
        match tree.resolve(ChunkedLevelParent::Root, gap_in_chunk) {
            IsographResolutionNode::Chunk(chunk_path) => {
                assert_eq!(render_chunk(text, chunk_path.inner), "foo { bar, baz }");
            }
            node => panic!("expected the chunk, got {node:?}"),
        }

        let gap_between = Span::new(span_of(text, "bar,").end, span_of(text, "baz").start);
        match tree.resolve(ChunkedLevelParent::Root, gap_between) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }

        let text_ws = " foo { bar, baz }";
        let tree_ws = chunked(text_ws);
        let leading_ws = Span::new(0, span_of(text_ws, "foo").start);
        match tree_ws.resolve(ChunkedLevelParent::Root, leading_ws) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn an_empty_interior_resolves_to_the_interior_level() {
        // A zero-width interior (`foo {}`) has no position that is not also the opening's
        // end / closing's start under Span::contains, so the case that can be asked is a
        // space inside the braces: the level has zero chunks and answers itself.
        let text = "foo { }";
        let tree = chunked(text);
        let space = Span::new(span_of(text, "{").end, span_of(text, "}").start);
        match tree.resolve(ChunkedLevelParent::Root, space) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_only_resolves_to_the_root_level() {
        let text = "   ";
        let tree = chunked(text);
        match tree.resolve(ChunkedLevelParent::Root, Span::new(1, 2)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        let empty = chunked("");
        match empty.resolve(ChunkedLevelParent::Root, Span::new(0, 0)) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn a_dropped_close_and_the_text_after_it_resolve_to_the_root_level() {
        let text = "a ) b";
        let (brackets, errors) = tree(text);
        match errors.as_slice() {
            [BracketError::UnmatchedClose(close)] => {
                assert_eq!(close.location, span_of(text, ")"));
            }
            errors => panic!("expected exactly the unmatched close, got {errors:?}"),
        }
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, ")")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "b")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Root));
            }
            node => panic!("expected the root level, got {node:?}"),
        }
    }

    #[test]
    fn a_dropped_open_inside_a_matched_brace_resolves_to_the_interior_level() {
        let text = "foo { ( }";
        let (brackets, errors) = tree(text);
        match errors.as_slice() {
            [BracketError::UnmatchedOpen(open)] => {
                assert_eq!(open.location, span_of(text, "("));
            }
            errors => panic!("expected exactly the unmatched open, got {errors:?}"),
        }
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![]);
        match tree.resolve(ChunkedLevelParent::Root, span_of(text, "(")) {
            IsographResolutionNode::ChunkedLevel(level) => {
                assert!(matches!(level.parent, ChunkedLevelParent::Interior(_)));
            }
            node => panic!("expected the interior level, got {node:?}"),
        }
    }

    fn parse_identifier(cursor: &mut ItemCursor<'_>) -> Result<Span, WithSpan<ParseError>> {
        cursor
            .require_token(Identifier, SemanticToken::FieldName)
            .map_err(|()| cursor.expected(Expectation::Token(Identifier)))
            .map(|token| token.location)
    }

    type ParsedEach = (
        Vec<WithSpan<Slot<Span, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed_each(text: &str) -> ParsedEach {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let dummy = chunked("x");
        let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let items = tree.item.parse_each_chunk(
            parent.cursor(),
            Separator(BracketKind::Parenthesis),
            parse_identifier,
        );
        (items, errors, comma_errors, tokens)
    }

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    #[test]
    fn parse_each_chunk_on_an_empty_level_is_no_slots_and_no_errors() {
        for text in ["", "   ", "\n\n"] {
            let (items, errors, comma_errors, tokens) = parsed_each(text);
            assert_eq!(items, vec![], "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            assert_eq!(tokens, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn parse_each_chunk_parses_one_identifier_per_chunk() {
        let text = "foo, bar";
        let (items, errors, comma_errors, tokens) = parsed_each(text);
        assert_eq!(comma_errors, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
        assert_eq!(
            items[1].item.item.as_ref().map(|item| item.item),
            span_of(text, "bar").wrap_some(),
        );
        assert!(items[0].item.extra.is_none());
        assert!(items[1].item.extra.is_none());
        assert_eq!(
            tokens,
            vec![
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
                SemanticToken::FieldName.with_span(span_of(text, "bar")),
            ],
        );
    }

    #[test]
    fn a_list_trailing_comma_is_not_a_parse_each_chunk_diagnostic() {
        let text = "foo,";
        let (items, errors, comma_errors, _) = parsed_each(text);
        assert_eq!(comma_errors, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
        assert!(items[0].item.extra.is_none());
    }

    #[test]
    fn leftover_after_a_list_item_keeps_the_item() {
        let text = "foo bar";
        let (items, errors, comma_errors, tokens) = parsed_each(text);
        assert_eq!(comma_errors, vec![]);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
        assert!(items[0].item.extra.as_ref().is_some());
        assert_eq!(
            errors,
            expected(
                Separator(BracketKind::Parenthesis),
                Found::Token(Identifier)
            )
            .with_span(span_of(text, "bar"))
            .wrap_vec(),
        );
        assert_eq!(
            tokens,
            SemanticToken::FieldName
                .with_span(span_of(text, "foo"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_failed_list_chunk_is_none_and_the_next_chunk_still_parses() {
        let text = ".\nfoo";
        let (items, errors, comma_errors, tokens) = parsed_each(text);
        assert_eq!(comma_errors, vec![]);
        assert_eq!(items.len(), 2);
        assert!(items[0].item.item.is_none());
        assert!(items[0].item.extra.as_ref().is_some());
        assert_eq!(
            items[1].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
        assert!(errors.iter().any(|error| {
            error.item == expected(Expectation::Token(Identifier), Found::Token(Period))
                && error.location == span_of(text, ".")
        }));
        assert_eq!(
            tokens,
            SemanticToken::FieldName
                .with_span(span_of(text, "foo"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_line_break_is_a_list_separator() {
        let text = "foo\nbar";
        let (items, errors, comma_errors, _) = parsed_each(text);
        assert_eq!(comma_errors, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
        assert_eq!(
            items[1].item.item.as_ref().map(|item| item.item),
            span_of(text, "bar").wrap_some(),
        );
    }

    #[test]
    fn a_comma_without_item_is_chunkings_error_and_the_item_parses() {
        let text = ",foo";
        let (items, errors, comma_errors, _) = parsed_each(text);
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 1);
        assert_eq!(
            items[0].item.item.as_ref().map(|item| item.item),
            span_of(text, "foo").wrap_some(),
        );
    }
}
