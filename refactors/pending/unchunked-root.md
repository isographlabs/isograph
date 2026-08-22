# Unchunked root

An iso literal is one declaration. The user writes it across lines. The parser consumes the whole root as one form.

```
field Query.Foo
{
  bar
}
```

A selectable `Query.Foo` whose selection set is `{ bar }`.

```
field Query.Foo
  to User
  @loadable
  ($id: ID)
  "home page"
{
  bar
}
```

`to`, the directive, the variable list, the description, and the selection set attach. `entrypoint` then `Query.foo` on the next line is one entrypoint.

```
entrypoint Query.foo
field User.name
```

The entrypoint parses. `field User.name` is leftover. `Expected(EndOfDeclaration, Identifier)` at `field`.

```
field Query.Foo,
```

The selectable parses. The comma is leftover. `Expected(EndOfDeclaration, Comma)` at the comma.

`""`, `"   "`, `"\n\n"` are `EmptyLiteral` and `None`. `","` is a failed declaration: `Expected(DECLARATION_KEYWORD, Comma)` at the comma, `item: None`, extra is the comma.

`,entrypoint Query.foo` is the same shape at the leading comma: the comma is the first item, the form fails there, extra is the whole contents.

A list interior still partitions on commas and line breaks. `{ bar\nbaz }` is two selections. `{ foo\n{ bar } }` is a scalar plus a failed selection. `[Pet\n!]` does not attach the bang.

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
    pub Option<WithSpan<Chunk>>,
);

