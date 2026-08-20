# parse-selection-sets: selections and selection sets

Scalar selections, `alias: name`, object selections, and argument lists on those selections. Lands after parse-arguments.md. parse-fields.md is the host that requires a selection set on a declaration.

## The grammar this doc accepts

Each contentful chunk of a selection set is one selection:

```
[<Identifier> :] <Identifier> [<paren group>] [<brace group>]
```

The leading identifier is the alias when a colon follows, the name otherwise. A paren group is `consume_argument_list`. A brace group is an object selection whose interior recurses; without one it is a scalar selection.

Tests feed a list's interior to `parse_each_chunk`. The wrapping brace group lands with the host that requires it.

## Change 1: `Expectation`

```rust
// from crates/isograph_parser/src/parse_error.rs
    #[error("a selection set, like '{{ id, name }}'")]
    SelectionSet,
    #[error("a field selection")]
    Selection,
```

Selection leftover is `Expectation::Separator(BracketKind::Brace)`.

## Change 2: `SelectionName` and `SelectionAlias`

```rust
// from crates/common_lang_types/src/string_key_types.rs
string_key_newtype!(SelectionName);
string_key_newtype!(SelectionAlias);
```

A selection's name is `SelectionName`, not `SelectableName`.

## Change 3: `selections.rs`

```rust
// from crates/isograph_parser/src/selections.rs
use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, BracketKind, Expectation, IsographResolutionNode, NonBracketTokenKind,
    ParseError, SemanticToken, Slot, UnparsedChunkItems, consume_argument_list,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionSet(
    #[resolve_field] pub Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum Selection {
    Scalar(ScalarSelection),
    Object(ObjectSelection),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ScalarSelection {
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub reader_alias: Option<WithSpan<SelectionAliasWrapper>>,
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(Scalar)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectSelection {
    #[resolve_field]
    #[parent_variant(Object)]
    pub reader_alias: Option<WithSpan<SelectionAliasWrapper>>,
    #[resolve_field]
    #[parent_variant(Object)]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(Object)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Object)]
    pub selection_set: WithSpan<SelectionSet>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(common_lang_types::SelectionName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionAliasWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionAliasWrapper(common_lang_types::SelectionAlias);

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    Object(Box<ObjectSelectionPath<'a>>),
}

#[derive(Debug)]
pub enum SelectionNameWrapperParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

#[derive(Debug)]
pub enum SelectionAliasWrapperParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

pub type SelectionSetPath<'a> = PositionResolutionPath<&'a SelectionSet, SelectionSetParent<'a>>;

pub type SelectionSlotPath<'a> =
    PositionResolutionPath<&'a Slot<Selection, UnparsedChunkItems>, SelectionSetPath<'a>>;

pub type ScalarSelectionPath<'a> =
    PositionResolutionPath<&'a ScalarSelection, SelectionSlotPath<'a>>;

pub type ObjectSelectionPath<'a> =
    PositionResolutionPath<&'a ObjectSelection, SelectionSlotPath<'a>>;

pub type SelectionNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectionNameWrapper, SelectionNameWrapperParent<'a>>;

pub type SelectionAliasWrapperPath<'a> =
    PositionResolutionPath<&'a SelectionAliasWrapper, SelectionAliasWrapperParent<'a>>;
```

`SelectionSetParent::Object` is boxed to break the cycle `SelectionSetPath -> ObjectSelectionPath -> SelectionSetPath`. The derive's `parent.into()` converts through `From<T> for Box<T>`. parse-fields.md adds `Field`.

`Selection` is the slot item. `ScalarSelection` and `ObjectSelection` share that slot path, as `EntrypointDeclaration` shares `IsoLiteralSlotPath` with `IsoLiteralItem`.

Before:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = ArgumentListParent, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>,
);

#[derive(Debug)]
pub enum ArgumentListParent {}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent>;
```

After:

```rust
// from crates/isograph_parser/src/arguments.rs
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(
    #[resolve_field] pub Vec<WithSpan<Slot<NamedArgument, UnparsedChunkItems>>>,
);

