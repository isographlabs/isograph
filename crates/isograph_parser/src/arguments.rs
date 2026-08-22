use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, BracketKind, ChunkContentItem, Expectation, Found, IsographFieldDirectivePath,
    IsographResolutionNode, IsographSemanticToken, NonBracketToken, NonBracketTokenKind,
    SelectionPath, Slot, UnparsedChunkItems, VariableDeclarationPath, intern_block_string_value,
};

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ArgumentList(#[resolve_field] pub Vec<WithSpan<Slot<Argument, UnparsedChunkItems>>>);

#[derive(Debug)]
pub enum ArgumentListParent<'a> {
    Selection(SelectionPath<'a>),
    IsographFieldDirective(IsographFieldDirectivePath<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectLiteral(#[resolve_field] pub Vec<WithSpan<Slot<ObjectEntry, UnparsedChunkItems>>>);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteral(
    #[resolve_field] pub Vec<WithSpan<Slot<ListLiteralValue, UnparsedChunkItems>>>,
);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ArgumentSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Argument {
    #[resolve_field]
    pub name: WithSpan<ArgumentNameWrapper>,
    #[resolve_field]
    #[parent_variant(Argument)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntrySlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ObjectEntry {
    #[resolve_field]
    pub name: WithSpan<ValueKeyNameWrapper>,
    #[resolve_field]
    #[parent_variant(ObjectEntry)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ListLiteralValueSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListLiteralValue {
    #[resolve_field]
    #[parent_variant(List)]
    pub value: WithSpan<NonConstantValue>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum NonConstantValue {
    Variable(VariableUse),
    String(StringLiteralValueWrapper),
    Integer(IntegerValue),
    Boolean(BooleanValue),
    Null(NullValue),
    Object(ObjectLiteral),
    List(ListLiteral),
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsageParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsage(#[resolve_field] pub WithSpan<VariableNameWrapper>);

#[derive(Debug)]
pub enum VariableDeclarationOrUsageParent<'a> {
    Declaration(VariableDeclarationPath<'a>),
    Usage(VariableUsePath<'a>),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableUse(
    #[resolve_field]
    #[parent_variant(Usage)]
    pub WithSpan<VariableDeclarationOrUsage>,
);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct StringLiteralValueWrapper(pub common_lang_types::StringLiteralValue);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IntegerValue(pub i64);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = NonConstantValueParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullValue;

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ArgumentPath<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct ArgumentNameWrapper(pub common_lang_types::ArgumentName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ObjectEntryPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ValueKeyNameWrapper(pub common_lang_types::ValueKeyName);

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsagePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableNameWrapper(pub common_lang_types::VariableName);

#[derive(Debug)]
pub enum NonConstantValueParent<'a> {
    Argument(Box<ArgumentPath<'a>>),
    ObjectEntry(Box<ObjectEntryPath<'a>>),
    VariableDefault(VariableDeclarationPath<'a>),
    List(Box<ListLiteralValuePath<'a>>),
}

pub type ArgumentListPath<'a> = PositionResolutionPath<&'a ArgumentList, ArgumentListParent<'a>>;

pub type ObjectLiteralPath<'a> =
    PositionResolutionPath<&'a ObjectLiteral, NonConstantValueParent<'a>>;

pub type ListLiteralPath<'a> = PositionResolutionPath<&'a ListLiteral, NonConstantValueParent<'a>>;

pub type ListLiteralValueSlotPath<'a> =
    PositionResolutionPath<&'a Slot<ListLiteralValue, UnparsedChunkItems>, ListLiteralPath<'a>>;

pub type ListLiteralValuePath<'a> =
    PositionResolutionPath<&'a ListLiteralValue, ListLiteralValueSlotPath<'a>>;

pub type ArgumentSlotPath<'a> =
    PositionResolutionPath<&'a Slot<Argument, UnparsedChunkItems>, ArgumentListPath<'a>>;

pub type ObjectEntrySlotPath<'a> =
    PositionResolutionPath<&'a Slot<ObjectEntry, UnparsedChunkItems>, ObjectLiteralPath<'a>>;

pub type ArgumentPath<'a> = PositionResolutionPath<&'a Argument, ArgumentSlotPath<'a>>;

pub type ObjectEntryPath<'a> = PositionResolutionPath<&'a ObjectEntry, ObjectEntrySlotPath<'a>>;

pub type VariableUsePath<'a> = PositionResolutionPath<&'a VariableUse, NonConstantValueParent<'a>>;

pub type StringLiteralValueWrapperPath<'a> =
    PositionResolutionPath<&'a StringLiteralValueWrapper, NonConstantValueParent<'a>>;

pub type IntegerValuePath<'a> =
    PositionResolutionPath<&'a IntegerValue, NonConstantValueParent<'a>>;

pub type BooleanValuePath<'a> =
    PositionResolutionPath<&'a BooleanValue, NonConstantValueParent<'a>>;

pub type NullValuePath<'a> = PositionResolutionPath<&'a NullValue, NonConstantValueParent<'a>>;

pub type ArgumentNameWrapperPath<'a> =
    PositionResolutionPath<&'a ArgumentNameWrapper, ArgumentPath<'a>>;

pub type ValueKeyNameWrapperPath<'a> =
    PositionResolutionPath<&'a ValueKeyNameWrapper, ObjectEntryPath<'a>>;

pub type VariableDeclarationOrUsagePath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsage, VariableDeclarationOrUsageParent<'a>>;

pub type VariableNameWrapperPath<'a> =
    PositionResolutionPath<&'a VariableNameWrapper, VariableDeclarationOrUsagePath<'a>>;

impl<'a> From<ArgumentSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ArgumentSlotPath<'a>) -> Self {
        IsographResolutionNode::ArgumentSlot(path)
    }
}

impl<'a> From<ObjectEntrySlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ObjectEntrySlotPath<'a>) -> Self {
        IsographResolutionNode::ObjectEntrySlot(path)
    }
}

impl<'a> From<ListLiteralValueSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: ListLiteralValueSlotPath<'a>) -> Self {
        IsographResolutionNode::ListLiteralValueSlot(path)
    }
}

pub(crate) fn parse_name_colon<L, R>(
    cursor: &mut ItemCursor<'_>,
    parse_lhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<L, WithSpan<AstError>>,
    parse_rhs: impl FnOnce(&mut ItemCursor<'_>) -> Result<R, WithSpan<AstError>>,
) -> Result<(L, R), WithSpan<AstError>> {
    let lhs = parse_lhs(cursor)?;
    cursor
        .require_token(NonBracketTokenKind::Colon, IsographSemanticToken::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Colon)))?;
    let rhs = parse_rhs(cursor)?;
    (lhs, rhs).wrap_ok()
}

fn require_interned_identifier<N: From<intern::string_key::StringKey>>(
    cursor: &mut ItemCursor<'_>,
    name_token: IsographSemanticToken,
    missing_name: Expectation,
) -> Result<WithSpan<N>, WithSpan<AstError>> {
    let name = cursor
        .require_token(NonBracketTokenKind::Identifier, name_token)
        .map_err(|()| cursor.expected(missing_name))?;
    name.interned().wrap_ok()
}

fn parse_argument(cursor: &mut ItemCursor<'_>) -> Result<Argument, WithSpan<AstError>> {
    let (name, value) = parse_name_colon(
        cursor,
        |cursor| {
            require_interned_identifier(
                cursor,
                IsographSemanticToken::Argument,
                Expectation::Argument,
            )
        },
        parse_non_constant_value,
    )?;
    Argument {
        name: name.map(ArgumentNameWrapper),
        value,
    }
    .wrap_ok()
}

fn parse_object_entry(cursor: &mut ItemCursor<'_>) -> Result<ObjectEntry, WithSpan<AstError>> {
    let (name, value) = parse_name_colon(
        cursor,
        |cursor| {
            require_interned_identifier(
                cursor,
                IsographSemanticToken::ObjectKey,
                Expectation::ObjectEntry,
            )
        },
        parse_non_constant_value,
    )?;
    ObjectEntry {
        name: name.map(ValueKeyNameWrapper),
        value,
    }
    .wrap_ok()
}

pub(crate) fn consume_argument_list(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<ArgumentList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        IsographSemanticToken::Parenthesis,
        |cursor, children| {
            ArgumentList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_argument,
            ))
        },
    )
}

pub(crate) fn parse_variable_name(
    cursor: &mut ItemCursor<'_>,
    missing_dollar: Expectation,
) -> Result<WithSpan<VariableDeclarationOrUsage>, WithSpan<AstError>> {
    cursor.spanning(|cursor| {
        cursor
            .require_token(NonBracketTokenKind::Dollar, IsographSemanticToken::Variable)
            .map_err(|()| cursor.expected(missing_dollar))?;
        let name = cursor
            .require_token(
                NonBracketTokenKind::Identifier,
                IsographSemanticToken::Variable,
            )
            .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
        VariableDeclarationOrUsage(name.interned().map(VariableNameWrapper)).wrap_ok()
    })
}

fn parse_string_literal(
    cursor: &mut ItemCursor<'_>,
) -> Result<StringLiteralValueWrapper, WithSpan<AstError>> {
    if let Some(span) = cursor.consume_token_if(
        NonBracketTokenKind::StringLiteral,
        IsographSemanticToken::String,
    ) {
        return StringLiteralValueWrapper(span.exclude_ends(1).interned().item).wrap_ok();
    }
    if let Some(span) = cursor.consume_token_if(
        NonBracketTokenKind::BlockStringLiteral,
        IsographSemanticToken::String,
    ) {
        return StringLiteralValueWrapper(intern_block_string_value(span.exclude_ends(3).text()))
            .wrap_ok();
    }
    cursor
        .expected(Expectation::Token(NonBracketTokenKind::StringLiteral))
        .wrap_err()
}

fn parse_integer_value(cursor: &mut ItemCursor<'_>) -> Result<IntegerValue, WithSpan<AstError>> {
    let span = cursor
        .require_token(
            NonBracketTokenKind::IntegerLiteral,
            IsographSemanticToken::Integer,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::IntegerLiteral)))?;
    match span.text().parse() {
        Ok(value) => IntegerValue(value).wrap_ok(),
        Err(_) => AstError::IntegerDoesNotFitI64
            .with_span(span.location)
            .wrap_err(),
    }
}

