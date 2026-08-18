# parse-fields: field declarations and selection sets

`field Type.name { ... }` declarations and selection sets. Lands `parse_items`. Lands after generic-slot.md.

## The grammar this doc accepts

```
field <Identifier> . <Identifier> <brace group>
```

The brace group is required and is the last item of the chunk. Each contentful chunk of its interior level is one selection:

```
[<Identifier> :] <Identifier> [<brace group>]
```

The leading identifier is the alias when a colon follows, the name otherwise. A selection with a brace group is an object selection whose interior recurses; without one it is a scalar selection. Arguments are not parsed: a paren group after a selection name is that selection's leftover.

## Change 1: `parse_items`

```rust
// from crates/isograph_parser/src/chunk.rs
impl ChunkedLevel {
    pub(crate) fn parse_items<'a, P, F>(
        &'a self,
        text: &'a str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'a>, &mut F) -> Result<P, WithSpan<ParseError>>,
        push_error: &mut F,
    ) -> Vec<WithSpan<Slot<P, UnparsedChunkItems>>>
    where
        F: FnMut(WithSpan<ParseError>),
    {
        self.0
            .iter()
            .map(|chunk| {
                parse_one_item(
                    chunk,
                    text,
                    leftover,
                    |cursor, push_error| parse_item(cursor, push_error),
                    push_error,
                )
            })
            .collect()
    }
}
```

A list trailing comma is legal and is not a diagnostic. Length equals chunk count.

## Change 2: `Expectation`

```rust
// from crates/isograph_parser/src/parse_error.rs
#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum Expectation {
    #[error("{0}")]
    Token(NonBracketTokenKind),
    #[error("one of `entrypoint`, `field`, or `pointer`")]
    DeclarationKeyword,
    #[error("the end of the declaration")]
    EndOfDeclaration,
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
    #[error("a comma, a line break, or {0}")]
    Separator(ClosingDelimiter),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum ClosingDelimiter {
    #[error("'}}'")]
    Brace,
    #[error("')'")]
    Parenthesis,
    #[error("']'")]
    Bracket,
}
```

Before, `Separator` is a unit variant (`"a comma or line break"`). This doc gives it the closer. Selection leftover is `Expectation::Separator(ClosingDelimiter::Brace)`.

## Change 3: `IsoLiteralItem::Field`

Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
```

After:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Field(ClientFieldDeclaration),
}
```

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
```

The test `field_and_pointer_declarations_do_not_parse_yet` narrows to its pointer case.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralParsePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldDeclaration {
    #[resolve_field(parent_variant = Field)]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field(parent_variant = Field)]
    pub client_field_name: WithSpan<ClientFieldName>,
    #[resolve_field(parent_variant = Field)]
    pub selection_set: WithSpan<SelectionSet>,
}

pub type ClientFieldDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientFieldDeclaration, IsoLiteralParsePath<'a>>;
```

There is no `FieldKeyword`. A position on `field` answers `ClientFieldDeclaration`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_field(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: cursor
            .token_text(parent_type)
            .intern()
            .to::<EntityName>()
            .with_span(parent_type),
        client_field_name: cursor
            .token_text(client_field_name)
            .intern()
            .to::<ClientFieldName>()
            .with_span(client_field_name),
        selection_set,
    }
    .wrap_ok()
}
```

`EntityName` and `ClientFieldName` gain a second parent. Before:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName(common_lang_types::EntityName);

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;
```

After (and identically for `ClientFieldName` with `ClientFieldNameParent`):

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
#[resolve_position(parent_type = EntityNameParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName(common_lang_types::EntityName);

