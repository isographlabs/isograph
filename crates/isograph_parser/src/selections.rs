use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::WithSpan;

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, BracketKind, Expectation, IsographResolutionNode, NonBracketTokenKind,
    ParseError, SemanticToken, Slot, UnparsedChunkItems, consume_argument_list,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSetParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionSet(#[resolve_field] pub Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Selection {
    #[resolve_field]
    pub reader_alias: Option<WithSpan<SelectionNameWrapper>>,
    #[resolve_field]
    pub name: WithSpan<SelectionNameWrapper>,
    #[resolve_field]
    pub arguments: Option<WithSpan<ArgumentList>>,
    #[resolve_field]
    #[parent_variant(Selection)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectionPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectionNameWrapper(pub common_lang_types::SelectionName);

#[derive(Debug)]
pub enum SelectionSetParent<'a> {
    FieldDeclaration(crate::FieldDeclarationPath<'a>),
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

pub(crate) fn require_selection_set(
    cursor: &mut ItemCursor<'_>,
    missing: Expectation,
) -> Result<WithSpan<SelectionSet>, WithSpan<ParseError>> {
    cursor
        .require_group(
            BracketKind::Brace,
            SemanticToken::Brace,
            |cursor, children| {
                SelectionSet(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_selection,
                ))
            },
        )
        .map_err(|()| cursor.expected(missing))
}

fn consume_selection_set(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<SelectionSet>> {
    cursor.consume_group_if(
        BracketKind::Brace,
        SemanticToken::Brace,
        |cursor, children| {
            SelectionSet(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Brace),
                parse_selection,
            ))
        },
    )
}

fn parse_selection(cursor: &mut ItemCursor<'_>) -> Result<Selection, WithSpan<ParseError>> {
    let first = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Selection))?;
    let (reader_alias, name) =
        match cursor.consume_token_if(NonBracketTokenKind::Colon, SemanticToken::Colon) {
            Some(_) => {
                let name = cursor
                    .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
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
    let selection_set = consume_selection_set(cursor);
    Selection {
        reader_alias,
        name,
        arguments,
        selection_set,
    }
    .wrap_ok()
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        CommaWithoutItem, Found, NonBracketTokenKind, ParseError, SemanticToken, chunk,
        match_brackets, tokenize,
    };

    type ParsedItems<P> = (
        Vec<WithSpan<Slot<P, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    type ParsedSelections = (
        Vec<WithSpan<Slot<Selection, UnparsedChunkItems>>>,
        Vec<WithSpan<ParseError>>,
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

    fn parsed_selections(text: &str) -> ParsedSelections {
        let (items, errors, comma_errors, tokens) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Brace),
            parse_selection,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors, tokens)
    }

    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
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
    fn selections_without_a_nested_set() {
        let text = "bar, baz";
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, _) = parsed_selections(text);
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
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
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
        let (items, errors, tokens) = parsed_selections(text);
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
        assert_eq!(
            tokens,
            vec![
                SemanticToken::FieldName
                    .with_span(Span::new(alias_anchor.start, alias_anchor.start + 1)),
                SemanticToken::Colon.with_span(span_of(text, ":")),
                SemanticToken::FieldName.with_span(span_of(text, "bar")),
            ]
        );
    }

    #[test]
    fn selections_nest() {
        let text = "pet { name, age }";
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, _) = parsed_selections(text);
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
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert!(items[1].item.item.is_none());
        assert!(items[1].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(Expectation::Selection, Found::Group(BracketKind::Brace))
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
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
        assert!(items[0].item.extra_tokens.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
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
    fn a_directive_on_a_selection_is_trailing_leftover() {
        let text = "bar @loadable";
        let (items, errors, _) = parsed_selections(text);
        assert_eq!(
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "bar")
        );
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
    fn a_period_where_a_selection_should_start_is_a_selection_error() {
        let text = "...UserAvatar";
        let (items, errors, _) = parsed_selections(text);
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == ParseError::expected(
                    Expectation::Selection,
                    Found::Token(NonBracketTokenKind::Period),
                )
                && error.location == Span::from_usize(0, 1)
        }));
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
            as_selection(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
    }
}
