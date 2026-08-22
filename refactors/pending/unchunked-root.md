# Unchunked root

A level inside brackets is partitioned (`chunk_level`). A level that is not is an item sequence (`to_content` on each `BracketItem`). The iso literal is the latter. There is no other root-only rule.

```
// from demos/pet-demo/src/components/HomeRoute.tsx
iso(`
  field Query.HomeRoute @component
  """
  Show a list of pets
  """
  {
    pets {
      id
      PetSummaryCard
    }
  }
`)
```

```
// from demos/vite-demo/src/components/HomePage.tsx
iso(`
  field Query.HomePage @component
  """
  Gets the first 150 pokemon, take is higher because there are alternative forms
  returned and the offset skips a bunch of other Pokemon that aren't in the first 150
  """
  {
    getAllPokemon(take: 232, offset: 93) {
      key
      forme
      Pokemon
    }
  }
`)
```

The description and the selection set sit on later lines than `field Query.HomeRoute @component`. Entrypoints in the same demos are one line, `iso(\`entrypoint Query.HomeRoute\`)`, with no comma in front of `entrypoint`.

Line breaks in the unpartitioned sequence are `NonBracket(LineBreak)`. The declaration parser consumes a run of them before the keyword, before the description, and before the selection set. A comma is still a comma: before `field` / `entrypoint`, `Expected(DECLARATION_KEYWORD, Comma)`.

A list interior still partitions on commas and line breaks. `{ pets {` / `id` / `PetSummaryCard` } is three selections. `{ foo\n{ bar } }` is a scalar plus a failed selection. `[Pet\n!]` does not attach the bang.

## Tree

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type IsoLiteralParse = Slot<IsoLiteralItem, UnparsedChunkItems>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;
```

`IsoLiteralSlotPath` is this alias. The `From` into `IsographResolutionNode` stays `IsoLiteralSlot`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl<'a> From<IsoLiteralParsePath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralParsePath<'a>) -> Self {
        IsographResolutionNode::IsoLiteralSlot(path)
    }
}
```

`IsoLiteralItem`, `EntrypointDeclaration`, and `SelectableDeclaration` take `parent_type = IsoLiteralParsePath<'a>`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralParsePath<'a>>;

pub type SelectableDeclarationPath<'a> =
    PositionResolutionPath<&'a SelectableDeclaration, IsoLiteralParsePath<'a>>;
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = (),
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
)]
pub struct ChunkedRoot(
    #[resolve_field]
    #[parent_variant(Root)]
    pub Vec<WithSpan<ChunkContentItem>>,
);

