use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, BracketKind, ChunkContentItem, ChunkedLevel, EntityNameWrapper, Expectation, Found,
    IsographResolutionNode, NonBracketToken, NonBracketTokenKind, NonConstantValue,
    SelectableDeclarationPath, SemanticToken, Slot, UnparsedChunkItems, VariableDeclarationOrUsage,
    parse_name_colon, parse_non_constant_value, parse_variable_name,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclaration, UnparsedChunkItems>>>,
);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = VariableDeclarationSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclaration {
    #[resolve_field]
    #[parent_variant(Declaration)]
    pub name: WithSpan<VariableDeclarationOrUsage>,
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
    Null(Box<NullTypeAnnotation>),
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
    pub extra: Option<WithSpan<UnparsedChunkItems>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NullTypeAnnotation(
    #[resolve_field]
    #[parent_variant(Null)]
    pub WithSpan<TypeAnnotation>,
);

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Null(Box<NullTypeAnnotationPath<'a>>),
}

pub type VariableDeclarationListPath<'a> =
    PositionResolutionPath<&'a VariableDeclarationList, SelectableDeclarationPath<'a>>;

pub type VariableDeclarationSlotPath<'a> = PositionResolutionPath<
    &'a Slot<VariableDeclaration, UnparsedChunkItems>,
    VariableDeclarationListPath<'a>,
>;

pub type VariableDeclarationPath<'a> =
    PositionResolutionPath<&'a VariableDeclaration, VariableDeclarationSlotPath<'a>>;

pub type NamedTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NamedTypeAnnotation, TypeAnnotationParent<'a>>;

pub type ListTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a ListTypeAnnotation, TypeAnnotationParent<'a>>;

pub type NullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NullTypeAnnotation, TypeAnnotationParent<'a>>;

impl<'a> From<VariableDeclarationSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationSlot(path)
    }
}

pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        SemanticToken::Parenthesis,
        |cursor, children| {
            VariableDeclarationList(children.item.parse_each_chunk(
                cursor,
                Expectation::Separator(BracketKind::Parenthesis),
                parse_variable_declaration,
            ))
        },
    )
}

fn parse_variable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<VariableDeclaration, WithSpan<AstError>> {
    let (name, type_) = parse_name_colon(
        cursor,
        |cursor| parse_variable_name(cursor, Expectation::VariableDeclaration),
        parse_type_annotation,
    )?;
    let default_value =
        match cursor.consume_token_if(NonBracketTokenKind::Equals, SemanticToken::Equals) {
            Some(_) => parse_non_constant_value(cursor)?.wrap_some(),
            None => None,
        };
    VariableDeclaration {
        name,
        type_,
        default_value,
    }
    .wrap_ok()
}

pub(crate) fn parse_type_annotation(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<AstError>> {
    let core = parse_named_or_list(cursor)?;
    if let Some(peek) = cursor.peek()
        && matches!(
            peek.view().item.reference(),
            ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Exclamation))
        )
    {
        peek.advance();
        return core.wrap_ok();
    }
    let location = core.location;
    TypeAnnotation::Null(NullTypeAnnotation(core).boxed())
        .with_span(location)
        .wrap_ok()
}

fn parse_named_or_list(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<AstError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            SemanticToken::GraphQLTypeName,
        ) {
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
            return TypeAnnotation::List(
                ListTypeAnnotation {
                    inner: parsed.item,
                    extra: parsed.extra,
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
    extra: Option<WithSpan<UnparsedChunkItems>>,
}

fn parse_bracket_interior_type(
    cursor: &mut ItemCursor<'_>,
    level: &WithSpan<ChunkedLevel>,
) -> Result<BracketInteriorType, WithSpan<AstError>> {
    if level.item.len() == 0 {
        return AstError::expected(Expectation::TypeAnnotation, Found::EndOfChunk)
            .with_span(Span::new(level.location.end, level.location.end))
            .wrap_err();
    }
    let singleton = cursor.parse_nested_singleton(
        level,
        Expectation::EndOfType,
        |extra| {
            AstError::expected(
                Expectation::EndOfType,
                Found::from(extra.item.first_item().item.reference()),
            )
            .with_span(extra.location)
        },
        parse_type_annotation,
    );
    BracketInteriorType {
        item: singleton.item.item.item.map(|wrapped| wrapped.item),
        extra: singleton.item.item.extra,
    }
    .wrap_ok()
}
