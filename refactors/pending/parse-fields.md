# parse-fields: field declarations and selection sets

Second doc of the series parsing-plan.md orders, after parse-entrypoint.md. It lands `field Type.name { ... }` declarations, selection sets with scalar and object selections and aliases, per-item degradation via `UnparsedItem`, and the parent-enum conversions that second parents force. Arguments are not parsed until parse-arguments.md: a paren group inside a selection is that selection's unparsed reason.

## The grammar this doc accepts

The declaration chunk:

```
field <Identifier> . <Identifier> <brace group>
```

The brace group is required and is the last item of the chunk. Each contentful chunk of its interior level is one selection:

```
[<Identifier> :] <Identifier> [<brace group>]
```

The leading identifier is the alias when a colon follows, the name otherwise. A selection with a brace group is an object selection whose interior recurses; without one it is a scalar selection. A selection chunk that fails to parse becomes `Selection::Unparsed`, holding the reason and its chunk; sibling selections and the declaration parse normally. Declaration-header failures still degrade the whole literal, as in parse-entrypoint.md.

## Changes to chunk.rs

Three changes, all listed here because parse code and resolution depend on them.

1. The chunk tree becomes cloneable, so an unparsed item can own the chunk it covers while the rest of the tree is dropped. `Clone` joins the derive lists of `ChunkedLevel`, `Chunk`, `ChunkContentItem`, `ChunkedGroup`, and `ChunkSeparator` (their contents are already `Copy`).

2. `Chunk`'s parent becomes an enum: a chunk sits in a level, or it is the payload of an unparsed item. Before:

   ```rust
   // from crates/isograph_parser/src/chunk.rs
   pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkedLevelPath<'a>>;
   ```

   After:

   ```rust
   // from crates/isograph_parser/src/chunk.rs
   #[derive(Debug)]
   pub enum ChunkParent<'a> {
       Level(ChunkedLevelPath<'a>),
       UnparsedItem(UnparsedItemPath<'a>),
   }

   pub type ChunkPath<'a> = PositionResolutionPath<&'a Chunk, ChunkParent<'a>>;
   ```

   No box: `UnparsedItemPath` reaches `ChunkPath` only through `SelectionSetParent::Object`, which is already boxed.

3. `ChunkedLevel`'s field wraps the parent in the new variant. Before:

   ```rust
   // from crates/isograph_parser/src/chunk.rs
   pub struct ChunkedLevel(#[resolve_field] pub Vec<WithSpan<Chunk>>);
   ```

   After:

   ```rust
   // from crates/isograph_parser/src/chunk.rs
   pub struct ChunkedLevel(#[resolve_field(parent_variant = Level)] pub Vec<WithSpan<Chunk>>);
   ```

Every other derive site names `ChunkPath` through the alias and is untouched. The chunk.rs and parse_iso_literal.rs tests that walked `chunk_path.parent.parent` now pattern through `ChunkParent::Level`; the test modules gain one helper and the affected matches respell:

```rust
// from crates/isograph_parser/src/chunk.rs (test module; parse_iso_literal.rs gets the same helper)
    fn level_of<'a>(parent: &'a ChunkParent<'a>) -> &'a ChunkedLevelPath<'a> {
        match parent {
            ChunkParent::Level(level) => level,
            parent => panic!("expected a level parent, got {parent:?}"),
        }
    }
```

The affected tests are `an_unmatched_open_resolves_with_the_host_chunk_as_parent`, `an_unmatched_close_at_the_root_resolves_with_the_root_level`, `an_ordinary_token_resolves_to_its_own_leaf`, and `resolution_walks_ancestry_against_source_text` in chunk.rs, and the two `..._resolve_through_the_chunk_tree` / `..._resolves_under_the_unparsed_literal` tests in parse_iso_literal.rs, whose `token.parent.parent.parent` becomes `level_of(&token.parent.parent).parent`.

## Changes to parse_error.rs

`Expectation` gains three variants and their `Display` arms:

```rust
// from crates/isograph_parser/src/parse_error.rs
pub enum Expectation {
    Token(NonBracketTokenKind),
    DeclarationKeyword,
    EndOfDeclaration,
    /// A `{ ... }` group holding selections.
    SelectionSet,
    /// One selection: `name`, `alias: name`, with or without a nested selection set.
    Selection,
    /// The item is complete; only a comma or line break may follow.
    Separator,
}
```