pub type ChunkedRootPath<'a> = PositionResolutionPath<&'a ChunkedRoot, ()>;
```

Empty vec is `""` / `"   "`. Otherwise every item of the unpartitioned sequence, line breaks and commas included. A group in that vec has a `ChunkedLevel` interior.

A `Chunk` exists only inside a `ChunkedLevel`. `ChunkParent` / `Extra` / `Root`-as-chunk-parent are gone.

```rust
// from crates/isograph_parser/src/chunk.rs
pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkedLevelPath<'a>>;
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum ChunkContentItemParent<'a> {
    Root(ChunkedRootPath<'a>),
    Chunk(ChunkPath<'a>),
    Unparsed(UnparsedChunkItemsPath<'a>),
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedGroupPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ChunkedLevel(
    #[resolve_field]
    #[parent_variant(Level)]
    pub Vec<WithSpan<Chunk>>,
);

pub type ChunkedLevelPath<'a> =
    PositionResolutionPath<&'a ChunkedLevel, ChunkedGroupPath<'a>>;
```

`ChunkedLevelParent` is gone. A `ChunkedLevel` is a group's interior.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path,
    pins = [
        (<IsoLiteralItem, UnparsedChunkItems>, ()),
        (<Argument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
        (<VariableDeclaration, UnparsedChunkItems>, VariableDeclarationListPath<'a>),
        (<ListLiteralValue, UnparsedChunkItems>, ListLiteralPath<'a>),
    ]
)]
pub struct Slot<T, E> {
    #[resolve_field]
    pub item: Option<WithSpan<T>>,
    #[resolve_field]
    #[parent_from]
    pub extra: Option<WithSpan<E>>,
}
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralParsePath<'a>),
    ArgumentSlot(ArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    ListLiteralValueSlot(ListLiteralValueSlotPath<'a>),
}

impl<'a> From<IsoLiteralParsePath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: IsoLiteralParsePath<'a>) -> Self {
        UnparsedChunkItemsParent::IsoLiteralSlot(path)
    }
}
```

`Singleton` is gone. `ExtraChunks` is gone. `ExtraChunksPath` is gone.

Delta from the landed `IsographResolutionNode`: drop `Singleton` and `ExtraChunks`; add `ChunkedRoot`; `IsoLiteralSlot`'s payload is `IsoLiteralParsePath` (the slot, parent `()`). `ChunkedLevelPath`'s parent is `ChunkedGroupPath`. Every other variant is unchanged.

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
pub enum IsographResolutionNode<'a> {
    IsoLiteralSlot(IsoLiteralParsePath<'a>),
    ChunkedRoot(ChunkedRootPath<'a>),
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Description(DescriptionPath<'a>),
    EntityNameWrapper(EntityNameWrapperPath<'a>),
    SelectableNameWrapper(SelectableNameWrapperPath<'a>),
    UnparsedChunkItems(UnparsedChunkItemsPath<'a>),
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
    ArgumentSlot(ArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    Argument(ArgumentPath<'a>),
    ObjectEntry(ObjectEntryPath<'a>),
    ArgumentNameWrapper(ArgumentNameWrapperPath<'a>),
    ValueKeyNameWrapper(ValueKeyNameWrapperPath<'a>),
    VariableUse(VariableUsePath<'a>),
    VariableNameWrapper(VariableNameWrapperPath<'a>),
    VariableDeclarationOrUsage(VariableDeclarationOrUsagePath<'a>),
    VariableDeclarationSlot(VariableDeclarationSlotPath<'a>),
    VariableDeclarationList(VariableDeclarationListPath<'a>),
    VariableDeclaration(VariableDeclarationPath<'a>),
    StringLiteralValueWrapper(StringLiteralValueWrapperPath<'a>),
    IntegerValue(IntegerValuePath<'a>),
    BooleanValue(BooleanValuePath<'a>),
    NullValue(NullValuePath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    Selection(SelectionPath<'a>),
    SelectionNameWrapper(SelectionNameWrapperPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
    ListTypeAnnotation(ListTypeAnnotationPath<'a>),
    UnionTypeAnnotation(UnionTypeAnnotationPath<'a>),
    IsographFieldDirectiveList(IsographFieldDirectiveListPath<'a>),
    IsographFieldDirective(IsographFieldDirectivePath<'a>),
    IsographDirectiveNameWrapper(IsographDirectiveNameWrapperPath<'a>),
    ListLiteral(ListLiteralPath<'a>),
    ListLiteralValue(ListLiteralValuePath<'a>),
    ListLiteralValueSlot(ListLiteralValueSlotPath<'a>),
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
impl<'a> From<ChunkedRootPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ChunkedRootPath<'a>) -> Self {
        IsographResolutionNode::ChunkedRoot(path)
    }
}
```

`AstError::MultipleDeclarations` is gone.

## Chunk

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn chunk(
    tree: &WithSpan<MatchedBrackets>,
) -> (WithSpan<ChunkedRoot>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let contents = tree
        .item
        .0
        .iter()
        .map(|item| to_content(item, &mut errors).with_span(item.location))
        .collect();
    (ChunkedRoot(contents).with_span(tree.location), errors)
}

fn to_content(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> ChunkContentItem {
    match item.item.reference() {
        BracketItem::Raw(token) => ChunkContentItem::NonBracket(*token),
        BracketItem::Bracketed(group) => ChunkContentItem::Group(chunk_group(group, errors)),
    }
}

fn as_content(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<ChunkContentItem> {
    match separator_of(item) {
        Some(_) => None,
        None => to_content(item, errors).wrap_some(),
    }
}
```

`chunk` converts an unpartitioned `MatchedBrackets` (the iso literal). `chunk_group` still runs `chunk_level` on `group.children`. `absorb_chunk` uses `as_content`, which is `to_content` except separators. `CommaWithoutItem` is produced only by `chunk_level`.

Before, `as_content` inlined the `Raw` / `Bracketed` match and returned `None` for a separator:

```rust
// from crates/isograph_parser/src/chunk.rs
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
```

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ChunkedLevelPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Chunk {
    #[resolve_field]
    #[parent_variant(Chunk)]
    contents: NonEmpty<WithSpan<ChunkContentItem>>,
    #[resolve_field]
    trailing_separator: Option<WithSpan<ChunkSeparator>>,
}
```

`#[parent_variant(Chunk)]` is `ChunkContentItemParent::Chunk`.

```rust
// from crates/isograph_parser/src/matched_brackets.rs
pub(crate) fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
    literal_length: u32,
) -> (WithSpan<MatchedBrackets>, Vec<BracketError>) {
    let mut tokens = tokens.into_iter().safe_peekable();
    let mut enclosing_stack = Stack::new();
    let mut errors = Vec::new();
    let items = parse_bracket_items(&mut tokens, &mut enclosing_stack, &mut errors);
    errors.sort_by_key(|error| match error {
        BracketError::UnmatchedOpen(open) => open.location.start,
        BracketError::UnmatchedClose(close) => close.location.start,
    });
    (
        MatchedBrackets(items).with_span(Span::new(0, literal_length)),
        errors,
    )
}
```

`strip_captured_line_breaks` runs in `parse_bracketed` after the group closes: the opening captured those line breaks. `match_brackets` does not call it. The newline after `iso(\`` is a `LineBreak` in the unpartitioned sequence.

## `consume_line_breaks`

Line breaks are legal before the keyword, before the description, and before the selection set. Those three sites (plus trailing after the declaration so a final newline is not leftover) call `consume_line_breaks`. Zero is legal (`iso(\`entrypoint Query.foo\`)`). It is `consume_*`, not `require_*`: a missing run is not an error. `advance`, not `commit`. No semantic token. A position on a consumed line break answers the containing node.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
pub(crate) struct ItemCursor<'a> {
    items: SafePeekable<std::slice::Iter<'a, WithSpan<ChunkContentItem>>>,
    previous_end: u32,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<IsographSemanticToken>>,
    errors: &'a mut Vec<WithSpan<AstError>>,
}

impl<'a> ChunkStream<'a> {
    pub(crate) fn new(
        contents: &'a [WithSpan<ChunkContentItem>],
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<IsographSemanticToken>>,
        errors: &'a mut Vec<WithSpan<AstError>>,
    ) -> Self {
        ChunkStream(ItemCursor {
            previous_end: match contents.first() {
                Some(item) => item.location.start,
                None => 0,
            },
            items: contents.iter().safe_peekable(),
            text,
            tokens,
            errors,
        })
    }
}

    pub(crate) fn consume_line_breaks(&mut self) {
        while let Some(peek) = self.peek() {
            match peek.view().item.reference() {
                ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::LineBreak)) => {
                    peek.advance();
                }
                _ => break,
            }
        }
    }
```

`Chunk::stream` is `ChunkStream::new(self.contents.as_slice(), text, tokens, errors)`. `peek`, `consume_token_if`, and `consume_group_if` do not eat line breaks. A comma is still there.

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn parse_stream<'a, P>(
    mut stream: ChunkStream<'a>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
    failed_extra: impl FnOnce() -> NonEmpty<WithSpan<ChunkContentItem>>,
) -> WithSpan<Slot<P, UnparsedChunkItems>> {
    let result = stream.cursor().spanning(parse);
    let slot = match result {
        Ok(item) => {
            let extra = match stream.remaining_contents() {
                None => None,
                Some(remaining) => {
                    stream.cursor().report_error(
                        AstError::expected(
                            leftover,
                            Found::from(remaining.first().item.reference()),
                        )
                        .with_span(remaining.first().location),
                    );
                    let leftover_span =
                        Span::join(remaining.first().location, remaining.last().location);
                    UnparsedChunkItems(remaining)
                        .with_span(leftover_span)
                        .wrap_some()
                }
            };
            let location = match extra.as_ref() {
                None => item.location,
                Some(extra) => Span::join(item.location, extra.location),
            };
            Slot {
                item: item.wrap_some(),
                extra,
            }
            .with_span(location)
        }
        Err(reason) => {
            stream.cursor().report_error(reason);
            let extra = match stream.remaining_contents() {
                Some(remaining) => remaining,
                None => failed_extra(),
            };
            let location = Span::join(extra.first().location, extra.last().location);
            Slot {
                item: None,
                extra: UnparsedChunkItems(extra).with_span(location).wrap_some(),
            }
            .with_span(location)
        }
    };
    record_leftover_extra(stream.tokens(), &slot.item.extra);
    slot
}

fn parse_one_chunk<'a, P>(
    chunk: &'a WithSpan<Chunk>,
    stream: ChunkStream<'a>,
    leftover: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<P, WithSpan<AstError>>,
) -> WithSpan<Slot<P, UnparsedChunkItems>> {
    let mut slot = parse_stream(stream, leftover, parse, || chunk.item.contents.clone());
    match leftover {
        Expectation::Separator(_) => slot,
        _ => {
            slot.item.extra = extra_plus_trailing_separator(
                slot.item.extra,
                chunk.item.trailing_separator.as_ref(),
            );
            slot
        }
    }
}
```

`parse_stream` parses a content slice. It does not consume line breaks. `parse_one_chunk` is that plus `extra_plus_trailing_separator` when leftover is not a list `Separator`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<AstError>> {
    cursor.consume_line_breaks();
    let keyword = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::Keyword,
        )
        .map_err(|()| cursor.expected(DECLARATION_KEYWORD))?;
    match keyword.text() {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Selectable(parse_selectable_declaration(cursor)?).wrap_ok(),
        _ => AstError::expected(
            DECLARATION_KEYWORD,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
    }
}

fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<AstError>> {
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    let directive_set = consume_directives(cursor)?;
    cursor.consume_line_breaks();
    EntrypointDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        directive_set,
    }
    .wrap_ok()
}

fn parse_selectable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectableDeclaration, WithSpan<AstError>> {
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let target_type = consume_to_target(cursor)?;
    let directive_set = consume_directives(cursor)?;
    cursor.consume_line_breaks();
    let description = consume_description(cursor);
    cursor.consume_line_breaks();
    let selection_set = consume_selection_set(cursor);
    cursor.consume_line_breaks();
    SelectableDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        target_type,
        directive_set,
        description,
        selection_set,
    }
    .wrap_ok()
}
```

`parse_type_dot_name`, `consume_to_target`, `consume_description`, `consume_selection_set`, `consume_directives`, `consume_variable_declaration_list`, and the nested list parsers stay as landed. They do not call `consume_line_breaks`.

The `consume_line_breaks` after `consume_selection_set` (and after an entrypoint's directives) eats trailing line breaks after `}` so they are not leftover. `field Query.Foo\nto User` does not consume line breaks before `to`: `consume_to_target` sees the line break, not `to`. Same for a directive or variable list on the next line.

## Parse of the unpartitioned sequence

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn parse_chunked_iso_literal(
    text: &str,
    root: WithSpan<ChunkedRoot>,
    errors: &mut Vec<WithSpan<AstError>>,
    tokens: &mut Vec<WithSpan<IsographSemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    let Some((head, tail)) = root.item.0.split_first() else {
        errors.push(AstError::EmptyLiteral.with_span(location));
        return None;
    };
    let failed_extra = NonEmpty {
        head: head.clone(),
        tail: tail.to_vec(),
    };
    let mut stream = ChunkStream::new(root.item.0.as_slice(), text, tokens, errors);
    stream.cursor().consume_line_breaks();
    if stream.require_end().is_ok() {
        errors.push(AstError::EmptyLiteral.with_span(location));
        return None;
    }
    let slot = parse_stream(
        stream,
        Expectation::EndOfDeclaration,
        parse_iso_literal_item,
        || failed_extra,
    );
    slot.with_span(location).wrap_some()
}
```

Empty is `root.item.0.split_first()`. `None` is `EmptyLiteral` before the stream. `Some((head, tail))` builds the `failed_extra` `NonEmpty` without an index.

`require_end` loses `#[cfg_attr(not(test), expect(dead_code))]`. After `consume_line_breaks`, a stream at end is `"\n\n"`. `parse_iso_literal_item` consumes line breaks again (no-op) then requires the keyword.

`""` and `"   "` are `ChunkedRoot(vec![])`. `"\n\n"` is two `LineBreak` items, `EmptyLiteral`, `None`. `",entrypoint Query.foo"`: `consume_line_breaks` does not eat the comma, fails at the comma, `Expected(DECLARATION_KEYWORD, Comma)`.

`parse_stream` and `parse_one_chunk` are `pub(crate)`. The returned `WithSpan` is the whole literal, so a position on a consumed line break or on a consumed keyword of a failed form (`fieldd`) is unmatched on the slot and answers `IsoLiteralSlot`. `item` and `extra` keep the tight spans `parse_stream` assigned.

Leftover after a complete declaration is `Slot.extra` plus `report_error(Expected(EndOfDeclaration, found))` at the first leftover item that is not a line break: a trailing comma, a second `field` / `entrypoint`.

```rust
// from crates/isograph_parser/src/lib.rs
pub(crate) use chunk::{chunk, parse_one_chunk, parse_singleton, parse_stream};
```

`parse_iso_literal` still match-brackets, then `chunk`, then this. `comma_errors` still become `ParseError::Comma`; they come from interiors.

## `parse_singleton`

One caller: `[...]` type interiors via `ItemCursor::parse_nested_singleton`. Extra chunks are an `Expected(end, found)` error and leftover-token recording. They are not a tree node.

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn parse_singleton<'a, T>(
    level: &'a WithSpan<ChunkedLevel>,
    text: &'a str,
    tokens: &'a mut Vec<WithSpan<IsographSemanticToken>>,
    errors: &'a mut Vec<WithSpan<AstError>>,
    end: Expectation,
    parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<AstError>>,
) -> WithSpan<Slot<T, UnparsedChunkItems>> {
    let item = parse_one_chunk(
        &level.item.0[0],
        level.item.0[0].item.stream(text, tokens, errors),
        end,
        parse,
    );
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        errors.push(
            AstError::expected(end, Found::Token(NonBracketTokenKind::Comma)).with_span(comma),
        );
    }
    if let Some(extra) = level.item.0.get(1) {
        errors.push(
            AstError::expected(end, Found::from(extra.item.first_item().item.reference()))
                .with_span(extra.location),
        );
        for chunk in level.item.0[1..].iter() {
            record_leftover_chunk(tokens, chunk.item.reference());
        }
    }
    item
}
```

Empty is still the caller (`parse_bracket_interior_type` on `len() == 0`). The `[0]` is the same index as today.

```rust
// from crates/isograph_parser/src/chunk_stream.rs
    pub(crate) fn parse_nested_singleton<T>(
        &mut self,
        level: &WithSpan<ChunkedLevel>,
        end: Expectation,
        parse: impl FnOnce(&mut ItemCursor<'_>) -> Result<T, WithSpan<AstError>>,
    ) -> WithSpan<Slot<T, UnparsedChunkItems>> {
        parse_singleton(level, self.text, self.tokens, self.errors, end, parse)
    }
```

```rust
// from crates/isograph_parser/src/variables.rs
    let slot = cursor.parse_nested_singleton(
        level,
        Expectation::EndOfType,
        parse_type_annotation,
    );
    BracketInteriorType {
        item: slot.item.item.map(|wrapped| wrapped.item),
        extra: slot.item.extra,
    }
    .wrap_ok()
```

`T` is `WithSpan<TypeAnnotation>` because `parse_type_annotation` returns `WithSpan`. The `map` peels `parse_one_chunk`'s extra `WithSpan`, same as today.

## `require_complete_literal`

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub fn require_complete_literal(parse: &IsoLiteralParse) -> Option<&IsoLiteralItem> {
    require_complete(parse)
}
```

slot-stages.md's `extra_chunks` check is this: leftover after the declaration is `Slot.extra`.

---

# Changes

## 1. Interior fixtures for list-splitting

`cargo test -p isograph_parser`. Additive. Root fixtures stay. Each new test is the same assertion as the named root test, inside `{ ... }`.

```rust
// from crates/isograph_parser/src/chunk.rs
    #[test]
    fn an_interior_splits_on_commas_and_line_breaks_the_same_way() {
        let text = "{ a, b }";
        let comma = chunked(text);
        let brace = as_group(content_item(comma.item.0[0].item.reference(), 0));
        assert_eq!(brace.children.item.0.len(), 2);

        let text = "{ a\nb }";
        let linebreak = chunked(text);
        let brace = as_group(content_item(linebreak.item.0[0].item.reference(), 0));
        assert_eq!(brace.children.item.0.len(), 2);
    }

    #[test]
    fn an_interior_line_break_before_a_group_splits_the_selection() {
        let text = "{ foo\n{ bar } }";
        let tree = chunked(text);
        let brace = as_group(content_item(tree.item.0[0].item.reference(), 0));
        assert_eq!(brace.children.item.0.len(), 2);
        as_non_bracket(content_item(brace.children.item.0[0].item.reference(), 0));
        as_group(content_item(brace.children.item.0[1].item.reference(), 0));
    }

    #[test]
    fn an_interior_non_separator_run_stays_in_one_chunk() {
        let text = "{ bar, baz watttt, qux }";
        let tree = chunked(text);
        let brace = as_group(content_item(tree.item.0[0].item.reference(), 0));
        assert_eq!(brace.children.item.0.len(), 3);
        let middle = brace.children.item.0[1].item.reference();
        assert_eq!(middle.contents.len(), 2);
        assert_eq!(render_chunk(text, middle), "baz watttt,");
    }
```

The existing interior comma tests (`foo {,}`, `{, a }`, `{ a, }`) stay. If a root comma-without-item test has no interior twin, add one: `{, a }`, `{ a,,\nb }`, `{ a, , }`, `{,,a }`.

## 2. Partition only group interiors

One commit. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.

### `chunk` and matcher

`chunk` / `to_content` / `as_content` / `ChunkedRoot` as above. `chunk_level` is `chunk_group` only. `strip_captured_line_breaks` stays in `parse_bracketed`.

### Types

`ChunkedRoot`, `ChunkedRootPath`, `ChunkParent`, `ChunkedLevel` parent, `Slot` pins, `UnparsedChunkItemsParent`, `IsoLiteralParse`, `IsographResolutionNode` as above.

Delete `Singleton`, `ExtraChunks`, `ChunkedLevelParent`, `ChunkParent`, `AstError::MultipleDeclarations`. The `ast_error_unit_variants_use_their_messages` arm for `MultipleDeclarations` goes with it.

`lib.rs` `pub use` drops `ExtraChunks`, `Singleton`, `ExtraChunksPath`, `ChunkedLevelParent`, `ChunkParent`. It adds `ChunkedRoot`, `ChunkedRootPath`. `IsoLiteralSlotPath` is deleted; callers use `IsoLiteralParsePath`. `ChunkContentItemParent` gains `Root`.

### Parse

`consume_line_breaks`, `parse_stream`, `parse_one_chunk`, `parse_chunked_iso_literal`, and the `parse_*` / `consume_*` listings above.

Helpers in `parse_iso_literal.rs` tests:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn first_slot(parse: &WithSpan<IsoLiteralParse>) -> &Slot<IsoLiteralItem, UnparsedChunkItems> {
        parse.item.reference()
    }
```

`chunked` in both test modules returns `WithSpan<ChunkedRoot>`. `stream_of` is `ChunkStream::new(tree.item.0.as_slice(), text, tokens, errors)`. Change 1's interior fixtures that indexed `tree.item.0[0]` as a root chunk become `as_group(&tree.item.0[0].item)` for `{ ... }` at the root.

### Chunk tests, after

`whitespace_only_and_empty_literals_are_empty_levels`: `""` and `"   "` have `tree.item.0` empty. `"\n\n"` has two `LineBreak` items. Span is still the whole literal.

`a_line_break_before_a_group_stays_in_the_unpartitioned_sequence`:

```rust
// from crates/isograph_parser/src/chunk.rs
    fn a_line_break_before_a_group_stays_in_the_unpartitioned_sequence() {
        let text = "foo\n{ bar }";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 3);
        as_non_bracket(&tree.item.0[0].item);
        assert_eq!(
            as_non_bracket(&tree.item.0[1].item).0,
            NonBracketTokenKind::LineBreak
        );
        as_group(&tree.item.0[2].item);
    }
```

`commas_and_line_breaks_are_equivalent_separators` at the root: `"a, b"` is identifier, comma, identifier; `"a\nb"` is identifier, line break, identifier. Neither is two chunks. Interior equivalence is change 1.

`captured_line_breaks_make_no_chunk` at the root: `"\n\na, b\n"` is leading line breaks, `a`, comma, `b`, trailing line break, one vec. Interior `"foo {\n bar\n}"` is unchanged (the `{` still captures the interior leading newline).

Root comma-without-item tests (`"\n, a"`, `"a,\n\n,b"`, `"a,,\nb"`, `",,a"`, `"a, ,"`): no `CommaWithoutItem`. The commas and line breaks are content of the root vec. Interior twins from change 1 keep the errors.

`contents_span_stops_at_the_last_content_item` is a `Chunk` method. It still applies to interior chunks (`{ foo, }`'s `foo,`). At the root, `"foo,"` is two content items, no `trailing_separator`.

`a_chunk_without_a_comma_has_no_boundary_comma` for `"foo\nbar"` at the root is not a chunk. Interior `{ foo\nbar }` still has two chunks and no boundary comma on the second.

`a_captured_line_break_resolves_to_its_level` at offset 0 of `"\nfoo {\n bar }"`: `NonBracketToken` (`LineBreak`) with parent `ChunkContentItemParent::Root`. The `{`'s captured interior newline is still `ChunkedLevel`.

`whitespace_only_resolves_to_the_root_level`: `"   "` and `""` are `ChunkedRoot`. `"\n"` is the `LineBreak` token with parent `Root`.

`a_dropped_close_and_the_text_after_it_resolve_to_the_root_level`: `ChunkedRoot`.

Resolve ancestry that matched `ChunkedLevelParent::Root` matches `ChunkContentItemParent::Root` on a root item, or `ChunkedRoot` on unmatched whitespace. A `Chunk`'s parent is a `ChunkedLevelPath`. `ChunkedLevelParent::Interior(_)` is `ChunkedLevelPath.parent`, a `ChunkedGroupPath`.

### Grammar tests, after

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_description_and_selection_set_on_following_lines_attach() {
        let text = "\n  field Query.HomeRoute @component\n  \"\"\"\n  Show a list of pets\n  \"\"\"\n  {\n    pets {\n      id\n    }\n  }\n";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "HomeRoute"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::String, "\"\"\"\n  Show a list of pets\n  \"\"\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "pets"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let field = as_selectable(parse.reference());
        assert!(field.description.is_some());
        assert!(field.selection_set.is_some());
    }

    fn to_on_the_next_line_does_not_attach() {
        let text = "field Query.Foo\nto User { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Content, "to"),
                (IsographSemanticToken::Content, "User"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(as_selectable(parse.reference()).target_type, None);
        assert!(first_slot(parse.reference()).extra.is_some());
        assert!(errors.iter().any(|error| {
            error.item == expected(EndOfDeclaration, Found::Token(Identifier))
                && error.location == span_of(text, "to")
        }));
    }

    fn a_directive_on_the_next_line_does_not_attach() {
        let text = "field Query.Foo\n@loadable { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Content, "@"),
                (IsographSemanticToken::Content, "loadable"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(as_selectable(parse.reference()).directive_set, None);
        assert!(first_slot(parse.reference()).extra.is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(At))
                .with_span(span_of(text, "@"))
                .wrap_vec(),
        );
    }

    fn variables_on_the_next_line_do_not_attach() {
        let text = "field Query.Foo\n($id: ID) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Bracket, "("),
                (IsographSemanticToken::Content, "$"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Content, ":"),
                (IsographSemanticToken::Content, "ID"),
                (IsographSemanticToken::Bracket, ")"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(as_selectable(parse.reference()).variable_definitions, None);
        assert!(first_slot(parse.reference()).extra.is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Group(BracketKind::Parenthesis))
                .with_span(span_of(text, "($id: ID)"))
                .wrap_vec(),
        );
    }

    fn a_second_declaration_on_the_next_line_is_leftover() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "field"),
                (IsographSemanticToken::Content, "User"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "name"),
            ],
        );
        as_entrypoint(parse.reference());
        assert!(first_slot(parse.reference()).extra.is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "field"))
                .wrap_vec(),
        );
    }

    fn a_comma_then_a_second_declaration_is_leftover_at_the_comma() {
        let text = "entrypoint Query.foo, field User.name";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, ","),
                (IsographSemanticToken::Content, "field"),
                (IsographSemanticToken::Content, "User"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "name"),
            ],
        );
        as_entrypoint(parse.reference());
        assert_eq!(
            first_slot(parse.reference())
                .extra
                .as_ref()
                .expect("comma and the second declaration")
                .location,
            span_of(text, ", field User.name"),
        );
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    fn a_comma_before_entrypoint_is_not_skipped() {
        let text = ",entrypoint Query.foo";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Content, ","),
                (IsographSemanticToken::Content, "entrypoint"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "foo"),
            ],
        );
        assert!(parsed_item(parse.reference()).is_none());
        assert_eq!(
            errors,
            expected(DECLARATION_KEYWORD, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    fn a_lone_comma_is_a_failed_declaration() {
        let text = ",";
        let (parse, errors) = parsed(
            text,
            &[(IsographSemanticToken::Content, ",")],
        );
        assert!(parsed_item(parse.reference()).is_none());
        assert_eq!(
            errors,
            expected(DECLARATION_KEYWORD, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    fn a_comma_between_the_name_and_the_selection_set_does_not_attach() {
        let text = "field Query.Foo,\n{ bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Content, ","),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    fn consume_line_breaks_does_not_record_them() {
        let text = "\nentrypoint Query.foo\n";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
            ],
        );
        assert_eq!(errors, vec![]);
        as_entrypoint(parse.reference());
    }
```

`a_selection_set_on_its_own_line_is_a_second_declaration` is `a_description_and_selection_set_on_following_lines_attach`. `a_second_contentful_chunk_is_multiple_declarations` is `a_second_declaration_on_the_next_line_is_leftover`. `entrypoint\nQuery.foo` fails at the line break: `parse_type_dot_name` does not call `consume_line_breaks`. `a_comma_before_a_second_declaration_is_the_boundary_comma` is `a_comma_then_a_second_declaration_is_leftover_at_the_comma`. `comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses` is `a_comma_before_entrypoint_is_not_skipped`. `a_lone_comma_is_chunkings_error_and_an_empty_literal` is `a_lone_comma_is_a_failed_declaration`.

`the_unrecognized_keyword_resolves_as_a_token_in_the_failed_chunk`: `IsoLiteralSlot`.

`a_final_comma_after_the_selectable_declaration_is_an_error` stays: leftover comma, `Expected(EndOfDeclaration, Comma)`. The comma is content, recorded as leftover `Content`.

`empty_and_whitespace_only_literals_are_empty_literal_errors` still covers `""`, `"   "`, `"\n\n"`.

Interior grammar tests that pin line-break splits stay: `a_directive_on_the_next_line_is_its_own_failed_selection`, `a_line_break_inside_a_list_type_does_not_attach_bang`, a selection `bar\n{ baz }`.

### Standards and follow-ups

parsing-standards.md:

- `parse_iso_literal` takes `WithSpan<ChunkedRoot>`.
- `IsoLiteralParse = Slot<IsoLiteralItem, UnparsedChunkItems>`.
- Extra leftover is `Slot.extra`. There are no extra chunks on the tree.
- A level inside brackets is a `ChunkedLevel`. A level that is not is `ChunkedRoot`: `Vec<WithSpan<ChunkContentItem>>`.
- `ItemCursor::consume_line_breaks` is `consume_*` of a run of `LineBreak` tokens (`advance`, no semantic token). Zero is legal. Call sites: `parse_iso_literal_item` before the keyword; `parse_selectable_declaration` before the description, before the selection set, and after the selection set; `parse_entrypoint` after directives. `peek` / `consume_token_if` / `consume_group_if` do not eat line breaks. A comma is not a line break.
- `ChunkStream::new` takes a slice. `parse_stream` spans then leftover. `parse_one_chunk` is `parse_stream` plus trailing-separator fold for a real chunk. `parse_chunked_iso_literal` streams the unpartitioned vec.
- `parse_singleton` returns `WithSpan<Slot<T, UnparsedChunkItems>>`, one-item `ChunkedLevel` interiors only, extra chunks are `Expected(end, found)` plus leftover recording.
- Diagnostic: `report_error` in `parse_one_chunk`; `errors.push` in `parse_singleton` (boundary comma, extra type chunks) and `parse_iso_literal` (`EmptyLiteral`).
- `ChunkedLevelParent` / `ExtraChunks` / `Singleton` / `MultipleDeclarations` listings deleted. `ChunkedRoot` listed.

future-improvements.md, "Line break and comma are the same chunk separator": drop the `field Query.Foo\n{ bar }` bullet. Keep the interior bullets (`bar\n{ baz }`, `bar\n@loadable`, `[Pet\n!]`).

slot-stages.md `require_complete_literal` is `require_complete(parse)`.

parser-minor-improvements.md `parse_singleton assumes a non-empty level`: still true, still `[...]` and the `len() == 0` check in `parse_bracket_interior_type`. The root empty check is `split_first` on the root vec, or `consume_line_breaks` then `require_end` for only line breaks.

parsing-notes.md: delete the `parse_iso_literal` / `parse_singleton` nonempty note.