#[derive(Debug)]
pub enum EntityNameParent<'a> {
    Entrypoint(EntrypointDeclarationPath<'a>),
    Field(ClientFieldDeclarationPath<'a>),
}

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntityNameParent<'a>>;
```

`EntrypointDeclaration`'s two marked fields respell from bare `#[resolve_field]` to `#[resolve_field(parent_variant = Entrypoint)]`.

The resolve test `names_resolve_to_their_leaves_and_the_rest_to_the_declaration` matches `name.parent.inner.client_field_name` through `EntityNameParent::Entrypoint`.

`as_entrypoint` gains an exhaustive arm:

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_entrypoint(parse: &WithSpan<IsoLiteralParse>) -> &EntrypointDeclaration {
        let item = parsed_item(parse).expect("the fixture's literal parsed an item");
        match item {
            IsoLiteralItem::Entrypoint(declaration) => declaration,
            item => panic!("expected an entrypoint declaration, got {item:?}"),
        }
    }
```

## Change 4: `selections.rs`

```rust
// from crates/isograph_parser/src/selections.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, ClientFieldDeclarationPath, ClosingDelimiter, Expectation,
    IsographResolutionNode, NonBracketTokenKind, ParseError, Slot, UnparsedChunkItems,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionSet(
    #[resolve_field] pub Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
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

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionNameParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionName(common_lang_types::SelectableName);

impl From<intern::string_key::StringKey> for SelectionName {
    fn from(key: intern::string_key::StringKey) -> Self {
        SelectionName(key.to())
    }
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionAliasParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionAlias(common_lang_types::SelectableAlias);

impl From<intern::string_key::StringKey> for SelectionAlias {
    fn from(key: intern::string_key::StringKey) -> Self {
        SelectionAlias(key.to())
    }
}

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    Field(ClientFieldDeclarationPath<'a>),
    Object(Box<ObjectSelectionPath<'a>>),
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

pub type SelectionNamePath<'a> = PositionResolutionPath<&'a SelectionName, SelectionNameParent<'a>>;

pub type SelectionAliasPath<'a> = PositionResolutionPath<&'a SelectionAlias, SelectionAliasParent<'a>>;
```

`SelectionSetParent::Object` is boxed to break the cycle `SelectionSetPath -> ObjectSelectionPath -> SelectionSetPath`. The derive's `parent.into()` converts through `From<T> for Box<T>`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
}
```

After:

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    SelectionSet(SelectionSetPath<'a>),
}

impl<'a> From<SelectionSetPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: SelectionSetPath<'a>) -> Self {
        UnparsedChunkItemsParent::SelectionSet(parent)
    }
}
```

`From<IsoLiteralParsePath>` stays. `Selection: ResolvePosition<Parent = SelectionSetPath>`, so `Slot<Selection, UnparsedChunkItems>::Parent` is `SelectionSetPath`. `From<SelectionSetPath> for SelectionSetPath` is identity (the `item` field). `From<SelectionSetPath> for UnparsedChunkItemsParent` is the `SelectionSet` variant (the `extra_tokens` field).

```rust
// from crates/isograph_parser/src/selections.rs
pub(crate) fn require_selection_set<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let group = cursor
        .require_group(BracketKind::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    SelectionSet(group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Brace),
        parse_selection,
        push_error,
    ))
    .with_span(group.location)
    .wrap_ok()
}

fn consume_selection_set<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Option<WithSpan<SelectionSet>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let group = cursor.consume_group_if(BracketKind::Brace)?;
    SelectionSet(group.item.children.item.parse_items(
        cursor.text(),
        Expectation::Separator(ClosingDelimiter::Brace),
        parse_selection,
        push_error,
    ))
    .with_span(group.location)
    .wrap_some()
}

fn parse_selection<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<Selection, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon) {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            (
                cursor
                    .token_text(first)
                    .intern()
                    .to::<SelectionAlias>()
                    .with_span(first)
                    .wrap_some(),
                cursor
                    .token_text(name)
                    .intern()
                    .to::<SelectionName>()
                    .with_span(name),
            )
        }
        None => (
            None,
            cursor
                .token_text(first)
                .intern()
                .to::<SelectionName>()
                .with_span(first),
        ),
    };
    let selection_set = consume_selection_set(cursor, push_error);
    match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection { reader_alias, name }),
    }
    .wrap_ok()
}
```

`parse_field` threads `push_error` into `require_selection_set`. `parse_iso_literal_item` already receives `push_error` from `parse_singleton` and ignores it (`|cursor, _|`). After this doc it is `|cursor, push_error| parse_iso_literal_item(cursor, push_error)`, and `parse_field` / `parse_entrypoint` take `&mut F` only as far as they call a nested list. `parse_entrypoint` stays `|cursor, _|`.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
fn parse_iso_literal_item<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<IsoLiteralItem, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Field(parse_field(cursor, push_error)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
}

fn parse_field<F>(
    cursor: &mut ItemCursor<'_>,
    push_error: &mut F,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>>
where
    F: FnMut(WithSpan<ParseError>),
{
    /* header tokens as above */
    let selection_set = require_selection_set(cursor, push_error)?;
    /* interned names as above */
}
```

