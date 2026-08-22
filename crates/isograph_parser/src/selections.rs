use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::WithSpan;

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, AstError, BracketKind, Expectation, IsographFieldDirectiveList,
    IsographResolutionNode, IsographSemanticToken, NonBracketTokenKind, Slot, UnparsedChunkItems,
    consume_argument_list, consume_directives,
};

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Selection {
    #[resolve_field]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectionName);

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    SelectableDeclaration(crate::SelectableDeclarationPath<'a>),
    Selection(Box<SelectionPath<'a>>),
}

pub type SelectionSetPath<'a> = PositionResolutionPath<&'a SelectionSet, SelectionSetParent<'a>>;

pub type SelectionSlotPath<'a> =
    PositionResolutionPath<&'a Slot<Selection, UnparsedChunkItems>, SelectionSetPath<'a>>;

pub type SelectionPath<'a> = PositionResolutionPath<&'a Selection, SelectionSlotPath<'a>>;

pub type SelectionNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectionNameWrapper, SelectionPath<'a>>;

impl<'a> From<SelectionSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: SelectionSlotPath<'a>) -> Self {
        IsographResolutionNode::SelectionSlot(path)
    }
}

pub(crate) fn consume_selection_set(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<SelectionSet>> {
    cursor.consume_group_if(
        BracketKind::Brace,
        IsographSemanticToken::Brace,
        |cursor, children| {
            SelectionSet(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_selection,
            ))
        },
    )
}