pub type ChunkedRootPath<'a> = PositionResolutionPath<&'a ChunkedRoot, ()>;
```

`None` is an empty literal. `Some` is the one root chunk: every non-line-break root item, `trailing_separator: None`. Groups in that chunk have `ChunkedLevel` interiors, partitioned as they are today.

```rust
// from crates/isograph_parser/src/chunk.rs
#[derive(Debug)]
pub enum ChunkParent<'a> {
    Root(ChunkedRootPath<'a>),
    Level(ChunkedLevelPath<'a>),
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

Delta from the landed enum: drop `Singleton` and `ExtraChunks`; add `ChunkedRoot`; `IsoLiteralSlot`'s payload is `IsoLiteralParsePath` (the slot, parent `()`). `ChunkedLevelPath`'s parent is `ChunkedGroupPath`. Every other variant is unchanged.

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
    let mut contents = Vec::new();
    for item in tree.item.0.iter() {
        if let Some(content) = root_content(item, &mut errors) {
            contents.push(content.with_span(item.location));
        }
    }
    let mut items = contents.into_iter();
    let chunk = items.next().map(|head| {
        let contents = NonEmpty {
            head,
            tail: items.collect(),
        };
        let location = Span::join(contents.first().location, contents.last().location);
        Chunk {
            contents,
            trailing_separator: None,
        }
        .with_span(location)
    });
    (ChunkedRoot(chunk).with_span(tree.location), errors)
}

fn root_content(
    item: &WithSpan<BracketItem>,
    errors: &mut Vec<CommaWithoutItem>,
) -> Option<ChunkContentItem> {
    match item.item.reference() {
        BracketItem::Raw(token) if token.0 == NonBracketTokenKind::LineBreak => None,
        BracketItem::Raw(token) => ChunkContentItem::NonBracket(*token).wrap_some(),
        BracketItem::Bracketed(group) => {
            ChunkContentItem::Group(chunk_group(group, errors)).wrap_some()
        }
    }
}
```

`chunk_level`, `absorb_chunk`, `as_content`, `chunk_group` are unchanged. `CommaWithoutItem` is produced only inside groups.

Root line breaks are dropped. A position on a dropped root line break answers `ChunkedRoot` on the chunk tree, and `IsoLiteralSlot` on the grammar tree (unmatched on the whole-literal span). Root commas are content.

## Parse

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub(crate) fn parse_chunked_iso_literal(
    text: &str,
    root: WithSpan<ChunkedRoot>,
    errors: &mut Vec<WithSpan<AstError>>,
    tokens: &mut Vec<WithSpan<IsographSemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    let Some(chunk) = root.item.0.as_ref() else {
        errors.push(AstError::EmptyLiteral.with_span(location));
        return None;
    };
    let slot = parse_one_chunk(
        chunk,
        chunk.item.stream(text, tokens, errors),
        Expectation::EndOfDeclaration,
        parse_iso_literal_item,
    );
    slot.with_span(location).wrap_some()
}
```

`parse_one_chunk` is `pub(crate)`. The returned `WithSpan` is the whole literal, so a position on leading whitespace or on a consumed keyword of a failed form (`fieldd` in `fieldd Query.foo { bar }`) is unmatched on the slot and answers `IsoLiteralSlot`. `item` and `extra` keep the tight spans `parse_one_chunk` assigned.

Leftover after a complete declaration is `Slot.extra` plus `report_error(Expected(EndOfDeclaration, found))` at the first leftover item, including a trailing comma and a second `field` / `entrypoint` on the next line.

```rust
// from crates/isograph_parser/src/lib.rs
pub(crate) use chunk::{chunk, parse_one_chunk, parse_singleton};
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

The existing interior comma tests (`foo {,}`, `{, a }`, `{ a, }`, `a_second_comma_after_line_breaks` rewritten onto `{ a,\n\n,b }` if that fixture is not already interior) stay. If a root comma-without-item test has no interior twin, add one: `{, a }`, `{ a,,\nb }`, `{ a, , }`, `{,,a }`.

## 2. Root is not partitioned

One commit. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.

### `chunk`

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub(crate) fn chunk(
    tree: &WithSpan<MatchedBrackets>,
) -> (WithSpan<ChunkedLevel>, Vec<CommaWithoutItem>) {
    let mut errors = Vec::new();
    let level = chunk_level(tree.item.reference(), &mut errors);
    (level.with_span(tree.location), errors)
}
```

After: the `chunk` / `root_content` / `ChunkedRoot` listings above. `chunk_level` is only `chunk_group`.

### Types

`ChunkedRoot`, `ChunkedRootPath`, `ChunkParent`, `ChunkedLevel` parent, `Slot` pins, `UnparsedChunkItemsParent`, `IsoLiteralParse`, `IsographResolutionNode` as above.

Delete:

```rust
// from crates/isograph_parser/src/chunk.rs
pub struct Singleton<T, E> {
    pub item: WithSpan<T>,
    pub extra_chunks: Option<WithSpan<E>>,
}

pub struct ExtraChunks(
    pub NonEmpty<WithSpan<Chunk>>,
);

pub enum ChunkedLevelParent<'a> {
    Root,
    Interior(Box<ChunkedGroupPath<'a>>),
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("Expected nothing after the declaration. Each literal holds exactly one declaration.")]
    MultipleDeclarations,
```

The `ast_error_unit_variants_use_their_messages` arm for `MultipleDeclarations` goes with it.

`lib.rs` `pub use` drops `ExtraChunks`, `Singleton`, `ExtraChunksPath`, `ChunkedLevelParent`. It adds `ChunkedRoot`, `ChunkedRootPath`. `IsoLiteralSlotPath` is deleted; callers use `IsoLiteralParsePath`.

### Parse

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    if root.item.len() == 0 {
        errors.push(AstError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        errors,
        Expectation::EndOfDeclaration,
        |extra| AstError::MultipleDeclarations.with_span(extra.location),
        parse_iso_literal_item,
    );
    singleton.with_span(location).wrap_some()
```

After: `parse_chunked_iso_literal` as above. `parse_singleton` / `parse_nested_singleton` / `parse_bracket_interior_type` as above.

Helpers in `parse_iso_literal.rs` tests:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn first_slot(parse: &WithSpan<IsoLiteralParse>) -> &Slot<IsoLiteralItem, UnparsedChunkItems> {
        parse.item.reference()
    }
```

`chunked` in both test modules returns `WithSpan<ChunkedRoot>`. `stream_of` streams `tree.item.0.as_ref().expect(...).item`.

### Chunk tests, after

`whitespace_only_and_empty_literals_are_empty_levels`: `tree.item.0` is `None`. Span is still the whole literal.

`a_line_break_before_a_group_splits_the_field_from_its_selection_set` becomes one root chunk whose contents are the identifier and the group:

```rust
// from crates/isograph_parser/src/chunk.rs
    fn a_line_break_before_a_group_keeps_the_group_in_the_root_chunk() {
        let text = "foo\n{ bar }";
        let tree = chunked(text);
        let root = tree.item.0.as_ref().expect("foo and the group");
        assert_eq!(root.item.contents.len(), 2);
        assert!(root.item.trailing_separator.is_none());
        as_non_bracket(content_item(root.item.reference(), 0));
        as_group(content_item(root.item.reference(), 1));
    }
```

`commas_and_line_breaks_are_equivalent_separators` at the root: `"a, b"` is one chunk, comma is content; `"a\nb"` is one chunk, two identifiers, no comma. Interior equivalence is change 1.

`captured_line_breaks_make_no_chunk` at the root: `"\n\na, b\n"` is one chunk `a`, comma, `b`. Interior `"foo {\n bar\n}"` is unchanged.

Root comma-without-item tests (`"\n, a"`, `"a,\n\n,b"`, `"a,,\nb"`, `",,a"`, `"a, ,"`): no `CommaWithoutItem`. The commas are content of the one root chunk. Interior twins from change 1 keep the errors.

`contents_span_stops_at_the_last_content_item` for `"foo,"`: contents include the comma, `contents_span` is `foo,`, `boundary_comma()` is `None`.

`a_chunk_without_a_comma_has_no_boundary_comma` for `"foo\nbar"`: one chunk, two identifiers.

`a_captured_line_break_resolves_to_its_level` at offset 0 of `"\nfoo {\n bar }"`: `ChunkedRoot`. The `{`'s captured interior newline is still `ChunkedLevel`.

`whitespace_only_resolves_to_the_root_level`: `ChunkedRoot`.

`a_dropped_close_and_the_text_after_it_resolve_to_the_root_level`: `ChunkedRoot`.

Resolve ancestry that matched `ChunkedLevelParent::Root` on the outer level matches `ChunkParent::Root`. `ChunkedLevelParent::Interior(_)` is `ChunkedLevelPath.parent`, a `ChunkedGroupPath`.

### Grammar tests, after

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn a_selection_set_on_the_next_line_attaches() {
        let text = "field Query.Foo\n{ bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(as_selectable(parse.reference()).selection_set.is_some());
        assert_eq!(
            selections(selection_set_of(as_selectable(parse.reference()))).len(),
            1,
        );
    }

    fn to_on_the_next_line_attaches() {
        let text = "field Query.Foo\nto User { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "User"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(as_selectable(parse.reference()).target_type.is_some());
    }

    fn a_directive_on_the_next_line_attaches() {
        let text = "field Query.Foo\n@loadable { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(as_selectable(parse.reference()).directive_set.is_some());
    }

    fn variables_on_the_next_line_attach() {
        let text = "field Query.Foo\n($id: ID) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(as_selectable(parse.reference()).variable_definitions.is_some());
    }

    fn a_description_on_the_next_line_attaches() {
        let text = "field Query.Foo\n\"home\" { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"home\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(as_selectable(parse.reference()).description.is_some());
    }

    fn type_dot_name_across_lines_parses() {
        let text = "field Query\n.\nFoo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        as_selectable(parse.reference());
    }

    fn entrypoint_name_on_the_next_line_parses() {
        let text = "entrypoint\nQuery.foo";
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

    fn a_leading_comma_fails_at_the_comma() {
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
        let parsed = parsed_with_errors(
            text,
            &[(IsographSemanticToken::Content, ",")],
        );
        let parse = parsed.item.expect("the comma is a chunk");
        assert!(parsed_item(parse.reference()).is_none());
        assert!(
            parsed
                .errors
                .iter()
                .any(|error| {
                    error.item
                        == ParseError::Ast(expected(DECLARATION_KEYWORD, Found::Token(Comma)))
                        && error.location == span_of(text, ",")
                })
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
```

`a_selection_set_on_its_own_line_is_a_second_declaration` is `a_selection_set_on_the_next_line_attaches`. `a_second_contentful_chunk_is_multiple_declarations` is `a_second_declaration_on_the_next_line_is_leftover`. `a_failed_first_chunk_is_reported_even_when_a_second_exists` (`entrypoint\nQuery.foo`) is `entrypoint_name_on_the_next_line_parses`. `a_comma_before_a_second_declaration_is_the_boundary_comma` is `a_comma_then_a_second_declaration_is_leftover_at_the_comma`. `comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses` is `a_leading_comma_fails_at_the_comma` (and `,,entrypoint Query.foo` fails at the first comma, extra is the whole contents). `a_lone_comma_is_chunkings_error_and_an_empty_literal` is `a_lone_comma_is_a_failed_declaration`.

`the_unrecognized_keyword_resolves_as_a_token_in_the_failed_chunk`: `IsoLiteralSlot`.

`a_final_comma_after_the_selectable_declaration_is_an_error` stays: leftover comma, `Expected(EndOfDeclaration, Comma)`. The comma is content, recorded as leftover `Content` the same way.

Interior grammar tests that pin line-break splits stay: `a_directive_on_the_next_line_is_its_own_failed_selection`, `a_line_break_inside_a_list_type_does_not_attach_bang`, a selection `bar\n{ baz }`.

### Standards and follow-ups

parsing-standards.md:

- `parse_iso_literal` takes `WithSpan<ChunkedRoot>`.
- `IsoLiteralParse = Slot<IsoLiteralItem, UnparsedChunkItems>`.
- Extra leftover is `Slot.extra`. There are no extra chunks on the tree.
- `parse_singleton` returns `WithSpan<Slot<T, UnparsedChunkItems>>`, one-item `ChunkedLevel` interiors only, extra chunks are `Expected(end, found)` plus leftover recording.
- Diagnostic: `report_error` in `parse_one_chunk`; `errors.push` in `parse_singleton` (boundary comma, extra type chunks) and `parse_iso_literal` (`EmptyLiteral`).
- `ChunkedLevelParent` / `ExtraChunks` / `Singleton` / `MultipleDeclarations` listings deleted. `ChunkedRoot` listed.

future-improvements.md, "Line break and comma are the same chunk separator": drop the `field Query.Foo\n{ bar }` bullet. Keep the interior bullets (`bar\n{ baz }`, `bar\n@loadable`, `[Pet\n!]`).

slot-stages.md `require_complete_literal` is `require_complete(parse)`.

parser-minor-improvements.md `parse_singleton assumes a non-empty level`: still true, still `[...]` and the `len() == 0` check in `parse_bracket_interior_type`. The root empty check is `ChunkedRoot(None)`.

parsing-notes.md: delete the `parse_iso_literal` / `parse_singleton` nonempty note.