```rust
// from crates/isograph_parser/src/parse_error.rs
            Expectation::SelectionSet => write!(f, "a selection set, like '{{ id, name }}'"),
            Expectation::Selection => write!(f, "a field selection"),
            Expectation::Separator => write!(f, "a comma or line break"),
```

## Changes to parse_iso_literal.rs

`IsoLiteralParse` gains the field variant; dispatch stops treating `field` as unsupported. Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralParse {
    Entrypoint(EntrypointDeclaration),
    Unparsed(UnparsedLiteral),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, &mut items)?)),
        "field" | "pointer" => Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword)),
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralParse {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
    Unparsed(UnparsedLiteral),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => Ok(IsoLiteralParse::Entrypoint(parse_entrypoint(keyword, &mut items)?)),
        "field" => Ok(IsoLiteralParse::Field(parse_field(keyword, &mut items)?)),
        "pointer" => Err(WithSpan::new(ParseError::UnsupportedDeclarationType, keyword)),
```

The parse-entrypoint.md test `field_and_pointer_declarations_do_not_parse_yet` narrows to its pointer case (parse-pointers.md deletes it entirely).

The declaration type and its parse function live here beside the entrypoint's:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = (), resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldDeclaration {
    pub field_keyword: WithSpan<FieldKeyword>,
    #[resolve_field(parent_variant = Field)]
    pub parent_type: WithSpan<EntityName>,
    pub dot: WithSpan<Dot>,
    #[resolve_field(parent_variant = Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}

/// The `field` keyword. Positions on it answer the declaration.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct FieldKeyword;

pub type ClientFieldDeclarationPath<'a> = PositionResolutionPath<&'a ClientFieldDeclaration, ()>;
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_field(
    keyword: Span,
    items: &mut ChunkContents<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let parent_type = expect_token(
        items,
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
        keyword.end,
    )?;
    let dot = expect_token(
        items,
        NonBracketTokenKind::Period,
        Expectation::Token(NonBracketTokenKind::Period),
        parent_type.end,
    )?;
    let client_field_name = expect_token(
        items,
        NonBracketTokenKind::Identifier,
        Expectation::Token(NonBracketTokenKind::Identifier),
        dot.end,
    )?;
    let selection_set = expect_selection_set(items, client_field_name.end)?;
    expect_chunk_end(items, Expectation::EndOfDeclaration)?;
    Ok(ClientFieldDeclaration {
        field_keyword: WithSpan::new(FieldKeyword, keyword),
        parent_type: WithSpan::new(EntityName, parent_type),
        dot: WithSpan::new(Dot, dot),
        client_field_name: WithSpan::new(ClientFieldName, client_field_name),
        selection_set,
    })
}
```

`expect_token`, `expect_chunk_end`, `token_text`, and the `ChunkContents` alias become `pub(crate)` so selections.rs can use them; `errors()` gains the field arm (listed with the error walk below).

The two names' parents become enums, since a name now sits under an entrypoint or a field declaration. Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;
```

After (and identically for `ClientFieldName` with `ClientFieldNameParent`):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = EntityNameParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName;

#[derive(Debug)]
pub enum EntityNameParent<'a> {
    Entrypoint(EntrypointDeclarationPath<'a>),
    Field(ClientFieldDeclarationPath<'a>),
}

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntityNameParent<'a>>;
```

`EntrypointDeclaration`'s two marked fields respell from bare `#[resolve_field]` to `#[resolve_field(parent_variant = Entrypoint)]`.

## New module: selections.rs

```rust
// from crates/isograph_parser/src/selections.rs
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use safe_peekable::IntoSafePeekable;
use span::{Span, WithSpan};

use crate::{
    empty_chunk_comma_span, expect_chunk_end, expect_token, BracketKind, Chunk,
    ChunkContentItem, ChunkContents, ChunkedLevel, ClientFieldDeclarationPath, Expectation,
    ExpectedFound, Found, IsographResolutionNode, NonBracketTokenKind, ParseError,
};

/// The selections a `{ ... }` group holds, one per contentful chunk of its interior.
/// The wrapping `WithSpan`'s span covers the braces.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<Selection>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
    Unparsed(#[resolve_field(parent_variant = SelectionSet)] UnparsedItem),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ScalarSelection {
    #[resolve_field(parent_variant = Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Scalar)]
    pub name: WithSpan<SelectionName>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectSelection {
    #[resolve_field(parent_variant = Object)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field(parent_variant = Object)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field(parent_variant = Object)]
    pub selection_set: WithSpan<SelectionSet>,
}

/// A chunk that failed to parse as its level's item: the reason, and the chunk itself
/// for positions to resolve against.
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnparsedItemParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnparsedItem {
    pub reason: WithSpan<ParseError>,
    #[resolve_field(parent_variant = UnparsedItem)]
    pub chunk: WithSpan<Chunk>,
}

/// The name a selection selects. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionNameParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionName;

/// The alias before the colon in `alias: name`. Its text is its span.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionAliasParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionAlias;

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    Field(ClientFieldDeclarationPath<'a>),
    Object(Box<ObjectSelectionPath<'a>>),
    // parse-pointers.md adds Pointer
}

#[derive(Debug)]
pub enum UnparsedItemParent<'a> {
    SelectionSet(SelectionSetPath<'a>),
    // parse-arguments.md adds ArgumentList and ObjectLiteral,
    // parse-variables.md adds VariableDeclarationList
}

#[derive(Debug)]
pub enum SelectionNameParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

#[derive(Debug)]
pub enum SelectionAliasParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

pub type SelectionSetPath<'a> = PositionResolutionPath<&'a SelectionSet, SelectionSetParent<'a>>;

pub type ScalarSelectionPath<'a> = PositionResolutionPath<&'a ScalarSelection, SelectionSetPath<'a>>;

pub type ObjectSelectionPath<'a> = PositionResolutionPath<&'a ObjectSelection, SelectionSetPath<'a>>;

pub type UnparsedItemPath<'a> = PositionResolutionPath<&'a UnparsedItem, UnparsedItemParent<'a>>;

pub type SelectionNamePath<'a> = PositionResolutionPath<&'a SelectionName, SelectionNameParent<'a>>;

pub type SelectionAliasPath<'a> = PositionResolutionPath<&'a SelectionAlias, SelectionAliasParent<'a>>;
```

The `SelectionSetParent::Object` payload is boxed to break the cycle `SelectionSetPath -> ObjectSelectionPath -> SelectionSetPath`, exactly as `ChunkedLevelParent::Interior` does; the derive's `parent.into()` converts through the std `From<T> for Box<T>`.

The parse functions:

```rust
// from crates/isograph_parser/src/selections.rs
/// A required selection set as the next item; the error otherwise.
pub(crate) fn expect_selection_set(
    items: &mut ChunkContents<'_>,
    missing_at: u32,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    if let Some(selection_set) = consume_selection_set(items) {
        return Ok(selection_set);
    }
    match items.peek() {
        Some(peek) => {
            let item = peek.view();
            Err(WithSpan::new(
                ParseError::expected(Expectation::SelectionSet, Found::from(&item.item)),
                item.location,
            ))
        }
        None => Err(WithSpan::new(
            ParseError::expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(missing_at, missing_at),
        )),
    }
}

/// The next item, consumed, when it is a brace group; its interior parses into
/// selections, infallibly, each failed chunk degrading to `Selection::Unparsed`.
fn consume_selection_set(items: &mut ChunkContents<'_>) -> Option<WithSpan<SelectionSet>> {
    let peek = items.peek()?;
    let item = peek.view();
    match &item.item {
        ChunkContentItem::Group(group) if group.opening.item.0 == BracketKind::Brace => {
            let selection_set = SelectionSet(parse_level_items(
                &group.children.item,
                Expectation::Selection,
                parse_selection,
                Selection::Unparsed,
            ));
            let span = item.location;
            peek.commit();
            Some(WithSpan::new(selection_set, span))
        }
        _ => None,
    }
}

fn parse_selection(chunk: &WithSpan<Chunk>) -> Result<Selection, WithSpan<ParseError>> {
    let mut items = chunk.item.contents.iter().safe_peekable();
    let first = expect_token(
        &mut items,
        NonBracketTokenKind::Identifier,
        Expectation::Selection,
        chunk.location.start,
    )?;
    // `first` is the alias when a colon follows, the name otherwise.
    let (reader_alias, name) = match consume_token_if(&mut items, NonBracketTokenKind::Colon) {
        Some(colon) => {
            let name = expect_token(
                &mut items,
                NonBracketTokenKind::Identifier,
                Expectation::Token(NonBracketTokenKind::Identifier),
                colon.end,
            )?;
            (
                Some(WithSpan::new(SelectionAlias, first)),
                WithSpan::new(SelectionName, name),
            )
        }
        None => (None, WithSpan::new(SelectionName, first)),
    };
    let selection_set = consume_selection_set(&mut items);
    expect_chunk_end(&mut items, Expectation::Separator)?;
    Ok(match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection { reader_alias, name }),
    })
}

/// The next item's span, consumed, when it is a non-bracket token of `kind`; `None`,
/// nothing consumed, otherwise.
pub(crate) fn consume_token_if(
    items: &mut ChunkContents<'_>,
    kind: NonBracketTokenKind,
) -> Option<Span> {
    let peek = items.peek()?;
    let item = peek.view();
    match &item.item {
        ChunkContentItem::NonBracket(token) if token.0 == kind => {
            let span = item.location;
            peek.commit();
            Some(span)
        }
        _ => None,
    }
}

/// Every contentful chunk of a level parses to one item via `parse_item`; a chunk that
/// fails becomes `unparsed` holding the reason and a clone of the chunk. Every empty
/// chunk is a comma no item precedes (refactors/past/one-comma-per-boundary.md) and becomes an
/// unparsed item at that comma; line breaks at a level's start are captured by the
/// opening bracket and never reach this walk. A parsed item's span covers the chunk's
/// contents, without its boundary.
pub(crate) fn parse_level_items<T>(
    level: &ChunkedLevel,
    item_expectation: Expectation,
    parse_item: impl Fn(&WithSpan<Chunk>) -> Result<T, WithSpan<ParseError>>,
    unparsed: impl Fn(UnparsedItem) -> T,
) -> Vec<WithSpan<T>> {
    let mut parsed = Vec::new();
    for chunk in level.0.iter() {
        if chunk.item.contents.is_empty() {
            let reason = WithSpan::new(
                ParseError::expected(item_expectation, Found::Token(NonBracketTokenKind::Comma)),
                empty_chunk_comma_span(chunk),
            );
            parsed.push(WithSpan::new(
                unparsed(UnparsedItem { reason, chunk: chunk.clone() }),
                chunk.location,
            ));
            continue;
        }
        let span = chunk
            .item
            .contents
            .iter()
            .map(|item| item.location)
            .reduce(Span::join)
            .unwrap_or(chunk.location);
        let item = match parse_item(chunk) {
            Ok(item) => item,
            Err(reason) => unparsed(UnparsedItem { reason, chunk: chunk.clone() }),
        };
        parsed.push(WithSpan::new(item, span));
    }
    parsed
}
```

## The errors

`errors()` walks the field declaration's tree; nested selection sets recurse. In source order, because selections are stored in source order.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
impl IsoLiteralParse {
    /// Every error the pass produced, in source order.
    pub fn errors(&self) -> Vec<WithSpan<ParseError>> {
        match self {
            IsoLiteralParse::Entrypoint(_) => vec![],
            IsoLiteralParse::Field(declaration) => {
                let mut errors = Vec::new();
                collect_selection_set_errors(&declaration.selection_set.item, &mut errors);
                errors
            }
            IsoLiteralParse::Unparsed(unparsed) => vec![unparsed.reason],
        }
    }
}
```

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn collect_selection_set_errors(
    selection_set: &SelectionSet,
    errors: &mut Vec<WithSpan<ParseError>>,
) {
    for selection in &selection_set.0 {
        match &selection.item {
            Selection::Scalar(_) => {}
            Selection::Object(object) => {
                collect_selection_set_errors(&object.selection_set.item, errors)
            }
            Selection::Unparsed(unparsed) => errors.push(unparsed.reason),
        }
    }
}
```

## The resolution surface

`IsographResolutionNode` gains the new leaves:

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
    SelectionName(SelectionNamePath<'a>),
    SelectionAlias(SelectionAliasPath<'a>),
    UnparsedItem(UnparsedItemPath<'a>),
```

Positions on a chunk's trailing separator inside a parsed selection set answer the `SelectionSet` leaf: a parsed item's span covers only its chunk's contents, so the separator falls through to the container. The chunk-stage `ChunkSeparator` leaf remains reachable only inside unparsed regions.

## Generated code

The novel shapes, expanded. The enum delegates two variants and wraps the third's parent:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for Selection {
    type Parent<'a> = SelectionSetPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        match self {
            Selection::Scalar(inner) => inner.resolve(parent, position),
            Selection::Object(inner) => inner.resolve(parent, position),
            Selection::Unparsed(inner) => inner.resolve(
                <UnparsedItem as ::resolve_position::ResolvePosition>::Parent::SelectionSet(parent.into()),
                position,
            ),
        }
    }
}
```

The vec field iterates, each hit descending with the container's path:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for SelectionSet {
    type Parent<'a> = SelectionSetParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        for item in self.0.iter() {
            if item.location.contains(position) {
                let new_parent = self.path(parent);
                return item.item.resolve(new_parent, position);
            }
        }
        return Self::ResolvedNode::SelectionSet(self.path(parent).into());
    }
}
```

The unparsed item descends into its chunk through the chunk tree's new parent variant:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for UnparsedItem {
    type Parent<'a> = UnparsedItemParent<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        if self.chunk.location.contains(position) {
            let new_parent = <Chunk as ::resolve_position::ResolvePosition>::Parent::UnparsedItem(self.path(parent).into());
            return self.chunk.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::UnparsedItem(self.path(parent).into());
    }
}
```

The object selection's set field boxes on the way in:

```rust
// generated by resolve_position_macros/src/resolve_position_macro.rs
impl ::resolve_position::ResolvePosition for ObjectSelection {
    type Parent<'a> = SelectionSetPath<'a>;
    type ResolvedNode<'a> = IsographResolutionNode<'a>;

    fn resolve<'a>(&'a self, parent: Self::Parent<'a>, position: ::span::Span) -> Self::ResolvedNode<'a> {
        for item in self.reader_alias.iter() {
            if item.location.contains(position) {
                let new_parent = <SelectionAlias as ::resolve_position::ResolvePosition>::Parent::Object(self.path(parent).into());
                return item.item.resolve(new_parent, position);
            }
        }
        if self.name.location.contains(position) {
            let new_parent = <SelectionName as ::resolve_position::ResolvePosition>::Parent::Object(self.path(parent).into());
            return self.name.item.resolve(new_parent, position);
        }
        if self.selection_set.location.contains(position) {
            let new_parent = <SelectionSet as ::resolve_position::ResolvePosition>::Parent::Object(self.path(parent).into());
            return self.selection_set.item.resolve(new_parent, position);
        }
        return Self::ResolvedNode::ObjectSelection(self.path(parent).into());
    }
}
```

`ClientFieldDeclaration` and `ScalarSelection` expand like `EntrypointDeclaration` with `parent_variant` wrapping; `SelectionName` and `SelectionAlias` are fieldless leaves like `EntityName`. All follow parse-entrypoint.md's expansions with the names substituted.

## Tests

The parse_iso_literal.rs test module grows; helpers (`parsed`, `span_of`, `expected`, `assert_unparsed`, `level_of`) are shared.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs (test module)
    fn as_field(parse: &WithSpan<IsoLiteralParse>) -> &ClientFieldDeclaration {
        match &parse.item {
            IsoLiteralParse::Field(declaration) => declaration,
            parse => panic!("expected a field declaration, got {parse:?}"),
        }
    }

    fn selections(selection_set: &WithSpan<SelectionSet>) -> &[WithSpan<Selection>] {
        &selection_set.item.0
    }

    fn as_scalar(selection: &Selection) -> &ScalarSelection {
        match selection {
            Selection::Scalar(scalar) => scalar,
            selection => panic!("expected a scalar selection, got {selection:?}"),
        }
    }

    fn as_object(selection: &Selection) -> &ObjectSelection {
        match selection {
            Selection::Object(object) => object,
            selection => panic!("expected an object selection, got {selection:?}"),
        }
    }

    fn as_unparsed_item(selection: &Selection) -> &UnparsedItem {
        match selection {
            Selection::Unparsed(unparsed) => unparsed,
            selection => panic!("expected an unparsed item, got {selection:?}"),
        }
    }

    #[test]
    fn a_field_declaration_parses_with_scalar_selections() {
        let text = "field Query.Foo {\n  bar,\n  baz\n}";
        let parse = parsed(text);
        let declaration = as_field(&parse);
        assert_eq!(declaration.field_keyword.location, span_of(text, "field"));
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "Foo"));
        assert_eq!(declaration.selection_set.location, Span::new(span_of(text, "{").start, span_of(text, "}").end));
        let items = selections(&declaration.selection_set);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(&items[0].item).name.location, span_of(text, "bar"));
        assert_eq!(as_scalar(&items[1].item).name.location, span_of(text, "baz"));
        assert_eq!(items[0].location, span_of(text, "bar"));
        assert_eq!(parse.item.errors(), vec![]);
    }

    #[test]
    fn a_single_line_selection_set_parses_without_a_trailing_separator() {
        let text = "field Query.Foo { bar }";
        let parse = parsed(text);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(&items[0].item).name.location, span_of(text, "bar"));
        assert_eq!(parse.item.errors(), vec![]);
    }

    #[test]
    fn empty_selection_sets_hold_zero_selections() {
        for text in [
            "field Query.Foo {}",
            "field Query.Foo { }",
            "field Query.Foo {\n}",
        ] {
            let parse = parsed(text);
            assert_eq!(selections(&as_field(&parse).selection_set).len(), 0, "for literal {text:?}");
            assert_eq!(parse.item.errors(), vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_comma_before_the_first_selection_is_an_unparsed_item() {
        let text = "field Query.Foo {, bar }";
        let parse = parsed(text);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 2);
        let unparsed = as_unparsed_item(&items[0].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Selection, Found::Token(Comma))
        );
        assert_eq!(unparsed.reason.location, span_of(text, ","));
        assert_eq!(as_scalar(&items[1].item).name.location, span_of(text, "bar"));

        let lone = "field Query.Foo {,}";
        let parse = parsed(lone);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 1);
        let unparsed = as_unparsed_item(&items[0].item);
        assert_eq!(unparsed.reason.location, span_of(lone, ","));
        assert_eq!(parse.item.errors(), vec![unparsed.reason]);
    }

    #[test]
    fn an_alias_splits_from_the_name_at_the_colon() {
        let text = "field Query.Foo { b: bar }";
        let parse = parsed(text);
        let scalar = as_scalar(&selections(&as_field(&parse).selection_set)[0].item);
        let alias = scalar.reader_alias.as_ref().expect("the fixture selects with an alias");
        let alias_anchor = span_of(text, "b:");
        assert_eq!(alias.location, Span::new(alias_anchor.start, alias_anchor.start + 1));
        assert_eq!(scalar.name.location, span_of(text, "bar"));
    }

    #[test]
    fn object_selections_nest() {
        let text = "field Query.Foo { pet { name, age } }";
        let parse = parsed(text);
        let object = as_object(&selections(&as_field(&parse).selection_set)[0].item);
        assert_eq!(object.name.location, span_of(text, "pet"));
        let inner = selections(&object.selection_set);
        assert_eq!(inner.len(), 2);
        assert_eq!(as_scalar(&inner[0].item).name.location, span_of(text, "name"));
        assert_eq!(as_scalar(&inner[1].item).name.location, span_of(text, "age"));
        assert_eq!(parse.item.errors(), vec![]);
    }

    #[test]
    fn an_orphaned_group_after_a_line_break_is_an_unparsed_selection() {
        let text = "field Query.Foo {\n  bar\n  { baz }\n}";
        let parse = parsed(text);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(&items[0].item).name.location, span_of(text, "bar"));
        let unparsed = as_unparsed_item(&items[1].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Selection, Found::Group(BracketKind::Brace))
        );
        assert_eq!(unparsed.reason.location, span_of(text, "{ baz }"));
        assert_eq!(parse.item.errors(), vec![unparsed.reason]);
    }

    #[test]
    fn a_doubled_comma_between_selections_is_an_unparsed_item() {
        let text = "field Query.Foo { a,, b }";
        let parse = parsed(text);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 3);
        assert_eq!(as_scalar(&items[0].item).name.location, span_of(text, "a"));
        assert_eq!(as_scalar(&items[2].item).name.location, span_of(text, "b"));
        let unparsed = as_unparsed_item(&items[1].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Selection, Found::Token(Comma))
        );
        let commas = span_of(text, ",,");
        assert_eq!(unparsed.reason.location, Span::new(commas.start + 1, commas.end));
    }

    #[test]
    fn two_selections_in_one_chunk_degrade_that_chunk_alone() {
        let text = "field Query.Foo {\n  bar baz\n  qux\n}";
        let parse = parsed(text);
        let items = selections(&as_field(&parse).selection_set);
        assert_eq!(items.len(), 2);
        let unparsed = as_unparsed_item(&items[0].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Separator, Found::Token(Identifier))
        );
        assert_eq!(unparsed.reason.location, span_of(text, "baz"));
        assert_eq!(as_scalar(&items[1].item).name.location, span_of(text, "qux"));
        assert_eq!(parse.item.errors(), vec![unparsed.reason]);
    }

    #[test]
    fn arguments_do_not_parse_yet() {
        let text = "field Query.Foo { bar(x: 1) }";
        let parse = parsed(text);
        let unparsed = as_unparsed_item(&selections(&as_field(&parse).selection_set)[0].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Separator, Found::Group(BracketKind::Parenthesis))
        );
        assert_eq!(unparsed.reason.location, span_of(text, "(x: 1)"));
    }

    #[test]
    fn a_directive_on_a_selection_is_an_ordinary_unexpected_token() {
        let text = "field Query.Foo { bar @loadable }";
        let parse = parsed(text);
        let unparsed = as_unparsed_item(&selections(&as_field(&parse).selection_set)[0].item);
        assert_eq!(
            unparsed.reason.item,
            expected(Expectation::Separator, Found::Token(At))
        );
        assert_eq!(unparsed.reason.location, span_of(text, "@"));
    }

    #[test]
    fn a_field_declaration_without_a_selection_set_degrades_the_literal() {
        let text = "field Query.Foo";
        let end = span_of(text, "Foo").end;
        assert_unparsed(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_selection_set_split_onto_its_own_line_degrades_the_literal() {
        let text = "field Query.Foo\n{ bar }";
        let end = span_of(text, "Foo").end;
        assert_unparsed(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn tokens_after_the_selection_set_are_leftover() {
        let text = "field Query.Foo { bar } junk";
        assert_unparsed(
            text,
            expected(Expectation::EndOfDeclaration, Found::Token(Identifier)),
            span_of(text, "junk"),
        );
    }

    #[test]
    fn errors_collect_in_source_order_across_nesting() {
        let text = "field Query.Foo {\n  a b\n  pet { c d }\n  e f\n}";
        let parse = parsed(text);
        let errors = parse.item.errors();
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "b"));
        assert_eq!(errors[1].location, span_of(text, "d"));
        assert_eq!(errors[2].location, span_of(text, "f"));
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionName(name) => {
                let scalar = match name.parent {
                    SelectionNameParent::Scalar(scalar) => scalar,
                    parent => panic!("expected a scalar parent, got {parent:?}"),
                };
                let object = match scalar.parent.parent {
                    SelectionSetParent::Object(object) => object,
                    parent => panic!("expected an object-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent.parent {
                    SelectionSetParent::Field(declaration) => {
                        assert_eq!(declaration.inner.client_field_name.location, span_of(text, "Foo"));
                    }
                    parent => panic!("expected the declaration at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn positions_inside_an_unparsed_selection_resolve_through_its_chunk() {
        let text = "field Query.Foo { bar baz }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, "baz")) {
            IsographResolutionNode::NonBracketToken(token) => {
                let chunk_parent = &token.parent.parent;
                match chunk_parent {
                    ChunkParent::UnparsedItem(unparsed) => {
                        assert_eq!(
                            unparsed.inner.reason.item,
                            expected(Expectation::Separator, Found::Token(Identifier))
                        );
                        match &unparsed.parent {
                            UnparsedItemParent::SelectionSet(_) => {}
                            parent => panic!("expected a selection-set parent, got {parent:?}"),
                        }
                    }
                    parent => panic!("expected an unparsed-item parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_and_separators_inside_a_selection_set_resolve_to_the_set() {
        let text = "field Query.Foo { bar, baz }";
        let parse = parsed(text);
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::SelectionSet(_) => {}
            node => panic!("expected the selection set, got {node:?}"),
        }
    }
```

## Landing checklist

1. The chunk.rs changes and their test respellings; `cargo test -p isograph_parser` passes before the rest lands.
2. selections.rs, the parse_iso_literal.rs and parse_error.rs changes, the resolution-node variants, and the tests; `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
3. Move this doc to refactors/past.