fn parse_selection(cursor: &mut ItemCursor<'_>) -> Result<Selection, WithSpan<AstError>> {
    let first = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::FieldName,
        )
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) =
        match cursor.consume_token_if(NonBracketTokenKind::Colon, IsographSemanticToken::Colon) {
            Some(_) => {
                let name = cursor
                    .require_token(
                        NonBracketTokenKind::Identifier,
                        IsographSemanticToken::FieldName,
                    )
                    .map_err(|()| {
                        cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier))
                    })?;
                (
                    first.interned().map(SelectionNameWrapper).wrap_some(),
                    name.interned().map(SelectionNameWrapper),
                )
            }
            None => (None, first.interned().map(SelectionNameWrapper)),
        };
    let arguments = consume_argument_list(cursor);
    let directive_set = consume_directives(cursor)?;
    let selection_set = consume_selection_set(cursor);
    Selection {
        reader_alias,
        name,
        arguments,
        directive_set,
        selection_set,
    }
    .wrap_ok()
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan};

    use super::*;
    use crate::{
        AstError, Found, IsographSemanticToken, NonBracketTokenKind,
        parsed_items::{parsed_items, span_of},
    };

    type ParsedSelections = (
        Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
        Vec<WithSpan<AstError>>,
    );

    fn parsed_selections(
        text: &str,
        expected_tokens: &[(IsographSemanticToken, &str)],
    ) -> ParsedSelections {
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
            expected_tokens,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
    }

    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
    }

    #[test]
    fn selections_without_a_nested_set() {
        let text = "bar, baz";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::FieldName, "baz"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.item,
            SelectionNameWrapper("bar".intern().to())
        );
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert_eq!(
            as_selection(items[1].item.reference()).name.location,
            span_of(text, "baz")
        );
        assert_eq!(items[0].location, span_of(text, "bar"));
    }

    #[test]
    fn a_single_selection_parses_without_a_trailing_separator() {
        let text = "bar";
        let (items, errors) = parsed_selections(text, &[(IsographSemanticToken::FieldName, "bar")]);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_selections() {
        for text in ["", "   ", "\n"] {
            let (items, errors) = parsed_selections(text, &[]);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_comma_before_the_first_selection_is_chunkings_error() {
        let text = ", bar";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
            &[(IsographSemanticToken::FieldName, "bar")],
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert_eq!(errors, vec![]);

        let lone = ",";
        let (items, errors, comma_errors) = parsed_items(
            lone,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
            &[],
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn an_alias_splits_from_the_name_at_the_colon() {
        let text = "b: bar";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::FieldName, "bar"),
            ],
        );
        let scalar = as_selection(items[0].item.reference());
        let alias = scalar
            .reader_alias
            .as_ref()
            .expect("the fixture selects with an alias");
        let alias_anchor = span_of(text, "b:");
        assert_eq!(alias.item, SelectionNameWrapper("b".intern().to()));
        assert_eq!(
            alias.location,
            Span::new(alias_anchor.start, alias_anchor.start + 1)
        );
        assert_eq!(scalar.name.location, span_of(text, "bar"));
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn selections_nest() {
        let text = "pet { name, age }";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "pet"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "name"),
                (IsographSemanticToken::FieldName, "age"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let object = as_selection(items[0].item.reference());
        assert_eq!(object.name.location, span_of(text, "pet"));
        let inner = object
            .selection_set
            .as_ref()
            .expect("the fixture selects a nested set")
            .item
            .0
            .reference();
        assert_eq!(inner.len(), 2);
        assert_eq!(
            as_selection(inner[0].item.reference()).name.location,
            span_of(text, "name")
        );
        assert_eq!(
            as_selection(inner[1].item.reference()).name.location,
            span_of(text, "age")
        );
        assert!(
            as_selection(inner[0].item.reference())
                .selection_set
                .is_none()
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn arguments_parse_on_selections() {
        let text = "pet(id: $petId) { name(shouted: true) }";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "pet"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "petId"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "name"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "shouted"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let object = as_selection(items[0].item.reference());
        let outer = object
            .arguments
            .as_ref()
            .expect("the fixture selects with arguments");
        assert_eq!(outer.location, span_of(text, "(id: $petId)"));
        let inner = as_selection(
            object
                .selection_set
                .as_ref()
                .expect("the fixture selects a nested set")
                .item
                .0[0]
                .item
                .reference(),
        )
        .arguments
        .as_ref()
        .expect("the nested selection selects with arguments");
        assert_eq!(inner.location, span_of(text, "(shouted: true)"));
    }

    #[test]
    fn an_orphaned_group_after_a_line_break_is_a_failed_selection() {
        let text = "bar\n{ baz }";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "baz"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert!(items[1].item.item.is_none());
        assert!(items[1].item.extra.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(Expectation::Selection, Found::Group(BracketKind::Brace))
                && error.location == span_of(text, "{ baz }")
        }));
    }

    #[test]
    fn a_doubled_comma_between_selections_is_chunkings_error() {
        let text = "a,, b";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
            &[
                (IsographSemanticToken::FieldName, "a"),
                (IsographSemanticToken::FieldName, "b"),
            ],
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
        assert_eq!(
            as_selection(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn leftover_after_a_selection_keeps_the_item() {
        let text = "bar baz\nqux";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Content, "baz"),
                (IsographSemanticToken::FieldName, "qux"),
            ],
        );
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert!(items[0].item.extra.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Separator(BracketKind::Brace),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "baz")
        }));
        assert_eq!(
            as_selection(items[1].item.reference()).name.location,
            span_of(text, "qux")
        );
    }

    #[test]
    fn a_period_where_a_selection_should_start_is_a_selection_error() {
        let text = "...UserAvatar";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "UserAvatar"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Selection,
                    Found::Token(NonBracketTokenKind::Period),
                )
                && error.location == Span::from_usize(0, 1)
        }));
    }

    #[test]
    fn errors_collect_in_source_order_across_nesting() {
        let text = "a b\npet { c d }\ne f";
        let (items, errors) = parsed_selections(
            text,
            &[
                (IsographSemanticToken::FieldName, "a"),
                (IsographSemanticToken::Content, "b"),
                (IsographSemanticToken::FieldName, "pet"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "c"),
                (IsographSemanticToken::Content, "d"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::FieldName, "e"),
                (IsographSemanticToken::Content, "f"),
            ],
        );
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "b"));
        assert_eq!(errors[1].location, span_of(text, "d"));
        assert_eq!(errors[2].location, span_of(text, "f"));
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
    }
}
