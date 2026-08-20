use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    BracketKind, ChunkedLevel, EntityNameWrapper, Expectation, FieldDeclarationPath, Found,
    IsographResolutionNode, NonBracketTokenKind, NonConstantValue, ParseError, SemanticToken, Slot,
    UnparsedChunkItems, VariableNameWrapper, parse_name_colon, parse_non_constant_value,
    parse_variable_name,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = FieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsageList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclarationOrUsage, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationOrUsageSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationOrUsage {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableNameWrapper>,
    #[resolve_field]
    #[parent_variant(Variable)]
    pub type_: WithSpan<TypeAnnotation>,
    #[resolve_field]
    #[parent_variant(VariableDefault)]
    pub default_value: Option<WithSpan<NonConstantValue>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedTypeAnnotation {
    #[resolve_field]
    #[parent_variant(NamedTypeAnnotation)]
    pub name: WithSpan<EntityNameWrapper>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    #[resolve_field]
    #[parent_variant(List)]
    pub inner: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    #[parent_from]
    pub extra_tokens: Option<WithSpan<UnparsedChunkItems>>,
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationOrUsagePath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
}

pub type VariableDeclarationOrUsageListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsageList, FieldDeclarationPath<'a>>;

pub type VariableDeclarationOrUsageSlotPath<'a> = PositionResolutionPath<
    &'a Slot<VariableDeclarationOrUsage, UnparsedChunkItems>,
    VariableDeclarationOrUsageListPath<'a>,
>;

pub type VariableDeclarationOrUsagePath<'a> =
    PositionResolutionPath<&'a VariableDeclarationOrUsage, VariableDeclarationOrUsageSlotPath<'a>>;

pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;

pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;

impl<'a> From<VariableDeclarationOrUsageSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationOrUsageSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationOrUsageSlot(path)
    }
}

pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationOrUsageList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            VariableDeclarationOrUsageList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_variable_declaration,
            ))
        },
    )
}

fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<VariableDeclarationOrUsage, WithSpan<ParseError>> {
    let (name, type_) = parse_name_colon(
        cursor,
        |cursor| parse_variable_name(cursor, Expectation::VariableDeclarationOrUsage),
        parse_type_annotation,
    )?;
    let default_value =
        match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals) {
            Some(_) => parse_non_constant_value(cursor)?.wrap_some(),
            None => None,
        };
    VariableDeclarationOrUsage {
        name,
        type_,
        default_value,
    }
    .wrap_ok()
}

pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::Named(NamedTypeAnnotation {
                name: name.interned().map(EntityNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(parsed) = cursor.consume_group_if(
            BracketKind::Bracket,
            SemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let parsed = parsed.item?;
            cursor.consume_token_if(
                NonBracketTokenKind::Exclamation,
                SemanticToken::GraphQLTypeName,
            );
            return TypeAnnotation::List(
                ListTypeAnnotation {
                    inner: parsed.item,
                    extra_tokens: parsed.extra_tokens,
                }
                .boxed(),
            )
            .wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}

struct BracketInteriorType {
    item: Option<WithSpan<TypeAnnotation>>,
    extra_tokens: Option<WithSpan<UnparsedChunkItems>>,
}

fn parse_bracket_interior_type(
    cursor: &mut ItemCursor<'_>,
    level: &WithSpan<ChunkedLevel>,
) -> Result<BracketInteriorType, WithSpan<ParseError>> {
    if level.item.len() == 0 {
        return ParseError::expected(Expectation::TypeAnnotation, Found::EndOfChunk)
            .with_span(Span::new(level.location.end, level.location.end))
            .wrap_err();
    }
    let singleton = cursor.parse_nested_singleton(
        level,
        Expectation::EndOfType,
        |extra| {
            ParseError::expected(
                Expectation::EndOfType,
                Found::from(extra.item.first_item().item.reference()),
            )
            .with_span(extra.location)
        },
        |cursor| parse_type_annotation(cursor).map(|wrapped| wrapped.item),
    );
    BracketInteriorType {
        item: singleton.item.item.item,
        extra_tokens: singleton.item.item.extra_tokens,
    }
    .wrap_ok()
}