fn parse_boolean_or_null(
    cursor: &mut ItemCursor<'_>,
) -> Result<NonConstantValue, WithSpan<AstError>> {
    let span = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::BooleanOrNull,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    match span.text() {
        "true" => NonConstantValue::Boolean(BooleanValue(Boolean::True)).wrap_ok(),
        "false" => NonConstantValue::Boolean(BooleanValue(Boolean::False)).wrap_ok(),
        "null" => NonConstantValue::Null(NullValue).wrap_ok(),
        _ => AstError::expected(
            Expectation::Value,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(span.location)
        .wrap_err(),
    }
}

fn parse_list_literal_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<ListLiteralValue, WithSpan<AstError>> {
    let value = parse_non_constant_value(cursor)?;
    ListLiteralValue { value }.wrap_ok()
}

fn parse_object_literal(cursor: &mut ItemCursor<'_>) -> Result<ObjectLiteral, WithSpan<AstError>> {
    let object = cursor
        .require_group(
            BracketKind::Brace,
            IsographSemanticToken::Brace,
            |cursor, children| {
                ObjectLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Brace),
                    parse_object_entry,
                ))
            },
        )
        .map_err(|()| cursor.expected(Expectation::Value))?;
    object.item.wrap_ok()
}

pub(crate) fn parse_non_constant_value(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NonConstantValue>, WithSpan<AstError>> {
    cursor.spanning(|cursor| {
        match cursor.peek().map(|peek| peek.view().item.reference()) {
            Some(ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Dollar))) => {
                return NonConstantValue::Variable(VariableUse(parse_variable_name(
                    cursor,
                    Expectation::Token(NonBracketTokenKind::Dollar),
                )?))
                .wrap_ok();
            }
            Some(ChunkContentItem::NonBracket(NonBracketToken(
                NonBracketTokenKind::StringLiteral | NonBracketTokenKind::BlockStringLiteral,
            ))) => {
                return NonConstantValue::String(parse_string_literal(cursor)?).wrap_ok();
            }
            Some(ChunkContentItem::NonBracket(NonBracketToken(
                NonBracketTokenKind::IntegerLiteral,
            ))) => {
                return NonConstantValue::Integer(parse_integer_value(cursor)?).wrap_ok();
            }
            Some(ChunkContentItem::NonBracket(NonBracketToken(
                NonBracketTokenKind::Identifier,
            ))) => {
                return parse_boolean_or_null(cursor);
            }
            Some(ChunkContentItem::Group(group)) if group.opening.item.0 == BracketKind::Brace => {
                return NonConstantValue::Object(parse_object_literal(cursor)?).wrap_ok();
            }
            _ => {}
        }
        if let Some(list) = cursor.consume_group_if(
            BracketKind::Bracket,
            IsographSemanticToken::Bracket,
            |cursor, children| {
                ListLiteral(children.item.parse_each_chunk(
                    cursor,
                    Expectation::Separator(BracketKind::Bracket),
                    parse_list_literal_value,
                ))
            },
        ) {
            return NonConstantValue::List(list.item).wrap_ok();
        }
        cursor.expected(Expectation::Value).wrap_err()
    })
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan};

    use super::*;
    use crate::{
        AstError, Found, IsographSemanticToken, NonBracketTokenKind,
        assert_semantic_tokens::assert_semantic_tokens,
        chunk,
        chunk_stream::ChunkStream,
        match_brackets,
        parsed_items::{parsed_items, span_of},
        tokenize,
    };

    type ParsedPairs = (
        Vec<WithSpan<Slot<Argument, UnparsedChunkItems>>>,
        Vec<WithSpan<AstError>>,
    );

    type ParsedArgumentList = (Option<WithSpan<ArgumentList>>, Vec<WithSpan<AstError>>);

    fn parsed_pairs(text: &str, expected_tokens: &[(IsographSemanticToken, &str)]) -> ParsedPairs {
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
            expected_tokens,
        );
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (items, errors)
    }

    fn parsed_argument_list(
        text: &str,
        expected_tokens: &[(IsographSemanticToken, &str)],
    ) -> ParsedArgumentList {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let mut stream = ChunkStream::new(tree.item.0.as_slice(), text, &mut tokens, &mut errors);
        let list = consume_argument_list(stream.cursor());
        assert_semantic_tokens(text, &tokens, expected_tokens);
        (list, errors)
    }

    fn as_argument(slot: &Slot<Argument, UnparsedChunkItems>) -> &Argument {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a named argument")
    }

    fn as_entry(slot: &Slot<ObjectEntry, UnparsedChunkItems>) -> &ObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected an object entry")
    }

    fn as_list_value(slot: &Slot<ListLiteralValue, UnparsedChunkItems>) -> &ListLiteralValue {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a list value")
    }

    #[test]
    fn pairs_parse_as_name_colon_value() {
        let text = "id: $petId, shouted: true";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "petId"),
                (IsographSemanticToken::Argument, "shouted"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            ArgumentNameWrapper("id".intern().to())
        );
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, "$petId")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "shouted")
        );
    }

    #[test]
    fn each_value_kind_parses() {
        let text = r#"a: $x, b: "hi", c: 42, d: -7, e: true, f: false, g: null"#;
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Argument, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"hi\""),
                (IsographSemanticToken::Argument, "c"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "42"),
                (IsographSemanticToken::Argument, "d"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "-7"),
                (IsographSemanticToken::Argument, "e"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
                (IsographSemanticToken::Argument, "f"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "false"),
                (IsographSemanticToken::Argument, "g"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "null"),
            ],
        );
        assert_eq!(errors, vec![]);
        let values: Vec<&NonConstantValue> = items
            .iter()
            .map(|slot| as_argument(slot.item.reference()).value.item.reference())
            .collect();
        assert!(matches!(values[0], NonConstantValue::Variable(_)));
        assert!(matches!(values[1], NonConstantValue::String(_)));
        assert!(matches!(
            values[2],
            NonConstantValue::Integer(IntegerValue(42))
        ));
        assert!(matches!(
            values[3],
            NonConstantValue::Integer(IntegerValue(-7))
        ));
        assert!(matches!(
            values[4],
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
        assert!(matches!(
            values[5],
            NonConstantValue::Boolean(BooleanValue(Boolean::False))
        ));
        assert!(matches!(values[6], NonConstantValue::Null(_)));
    }

    #[test]
    fn object_values_use_braces() {
        let text = "input: { id: 4, nested: { on: true } }";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "input"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::ObjectKey, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "4"),
                (IsographSemanticToken::ObjectKey, "nested"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::ObjectKey, "on"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let value = as_argument(items[0].item.reference()).value.reference();
        assert_eq!(
            value.location,
            span_of(text, "{ id: 4, nested: { on: true } }")
        );
        let object = match value.item.reference() {
            NonConstantValue::Object(object) => object,
            value => panic!("expected an object, got {value:?}"),
        };
        assert_eq!(object.0.len(), 2);
        assert_eq!(
            as_entry(object.0[1].item.reference()).name.location,
            span_of(text, "nested")
        );
    }

    #[test]
    fn empty_and_whitespace_levels_hold_zero_pairs() {
        for text in ["", "   ", "\n"] {
            let (items, errors) = parsed_pairs(text, &[]);
            assert_eq!(items.len(), 0, "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_trailing_comma_after_a_pair_is_not_a_parse_error() {
        let text = "id: 1,";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
            ],
        );
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.item,
            ArgumentNameWrapper("id".intern().to())
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn integer_overflow_is_a_typed_error_on_that_pair() {
        let text = "a: 99999999999999999999, b: 1";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "99999999999999999999"),
                (IsographSemanticToken::Argument, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
            ],
        );
        assert!(items[0].item.item.is_none());
        as_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == AstError::IntegerDoesNotFitI64
                && error.location == span_of(text, "99999999999999999999")
        }));
    }

    #[test]
    fn a_malformed_pair_degrades_that_pair_alone() {
        let text = "a 1, b: 2";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Argument, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "2"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Token(NonBracketTokenKind::Colon),
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
        }));
    }

    #[test]
    fn a_non_value_identifier_is_an_error_at_the_value() {
        let text = "a: yes";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "yes"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "yes")
        }));
    }

    #[test]
    fn a_pair_that_does_not_start_with_a_name_is_an_argument_error() {
        let text = "42: 1";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Integer, "42"),
                (IsographSemanticToken::Content, ":"),
                (IsographSemanticToken::Integer, "1"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Argument,
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
                && error.location == span_of(text, "42")
        }));
    }

    #[test]
    fn leftover_after_a_pair_keeps_the_item() {
        let text = "id: $x junk";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Content, "junk"),
            ],
        );
        assert_eq!(items.len(), 1);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "id")
        );
        assert!(items[0].item.extra.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Separator(BracketKind::Parenthesis),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_between_pairs_is_chunkings_error() {
        let text = "a: 1,, b: 2";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Parenthesis),
            parse_argument,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Argument, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "2"),
            ],
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(
            as_argument(items[0].item.reference()).name.location,
            span_of(text, "a")
        );
        assert_eq!(
            as_argument(items[1].item.reference()).name.location,
            span_of(text, "b")
        );
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn consume_argument_list_reads_a_paren_group() {
        let text = "(id: $petId)";
        let (list, errors) = parsed_argument_list(
            text,
            &[
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "petId"),
                (IsographSemanticToken::Parenthesis, ")"),
            ],
        );
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(errors, vec![]);
        assert_eq!(list.location, span_of(text, "(id: $petId)"));
        assert_eq!(list.item.0.len(), 1);
        assert_eq!(
            as_argument(list.item.0[0].item.reference()).name.item,
            ArgumentNameWrapper("id".intern().to())
        );
    }

    #[test]
    fn an_empty_paren_group_is_zero_pairs() {
        let text = "()";
        let (list, errors) = parsed_argument_list(
            text,
            &[
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Parenthesis, ")"),
            ],
        );
        let list = list.expect("the fixture opens with a paren group");
        assert_eq!(list.item.0.len(), 0);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_list_interior_holds_three_values() {
        let text = "1, $x, true";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_list_literal_value,
            &[
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::BooleanOrNull, "true"),
            ],
        );
        assert_eq!(comma_errors, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 3);
        assert!(matches!(
            as_list_value(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(1))
        ));
        assert!(matches!(
            as_list_value(items[1].item.reference()).value.item,
            NonConstantValue::Variable(_)
        ));
        assert!(matches!(
            as_list_value(items[2].item.reference()).value.item,
            NonConstantValue::Boolean(BooleanValue(Boolean::True))
        ));
    }

    #[test]
    fn a_list_value_parses_nested_lists_and_objects() {
        let text = "[[1], { a: 2 }]";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Value,
            |cursor| parse_non_constant_value(cursor).map(|wrapped| wrapped.item),
            &[
                (IsographSemanticToken::Bracket, "["),
                (IsographSemanticToken::Bracket, "["),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Bracket, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::ObjectKey, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "2"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Bracket, "]"),
            ],
        );
        assert_eq!(comma_errors, vec![]);
        assert_eq!(errors, vec![]);
        assert_eq!(items.len(), 1);
        let list = match items[0]
            .item
            .item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
        {
            Some(NonConstantValue::List(list)) => list,
            value => panic!("expected a list, got {value:?}"),
        };
        assert_eq!(list.0.len(), 2);
        assert!(matches!(
            as_list_value(list.0[0].item.reference()).value.item,
            NonConstantValue::List(_)
        ));
        assert!(matches!(
            as_list_value(list.0[1].item.reference()).value.item,
            NonConstantValue::Object(_)
        ));
    }

    #[test]
    fn empty_and_whitespace_list_interiors_are_empty() {
        for text in ["[]", "[ ]", "[\n]"] {
            let (items, errors, comma_errors) = parsed_items(
                text,
                Expectation::Value,
                |cursor| parse_non_constant_value(cursor).map(|wrapped| wrapped.item),
                &[
                    (IsographSemanticToken::Bracket, "["),
                    (IsographSemanticToken::Bracket, "]"),
                ],
            );
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            assert_eq!(errors, vec![], "for literal {text:?}");
            match items[0]
                .item
                .item
                .as_ref()
                .map(|wrapped| wrapped.item.reference())
            {
                Some(NonConstantValue::List(list)) => {
                    assert_eq!(list.0.len(), 0, "for literal {text:?}");
                }
                value => panic!("expected an empty list, got {value:?}"),
            }
        }
    }

    #[test]
    fn a_trailing_comma_in_a_list_is_not_a_parse_error() {
        let text = "1,";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_list_literal_value,
            &[(IsographSemanticToken::Integer, "1")],
        );
        assert_eq!(comma_errors, vec![]);
        assert_eq!(items.len(), 1);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn leftover_after_a_list_value_keeps_the_item() {
        let text = "1 junk";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_list_literal_value,
            &[
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Content, "junk"),
            ],
        );
        assert_eq!(comma_errors, vec![]);
        assert_eq!(items.len(), 1);
        assert!(matches!(
            as_list_value(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(1))
        ));
        assert!(items[0].item.extra.is_some());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Separator(BracketKind::Bracket),
                    Found::Token(NonBracketTokenKind::Identifier),
                )
                && error.location == span_of(text, "junk")
        }));
    }

    #[test]
    fn a_doubled_comma_in_a_list_is_chunkings_error() {
        let text = "1,, 2";
        let (items, errors, comma_errors) = parsed_items(
            text,
            Expectation::Separator(BracketKind::Bracket),
            parse_list_literal_value,
            &[
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Integer, "2"),
            ],
        );
        assert_eq!(comma_errors.len(), 1);
        assert_eq!(items.len(), 2);
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn a_list_parses_as_an_argument_value() {
        let text = "id: [1, 2]";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Bracket, "["),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Integer, "2"),
                (IsographSemanticToken::Bracket, "]"),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::List(list) => {
                assert_eq!(list.0.len(), 2);
            }
            value => panic!("expected a list argument, got {value:?}"),
        }
    }

    #[test]
    fn integer_underflow_is_a_typed_error_on_that_pair() {
        let text = "a: -99999999999999999999, b: 1";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "-99999999999999999999"),
                (IsographSemanticToken::Argument, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
            ],
        );
        assert!(items[0].item.item.is_none());
        as_argument(items[1].item.reference());
        assert!(errors.iter().any(|error| {
            error.item == AstError::IntegerDoesNotFitI64
                && error.location == span_of(text, "-99999999999999999999")
        }));
    }

    #[test]
    fn zero_parses_as_an_integer() {
        let text = "a: 0";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "0"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(0))
        ));
    }

    #[test]
    fn i64_min_parses() {
        let text = "a: -9223372036854775808";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "-9223372036854775808"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(i64::MIN))
        ));
    }

    #[test]
    fn a_leading_zero_integer_is_not_a_value() {
        let text = "a: 01";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Content, "01"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::ErrorNumberLiteralLeadingZero),
                )
                && error.location == span_of(text, "01")
        }));
    }

    #[test]
    fn a_float_is_not_a_value() {
        let text = "a: 1.5";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Content, "1.5"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Value,
                    Found::Token(NonBracketTokenKind::FloatLiteral),
                )
                && error.location == span_of(text, "1.5")
        }));
    }

    #[test]
    fn an_empty_object_is_a_value() {
        let text = "input: {}";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "input"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::Object(object) => assert_eq!(object.0.len(), 0),
            value => panic!("expected an object, got {value:?}"),
        }
    }

    #[test]
    fn a_quoted_string_value_drops_the_quotes() {
        let text = r#"a: "hi""#;
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"hi\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper("hi".intern().to()));
            }
            value => panic!("expected a string, got {value:?}"),
        }
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, r#""hi""#),
        );
    }

    #[test]
    fn a_quoted_string_does_not_process_escapes() {
        let text = r#"a: "hi\n""#;
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"hi\\n\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper(r#"hi\n"#.intern().to()));
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }

    #[test]
    fn a_quoted_string_keeps_inner_quote_characters() {
        let text = r#"a: "\"hi\"""#;
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"\\\"hi\\\"\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper(r#"\"hi\""#.intern().to()),);
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }

    #[test]
    fn a_block_string_is_a_value() {
        let text = "a: \"\"\"hi\"\"\"";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"\"\"hi\"\"\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper("hi".intern().to()));
            }
            value => panic!("expected a string, got {value:?}"),
        }
        assert_eq!(
            as_argument(items[0].item.reference()).value.location,
            span_of(text, "\"\"\"hi\"\"\"")
        );
    }

    #[test]
    fn a_one_line_block_string_keeps_leading_spaces() {
        let text = r#"a: """   hi""""#;
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"\"\"   hi\"\"\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper("   hi".intern().to()));
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }

    #[test]
    fn i64_max_parses() {
        let text = "a: 9223372036854775807";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "9223372036854775807"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(i64::MAX))
        ));
    }

    #[test]
    fn negative_zero_parses_as_zero() {
        let text = "a: -0";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "-0"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::Integer(IntegerValue(0))
        ));
    }

    #[test]
    fn an_empty_string_is_a_value() {
        let text = "a: \"\"";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"\""),
            ],
        );
        assert_eq!(errors, vec![]);
        assert!(matches!(
            as_argument(items[0].item.reference()).value.item,
            NonConstantValue::String(_)
        ));
    }

    #[test]
    fn an_empty_block_string_is_a_value() {
        let text = "a: \"\"\"\"\"\"";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::String, "\"\"\"\"\"\""),
            ],
        );
        assert_eq!(errors, vec![]);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::String(value) => {
                assert_eq!(*value, StringLiteralValueWrapper("".intern().to()));
            }
            value => panic!("expected a string, got {value:?}"),
        }
    }

    #[test]
    fn a_colon_without_a_value_fails_that_pair() {
        let text = "a:";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
            ],
        );
        assert!(items[0].item.item.is_none());
        let colon_end = span_of(text, ":").end;
        assert!(errors.iter().any(|error| {
            error.item == AstError::expected(Expectation::Value, Found::EndOfChunk)
                && error.location == Span::new(colon_end, colon_end)
        }));
    }

    #[test]
    fn a_dollar_without_a_name_is_not_a_value() {
        let text = "a: $";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
            ],
        );
        assert!(items[0].item.item.is_none());
        let dollar_end = span_of(text, "$").end;
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::Token(NonBracketTokenKind::Identifier),
                    Found::EndOfChunk,
                )
                && error.location == Span::new(dollar_end, dollar_end)
        }));
    }

    #[test]
    fn a_paren_group_is_not_a_value() {
        let text = "a: (x)";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Bracket, "("),
                (IsographSemanticToken::Content, "x"),
                (IsographSemanticToken::Bracket, ")"),
            ],
        );
        assert!(items[0].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(Expectation::Value, Found::Group(BracketKind::Parenthesis))
                && error.location == span_of(text, "(x)")
        }));
    }

    #[test]
    fn a_non_integer_number_is_not_a_value() {
        for (text, found, pattern) in [
            (
                "a: .5",
                Found::Token(NonBracketTokenKind::ErrorFloatLiteralMissingZero),
                ".5",
            ),
            (
                "a: 1.",
                Found::Token(NonBracketTokenKind::ErrorNumberLiteralTrailingInvalid),
                "1.",
            ),
            (
                "a: 1e2",
                Found::Token(NonBracketTokenKind::FloatLiteral),
                "1e2",
            ),
        ] {
            let (items, errors) = parsed_pairs(
                text,
                &[
                    (IsographSemanticToken::Argument, "a"),
                    (IsographSemanticToken::Colon, ":"),
                    (IsographSemanticToken::Content, pattern),
                ],
            );
            assert!(items[0].item.item.is_none(), "for literal {text:?}");
            assert!(
                errors.iter().any(|error| {
                    error.item == AstError::expected(Expectation::Value, found)
                        && error.location == span_of(text, pattern)
                }),
                "for literal {text:?}, errors were {errors:?}",
            );
        }
    }

    #[test]
    fn an_object_entry_that_does_not_start_with_a_name_fails_that_entry() {
        let text = "input: { 1: 2 }";
        let (items, errors) = parsed_pairs(
            text,
            &[
                (IsographSemanticToken::Argument, "input"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Content, ":"),
                (IsographSemanticToken::Integer, "2"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors.len(), 1);
        match as_argument(items[0].item.reference())
            .value
            .item
            .reference()
        {
            NonConstantValue::Object(object) => {
                assert!(object.0[0].item.item.is_none());
            }
            value => panic!("expected an object, got {value:?}"),
        }
        assert!(errors.iter().any(|error| {
            error.item
                == AstError::expected(
                    Expectation::ObjectEntry,
                    Found::Token(NonBracketTokenKind::IntegerLiteral),
                )
                && error.location == span_of(text, "1")
        }));
    }
}