#[derive(Debug)]
pub enum ArgumentListParent<'a> {
    Scalar(ScalarSelectionPath<'a>),
    Object(ObjectSelectionPath<'a>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;
```

`ScalarSelection.arguments` and `ObjectSelection.arguments` take `#[parent_variant(Scalar)]` and `#[parent_variant(Object)]`.

Before:

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
}
```

After. Origin: those two listings. Delta: the `Selection` pin and the `SelectionSlot` leftover variant.

```rust
// from crates/isograph_parser/src/chunk.rs
    self_type_generics = [
        (<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>),
        (<NamedArgument, UnparsedChunkItems>, ArgumentListPath<'a>),
        (<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>),
        (<Selection, UnparsedChunkItems>, SelectionSetPath<'a>),
    ]
```

```rust
// from crates/isograph_parser/src/chunk.rs
pub enum UnparsedChunkItemsParent<'a> {
    IsoLiteralSlot(IsoLiteralSlotPath<'a>),
    NamedArgumentSlot(NamedArgumentSlotPath<'a>),
    ObjectEntrySlot(ObjectEntrySlotPath<'a>),
    SelectionSlot(SelectionSlotPath<'a>),
}

impl<'a> From<SelectionSlotPath<'a>> for UnparsedChunkItemsParent<'a> {
    fn from(path: SelectionSlotPath<'a>) -> Self {
        UnparsedChunkItemsParent::SelectionSlot(path)
    }
}
```

`From<IsoLiteralSlotPath>`, `From<NamedArgumentSlotPath>`, and `From<ObjectEntrySlotPath>` stay.

```rust
// from crates/isograph_parser/src/selections.rs
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
#[cfg_attr(not(test), expect(dead_code))]
pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    let group = cursor
        .require_group(BracketKind::Brace, SemanticToken::Brace)
        .map_err(|()| cursor.expected(Expectation::SelectionSet))?;
    let selection_set = SelectionSet(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_selection,
    ));
    cursor.record_group_close(group.item, SemanticToken::Brace);
    selection_set.with_span(group.location).wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn consume_selection_set(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<SelectionSet>> {
    let group = cursor.consume_group_if(BracketKind::Brace, SemanticToken::Brace)?;
    let selection_set = SelectionSet(group.item.children.item.parse_each_chunk(
        cursor,
        Expectation::Separator(BracketKind::Brace),
        parse_selection,
    ));
    cursor.record_group_close(group.item, SemanticToken::Brace);
    selection_set.with_span(group.location).wrap_some()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_selection(cursor: &mut ItemCursor<'_>) -> Result<Selection, WithSpan<ParseError>> {
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) = match cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon)
    {
        Some(_) => {
            let name = cursor
                .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                })?;
            (
                first.interned().map(SelectionAliasWrapper).wrap_some(),
                name.interned().map(SelectionNameWrapper),
            )
        }
        None => (None, first.interned().map(SelectionNameWrapper)),
    };
    let arguments = consume_argument_list(cursor);
    let selection_set = consume_selection_set(cursor);
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
    SelectionNameWrapper(SelectionNameWrapperPath<'a>),
    SelectionAliasWrapper(SelectionAliasWrapperPath<'a>),
```

A leftover token answers `NonBracketToken` through `UnparsedChunkItemsParent::SelectionSlot` once a host supplies `SelectionSetParent`. A failed selection chunk is the same walk; the whole chunk's items sit in `extra_tokens`.

`SelectionSet` iterates the vec, each hit descending with the container's path. `Selection` delegates. `ObjectSelection` / `ScalarSelection` expand with `parent_variant` wrapping. `SelectionNameWrapper` and `SelectionAliasWrapper` are interned-key leaves.

Resolve-from-a-declaration tests wait for parse-fields.md. This doc asserts parse structure.

## Tests

```rust
// from crates/isograph_parser/src/selections.rs
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        BracketKind, CommaWithoutItem, Expectation, Found, NonBracketTokenKind, ParseError,
        SemanticToken, chunk, match_brackets, tokenize,
    };

    type ParsedItems<P> = (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed_items<P>(
        text: &str,
        leftover: Expectation,
        parse_item: impl Fn(&mut ItemCursor<'_>) -> Result<P, WithSpan<ParseError>>,
    ) -> ParsedItems<P> {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let dummy = {
            let (brackets, _) = match_brackets(tokenize("x"), 1);
            chunk(brackets.reference()).0
        };
        let mut parent = dummy.item.0[0].item.stream(text, &mut tokens, &mut errors);
        let items = tree
            .item
            .parse_each_chunk(parent.cursor(), leftover, parse_item);
        (items, errors, comma_errors, tokens)
    }

    fn parsed_selections(
        text: &str,
    ) -> (
        Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<WithSpan<SemanticToken>>,
    ) {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
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
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_scalar(items[0].item.reference()).name.item,
            SelectionNameWrapper("bar".intern().to())
        );
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "baz"));
        assert_eq!(items[0].location, span_of(text, "bar"));
    }

    #[test]
    fn a_single_selection_parses_without_a_trailing_separator() {
        let text = "bar";
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_selections() {
        for text in ["", "   ", "\n"] {
            let (items, errors, _) = parsed_selections(text);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_comma_before_the_first_selection_is_chunkings_error() {
        let text = ", bar";
        let (items, errors, comma_errors, _) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 1);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);

        let lone = ",";
        let (items, errors, comma_errors, _) = parsed_items(
            lone,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn an_alias_splits_from_the_name_at_the_colon() {
        let text = "b: bar";
        let (items, errors, _) = parsed_selections(text);
        let scalar = as_scalar(items[0].item.reference());
        let alias = scalar
            .reader_alias
            .as_ref()
            .expect("the fixture selects with an alias");
        let alias_anchor = span_of(text, "b:");
        assert_eq!(alias.item, SelectionAliasWrapper("b".intern().to()));
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
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, comma_errors, _) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
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
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(items.len(), 2);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Separator(BracketKind::Brace),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "baz")
        }));
        assert_eq!(as_scalar(items[1].item.reference()).name.location, span_of(text, "qux"));
    }

    #[test]
    fn a_directive_on_a_selection_is_trailing_leftover() {
        let text = "bar @loadable";
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(as_scalar(items[0].item.reference()).name.location, span_of(text, "bar"));
        assert_eq!(
            errors,
            ParseError::expected(
                Expectation::Separator(BracketKind::Brace),
                Found::Token(NonBracketTokenKind::At),
            )
            .with_span(span_of(text, "@"))
            .wrap_vec(),
        );
    }

    #[test]
    fn errors_collect_in_source_order_across_nesting() {
        let text = "a b\npet { c d }\ne f";
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "b"));
        assert_eq!(errors[1].location, span_of(text, "d"));
        assert_eq!(errors[2].location, span_of(text, "f"));
        assert_eq!(
            as_scalar(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
    }

    #[test]
    fn a_selection_records_field_name_colon_and_braces() {
        let text = "b: pet { name }";
        let (_, errors, tokens) = parsed_selections(text);
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::FieldName.with_span(span_of(text, "b")),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::FieldName.with_span(span_of(text, "pet")),
                SemanticToken::Brace.with_span(span_of(text, "{")),
                SemanticToken::FieldName.with_span(span_of(text, "name")),
                SemanticToken::Brace.with_span(span_of(text, "}")),
            ],
        );
    }
```

Resolve-from-a-declaration tests wait for parse-fields.md. Nested structure is `object_selections_nest`.

## Landing checklist

1. `Expectation::{SelectionSet, Selection}`, `SelectionName` / `SelectionAlias`, `selections.rs`, `ArgumentListParent::{Scalar, Object}`, `UnparsedChunkItemsParent::SelectionSlot`, the `Selection` pin, the resolution-node variants, the tests. `cargo test -p isograph_parser` and the clippy pre-commit hook pass.
2. Move this doc to refactors/past.
