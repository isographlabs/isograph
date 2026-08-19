# parse-selection-sets: selections and selection sets

Scalar selections, `alias: name`, object selections, and argument lists on those selections. Lands after parse-arguments.md. parse-fields.md is the host that requires a selection set on a declaration.

## The grammar this doc accepts

Each contentful chunk of a selection set is one selection:

```
[<Identifier> :] <Identifier> [<paren group>] [<brace group>]
```

The leading identifier is the alias when a colon follows, the name otherwise. A paren group is `consume_argument_list`. A brace group is an object selection whose interior recurses; without one it is a scalar selection.

Tests feed a list's interior to `parse_items`. The wrapping brace group lands with the host that requires it.

## Change 1: `Expectation`

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
```

Selection leftover is `Expectation::Separator(ClosingDelimiter::Brace)`.

## Change 2: `selections.rs`

```rust
// from crates/isograph_parser/src/selections.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, BracketKind, ClosingDelimiter, Expectation, IsographResolutionNode,
    NonBracketTokenKind, ParseError, Slot, UnparsedChunkItems, consume_argument_list,
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
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectSelection {
    #[resolve_field]
    #[parent_variant(Object)]
    pub reader_alias: Option<WithSpan<SelectionAlias>>,
    #[resolve_field]
    #[parent_variant(Object)]
    pub name: WithSpan<SelectionName>,
    #[resolve_field]
    #[parent_variant(Object)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Object)]
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

`SelectionSetParent::Object` is boxed to break the cycle `SelectionSetPath -> ObjectSelectionPath -> SelectionSetPath`. The derive's `parent.into()` converts through `From<T> for Box<T>`. parse-fields.md adds `Field`.

```rust
// from crates/isograph_parser/src/arguments.rs
#[derive(Debug)]
pub enum ArgumentListParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;
```

`ArgumentList`'s `parent_type` becomes `ArgumentListParent<'a>`.

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    Literal(IsoLiteralParsePath<'a>),
    ArgumentList(ArgumentListPath<'a>),
    ObjectLiteral(ObjectLiteralPath<'a>),
    SelectionSet(SelectionSetPath<'a>),
}

impl<'a> From<SelectionSetPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(parent: SelectionSetPath<'a>) -> Self {
        UnparsedChunkItemsParent::SelectionSet(parent)
    }
}
```

`From<ArgumentListPath>` and `From<ObjectLiteralPath>` stay. `Selection: ResolvePosition<Parent = SelectionSetPath>`, so `Slot<Selection, UnparsedChunkItems>::Parent` is `SelectionSetPath`. `From<SelectionSetPath> for SelectionSetPath` is identity (the `item` field). `From<SelectionSetPath> for UnparsedChunkItemsParent` is the `SelectionSet` variant (the `extra_tokens` field).

```rust
// from crates/isograph_parser/src/selections.rs
pub type SelectionSlotPath<'a> =
    PositionResolutionPath<&'a Slot<Selection, UnparsedChunkItems>, SelectionSetPath<'a>>;

impl<'a> From<SelectionSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: SelectionSlotPath<'a>) -> Self {
        IsographResolutionNode::SelectionSlot(path)
    }
}
```

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    SelectionSlot(SelectionSlotPath<'a>),
```

A gap in a selection slot answers `IsographResolutionNode::SelectionSlot`.

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
    let arguments = consume_argument_list(cursor, push_error);
    let selection_set = consume_selection_set(cursor, push_error);
    match selection_set {
        Some(selection_set) => Selection::Object(ObjectSelection {
            reader_alias,
            name,
            arguments,
            selection_set,
        }),
        None => Selection::Scalar(ScalarSelection {
            reader_alias,
            name,
            arguments,
        }),
    }
    .wrap_ok()
}
```

`lib.rs` adds `mod selections;` and `pub use selections::*;`.

## The resolution surface

```rust
// from crates/isograph_parser/src/isograph_resolution_node.rs
    SelectionSet(SelectionSetPath<'a>),
    ScalarSelection(ScalarSelectionPath<'a>),
    ObjectSelection(ObjectSelectionPath<'a>),
    SelectionName(SelectionNamePath<'a>),
    SelectionAlias(SelectionAliasPath<'a>),
```

A leftover token answers `NonBracketToken` through `UnparsedChunkItemsParent::SelectionSet` once a host supplies `SelectionSetParent`. A failed selection chunk is the same walk; the whole chunk's items sit in `extra_tokens`.

`SelectionSet` iterates the vec, each hit descending with the container's path. `Selection` delegates. `ObjectSelection` / `ScalarSelection` expand with `parent_variant` wrapping. `SelectionName` and `SelectionAlias` are interned-key leaves.

Resolve-from-a-declaration tests wait for parse-fields.md. This doc asserts parse structure.

## Tests

```rust
// from crates/isograph_parser/src/selections.rs
    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(
            &mut ItemCursor<'_>,
            &mut Vec<WithSpan<ParseError>>,
        ) -> Result<P, WithSpan<ParseError>>,
    ) -> (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
    ) {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let items = tree
            .item
            .parse_items(text, leftover, parse_item, &mut errors);
        (items, errors, comma_errors)
    }

    fn parsed_selections(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
    ) {
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(ClosingDelimiter::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
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

    #[test]
    fn scalar_selections_parse() {
        let text = "bar, baz";
        let (items, errors) = parsed_selections(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.item, "bar".intern().to());
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "baz"));
        assert_eq!(items[0].location, span_of(text, "bar"));
    }

    #[test]
    fn a_single_selection_parses_without_a_trailing_separator() {
        let text = "bar";
        let (items, errors) = parsed_selections(text);
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_selections() {
        for text in ["", "   ", "\n"] {
            let (items, errors) = parsed_selections(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_comma_before_the_first_selection_is_chunkings_error() {
        let text = ", bar";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(ClosingDelimiter::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);

        let lone = ",";
        let (items, errors, comma_errors) = parsed_items(
            lone,
            Expectation::Separator(ClosingDelimiter::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn an_alias_splits_from_the_name_at_the_colon() {
        let text = "b: bar";
        let (items, errors) = parsed_selections(text);
        let scalar = as_scalar(items[0].item.reference());
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
        let text = "pet { name, age }";
        let (items, errors) = parsed_selections(text);
        let object = as_object(items[0].item.reference());
        assert_eq!(object.name.location, span_of(text, "pet"));
        let inner = object.selection_set.item.0.reference();
        assert_eq!(inner.len(), 2);
        assert_eq!(as_scalar(inner[0].item.reference()).name.location, span_of(text, "name"));
        assert_eq!(as_scalar(inner[1].item.reference()).name.location, span_of(text, "age"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn arguments_parse_on_scalar_and_object_selections() {
        let text = "pet(id: $petId) { name(shouted: true) }";
        let (items, errors) = parsed_selections(text);
        assert_eq!(errors, vec![]);
        let object = as_object(items[0].item.reference());
        let outer = object
            .arguments
            .as_ref()
            .expect("the fixture selects with arguments");
        assert_eq!(outer.location, span_of(text, "(id: $petId)"));
        let inner = as_scalar(object.selection_set.item.0[0].item.reference())
            .arguments
            .as_ref()
            .expect("the nested selection selects with arguments");
        assert_eq!(inner.location, span_of(text, "(shouted: true)"));
    }

    #[test]
    fn an_orphaned_group_after_a_line_break_is_a_failed_selection() {
        let text = "bar\n{ baz }";
        let (items, errors) = parsed_selections(text);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert!(items[1].item.item.is_none());
        assert!(items[1].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Selection,
                    Found::Group(BracketKind::Brace),
                )
                && error.location == span_of(text, "{ baz }")
        }));
    }

    #[test]
    fn a_doubled_comma_between_selections_is_chunkings_error() {
        let text = "a,, b";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(ClosingDelimiter::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "a"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "b"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn leftover_after_a_selection_keeps_the_item() {
        let text = "bar baz\nqux";
        let (items, errors) = parsed_selections(text);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(ClosingDelimiter::Brace),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "baz")
        }));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "qux"));
    }

    #[test]
    fn a_directive_on_a_selection_is_trailing_leftover() {
        let text = "bar @loadable";
        let (items, errors) = parsed_selections(text);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(
            errors,
            ParseError::expected(
                Expectation::Separator(ClosingDelimiter::Brace),
                Found::Token(NonBracketTokenKind::At),
            )
            .with_span(span_of(text, "@"))
            .wrap_vec(),
        );
    }

    #[test]
    fn errors_collect_in_source_order_across_nesting() {
        let text = "a b\npet { c d }\ne f";
        let (items, errors) = parsed_selections(text);
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "b"));
        assert_eq!(errors[1].location, span_of(text, "d"));
        assert_eq!(errors[2].location, span_of(text, "f"));
        assert_eq!(
            as_scalar(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
    }
```

Resolve-from-a-declaration tests wait for parse-fields.md. Nested structure is `object_selections_nest`.

## Landing checklist

1. `Expectation::{SelectionSet, Selection}`, `selections.rs`, `ArgumentListParent::{Scalar, Object}`, `UnparsedChunkItemsParent::SelectionSet`, the resolution-node variants, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
