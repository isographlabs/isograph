use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithGenericLocation, WithOptionalSpan, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, BracketKind, ChunkContentItem, ChunkedLevel, EntityNameWrapper, Expectation, Found,
    IsographResolutionNode, IsographSemanticToken, NonBracketToken, NonBracketTokenKind,
    NonConstantValue, SelectableDeclarationPath, Slot, UnparsedChunkItems,
    VariableDeclarationOrUsage, parse_name_colon, parse_non_constant_value, parse_variable_name,
};

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct VariableDeclarationList(
    #[resolve_field] pub Vec<WithSpan<Slot<VariableDeclaration, UnparsedChunkItems>>>,
);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
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

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    Union(UnionTypeAnnotation),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct UnionTypeAnnotation(#[resolve_field] pub Vec<WithOptionalSpan<UnionVariant>>);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = UnionTypeAnnotationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum UnionVariant {
    Named(#[parent_variant(Union)] NamedTypeAnnotation),
    List(#[parent_variant(Union)] Box<ListTypeAnnotation>),
    Null(NullTypeAnnotation),
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = UnionTypeAnnotationPath<'a>,
    resolved_node = IsographResolutionNode<'a>,
    on_unmatched_span = from_path
)]
pub struct NullTypeAnnotation;

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct NamedTypeAnnotation {
    #[resolve_field]
    #[parent_variant(NamedTypeAnnotation)]
    pub name: WithSpan<EntityNameWrapper>,
}

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = TypeAnnotationParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ListTypeAnnotation {
    #[resolve_field]
    #[parent_variant(List)]
    pub inner: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    #[parent_from]
    pub extra: Option<WithSpan<UnparsedChunkItems>>,
}

#[derive(Debug)]
pub enum TypeAnnotationParent<'a> {
    Variable(VariableDeclarationPath<'a>),
    List(Box<ListTypeAnnotationPath<'a>>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Union(Box<UnionTypeAnnotationPath<'a>>),
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

pub type UnionTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a UnionTypeAnnotation, TypeAnnotationParent<'a>>;

pub type NullTypeAnnotationPath<'a> =
    PositionResolutionPath<&'a NullTypeAnnotation, UnionTypeAnnotationPath<'a>>;

impl<'a> From<VariableDeclarationSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: VariableDeclarationSlotPath<'a>) -> Self {
        IsographResolutionNode::VariableDeclarationSlot(path)
    }
}

impl<'a> From<NullTypeAnnotationPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: NullTypeAnnotationPath<'a>) -> Self {
        IsographResolutionNode::UnionTypeAnnotation(path.parent)
    }
}

pub(crate) fn consume_variable_declaration_list(
    cursor: &mut ItemCursor<'_>,
) -> Option<WithSpan<VariableDeclarationList>> {
    cursor.consume_group_if(
        BracketKind::Parenthesis,
        IsographSemanticToken::Parenthesis,
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
        match cursor.consume_token_if(NonBracketTokenKind::Equals, IsographSemanticToken::Equals) {
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

enum NamedOrList {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

impl NamedOrList {
    fn into_type_annotation(self) -> TypeAnnotation {
        match self {
            NamedOrList::Named(named) => TypeAnnotation::Named(named),
            NamedOrList::List(list) => TypeAnnotation::List(list),
        }
    }

    fn into_union_variant(self) -> UnionVariant {
        match self {
            NamedOrList::Named(named) => UnionVariant::Named(named),
            NamedOrList::List(list) => UnionVariant::List(list),
        }
    }
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
        return core
            .item
            .into_type_annotation()
            .with_span(core.location)
            .wrap_ok();
    }
    let location = core.location;
    let written = WithGenericLocation::new(core.item.into_union_variant(), location.wrap_some());
    let null = WithGenericLocation::new(UnionVariant::Null(NullTypeAnnotation), None);
    TypeAnnotation::Union(UnionTypeAnnotation(vec![written, null]))
        .with_span(location)
        .wrap_ok()
}

fn parse_named_or_list(
    cursor: &mut ItemCursor<'_>,
) -> Result<WithSpan<NamedOrList>, WithSpan<AstError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::GraphQLTypeName,
        ) {
            return NamedOrList::Named(NamedTypeAnnotation {
                name: name.interned().map(EntityNameWrapper),
            })
            .wrap_ok();
        }
        if let Some(parsed) = cursor.consume_group_if(
            BracketKind::Bracket,
            IsographSemanticToken::GraphQLTypeName,
            |cursor, children| parse_bracket_interior_type(cursor, children),
        ) {
            let parsed = parsed.item?;
            return NamedOrList::List(
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
    let slot = crate::parse_one_chunk(
        &level.item.0[0],
        cursor.stream_chunk(&level.item.0[0].item),
        Expectation::EndOfType,
        parse_type_annotation,
    );
    if let Some(comma) = level.item.0[0].item.boundary_comma() {
        cursor.report_error(
            AstError::expected(
                Expectation::EndOfType,
                Found::Token(NonBracketTokenKind::Comma),
            )
            .with_span(comma),
        );
    }
    if let Some(extra) = level.item.0.get(1) {
        cursor.report_error(
            AstError::expected(
                Expectation::EndOfType,
                Found::from(extra.item.first_item().item.reference()),
            )
            .with_span(extra.location),
        );
        for chunk in level.item.0[1..].iter() {
            cursor.record_leftover_chunk(chunk.item.reference());
        }
    }
    BracketInteriorType {
        item: slot.item.item.map(|wrapped| wrapped.item),
        extra: slot.item.extra,
    }
    .wrap_ok()
}