`lib.rs` adds `mod selections;` and `pub use selections::*;`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
    SelectionName(SelectionNamePath<'a>),
    SelectionAlias(SelectionAliasPath<'a>),
```

A position on a list comma or on whitespace inside a selection set answers `SelectionSet`. A leftover token answers `NonBracketToken` through `UnparsedChunkItemsParent::SelectionSet`. A failed selection chunk is the same walk; the whole chunk's items sit in `extra_tokens`.

`SelectionSet` iterates the vec, each hit descending with the container's path. `Selection` delegates. `ObjectSelection` / `ScalarSelection` / `ClientFieldDeclaration` expand like `EntrypointDeclaration` with `parent_variant` wrapping. `SelectionName` and `SelectionAlias` are interned-key leaves like `EntityName`.

## Tests

Extending the `parse_iso_literal.rs` test module. Helpers `parsed`, `parsed_with_errors`, `span_of`, `expected`, `token`, `first_slot`, `parsed_item` stay.

```rust
// from crates/isograph_parser/src/parse_iso_literal.rs
    fn as_field(parse: &WithSpan<IsoLiteralParse>) -> &ClientFieldDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Field(declaration) => declaration,
            item => panic!("expected a field declaration, got {item:?}"),
        }
    }

    fn selections(
        selection_set: &WithSpan<SelectionSet>,
    ) -> &[WithSpan<Slot<Selection, UnparsedChunkItems>>] {
        selection_set.item.0.reference()
    }

    fn as_scalar(slot: &Slot<Selection, UnparsedChunkItems>) -> &ScalarSelection {
        match slot.item.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(Selection::Scalar(scalar)) => scalar,
            other => panic!("expected a scalar selection, got {other:?}"),
        }
    }

    fn as_object(slot: &Slot<Selection, UnparsedChunkItems>) -> &ObjectSelection {
        match slot.item.as_ref().map(|wrapped| wrapped.item.reference()) {
            Some(Selection::Object(object)) => object,
            other => panic!("expected an object selection, got {other:?}"),
        }
    }

    #[test]
    fn a_field_declaration_parses_with_scalar_selections() {
        let text = "field Query.Foo {\n  bar,\n  baz\n}";
        let (parse, errors) = parsed(text);
        let declaration = as_field(parse.reference());
        assert_eq!(declaration.parent_type.item, "Query".intern().to());
        assert_eq!(declaration.client_field_name.item, "Foo".intern().to());
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "Foo"));
        assert_eq!(
            declaration.selection_set.location,
            Span::new(span_of(text, "{").start, span_of(text, "}").end)
        );
        let items = selections(declaration.selection_set.reference());
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.item, "bar".intern().to());
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "baz"));
        assert_eq!(items[0].location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_single_line_selection_set_parses_without_a_trailing_separator() {
        let text = "field Query.Foo { bar }";
        let (parse, errors) = parsed(text);
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_selection_sets_hold_zero_selections() {
        for text in [
            "field Query.Foo {}",
            "field Query.Foo { }",
            "field Query.Foo {\n}",
        ] {
            let (parse, errors) = parsed(text);
            assert_eq!(
                selections(as_field(parse.reference()).selection_set.reference()).len(),
                0,
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_comma_before_the_first_selection_is_chunkings_error() {
        let text = "field Query.Foo {, bar }";
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        let parse = parse.expect("the fixture is not an empty literal");
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);

        let lone = "field Query.Foo {,}";
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(lone);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        let parse = parse.expect("the fixture is not an empty literal");
        assert_eq!(
            selections(as_field(parse.reference()).selection_set.reference()).len(),
            0
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn an_alias_splits_from_the_name_at_the_colon() {
        let text = "field Query.Foo { b: bar }";
        let (parse, errors) = parsed(text);
        let scalar = as_scalar(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        let alias = scalar
            .reader_alias
            .as_ref()
            .expect("the fixture selects with an alias");
        let alias_anchor = span_of(text, "b:");
        assert_eq!(alias.item, "b".intern().to());
        assert_eq!(
            alias.location,
            Span::new(alias_anchor.start, alias_anchor.start + 1)
        );
        assert_eq!(scalar.name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn object_selections_nest() {
        let text = "field Query.Foo { pet { name, age } }";
        let (parse, errors) = parsed(text);
        let object = as_object(
            selections(as_field(parse.reference()).selection_set.reference())[0]
                .item
                .reference(),
        );
        assert_eq!(object.name.location, span_of(text, "pet"));
        let inner = selections(object.selection_set.reference());
        assert_eq!(inner.len(), 2);
        assert_eq!(as_scalar(inner[0].item.reference()).name.location, span_of(text, "name"));
        assert_eq!(as_scalar(inner[1].item.reference()).name.location, span_of(text, "age"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn an_orphaned_group_after_a_line_break_is_a_failed_selection() {
        let text = "field Query.Foo {\n  bar\n  { baz }\n}";
        let (parse, errors) = parsed(text);
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert!(items[1].item.item.is_none());
        assert!(items[1].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item == expected(Expectation::Selection, Found::Group(BracketKind::Brace))
                && error.location == span_of(text, "{ baz }")
        }));
    }

    #[test]
    fn a_doubled_comma_between_selections_is_chunkings_error() {
        let text = "field Query.Foo { a,, b }";
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        let parse = parse.expect("the fixture is not an empty literal");
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "a"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "b"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn leftover_after_a_selection_keeps_the_item() {
        let text = "field Query.Foo {\n  bar baz\n  qux\n}";
        let (parse, errors) = parsed(text);
        let items = selections(as_field(parse.reference()).selection_set.reference());
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::Separator(ClosingDelimiter::Brace),
                    Found::Token(Identifier),
                )
                && error.location == span_of(text, "baz")
        }));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "qux"));
    }

    #[test]
    fn arguments_are_trailing_leftover_until_parse_arguments() {
        let text = "field Query.Foo { bar(x: 1) }";
        let (parse, errors) = parsed(text);
        let slot = selections(as_field(parse.reference()).selection_set.reference())[0]
            .item
            .reference();
        assert_eq!(as_scalar(slot).name.location, span_of(text, "bar"));
        assert!(slot.extra_tokens.is_some());
        assert_eq!(
            errors,
            expected(
                Expectation::Separator(ClosingDelimiter::Brace),
                Found::Group(BracketKind::Parenthesis)
            )
            .with_span(span_of(text, "(x: 1)"))
            .wrap_vec(),
        );
    }

    #[test]
    fn a_directive_on_a_selection_is_trailing_leftover() {
        let text = "field Query.Foo { bar @loadable }";
        let (parse, errors) = parsed(text);
        let slot = selections(as_field(parse.reference()).selection_set.reference())[0]
            .item
            .reference();
        assert_eq!(as_scalar(slot).name.location, span_of(text, "bar"));
        assert_eq!(
            errors,
            expected(Expectation::Separator(ClosingDelimiter::Brace), Found::Token(At))
                .with_span(span_of(text, "@"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_field_declaration_without_a_selection_set_is_a_failed_item() {
        let text = "field Query.Foo";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_selection_set_split_onto_its_own_line_is_a_failed_item() {
        let text = "field Query.Foo\n{ bar }";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_final_comma_after_the_field_declaration_is_an_error() {
        let text = "field Query.Foo { bar },";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn tokens_after_the_selection_set_are_leftover() {
        let text = "field Query.Foo { bar } junk";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert!(first_slot(parse.reference()).extra_tokens.is_some());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "junk"))
                .wrap_vec(),
        );
    }

    #[test]
    fn errors_collect_in_source_order_across_nesting() {
        let text = "field Query.Foo {\n  a b\n  pet { c d }\n  e f\n}";
        let (parse, errors) = parsed(text);
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "b"));
        assert_eq!(errors[1].location, span_of(text, "d"));
        assert_eq!(errors[2].location, span_of(text, "f"));
        assert_eq!(
            as_scalar(
                selections(as_field(parse.reference()).selection_set.reference())[0]
                    .item
                    .reference()
            )
            .name
            .location,
            span_of(text, "a")
        );
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionName(name) => {
                let scalar = match name.parent {
                    SelectionNameParent::Scalar(scalar) => scalar,
                    parent => panic!("expected a scalar parent, got {parent:?}"),
                };
                let object = match scalar.parent {
                    SelectionSetParent::Object(object) => object,
                    parent => panic!("expected an object-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent {
                    SelectionSetParent::Field(declaration) => {
                        assert_eq!(
                            declaration.inner.client_field_name.location,
                            span_of(text, "Foo")
                        );
                    }
                    parent => panic!("expected the declaration at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn leftover_positions_resolve_to_the_leftover_token() {
        let text = "field Query.Foo { bar baz }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "baz")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::SelectionName(_) => {}
            node => panic!("expected the selection name, got {node:?}"),
        }
    }

    #[test]
    fn positions_inside_a_failed_selection_resolve_through_unparsed_items() {
        let text = "field Query.Foo { 42 }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "42")) {
            IsographResolutionNode::NonBracketToken(token) => {
                match token.parent {
                    ChunkContentItemParent::Unparsed(unparsed) => match unparsed.parent {
                        UnparsedChunkItemsParent::SelectionSet(_) => {}
                        parent => panic!("expected a selection-set parent, got {parent:?}"),
                    },
                    parent => panic!("expected an unparsed parent, got {parent:?}"),
                }
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_and_separators_inside_a_selection_set_resolve_to_the_set() {
        let text = "field Query.Foo { bar, baz }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::SelectionSet(_) => {}
            node => panic!("expected the selection set, got {node:?}"),
        }
    }
```

`field_and_pointer_declarations_do_not_parse_yet` keeps only the pointer fixture.

## Landing checklist

1. `parse_items`, `selections.rs`, `IsoLiteralItem::Field`, `Expectation::{SelectionSet, Selection}`, `ClosingDelimiter`, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
