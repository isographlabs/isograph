use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ArgumentList, AstError, ChunkContentItem, EntrypointDeclarationPath, Expectation,
    IsographResolutionNode, IsographSemanticToken, NonBracketToken, NonBracketTokenKind,
    SelectableDeclarationPath, SelectionPath, consume_argument_list,
};

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectiveListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographFieldDirectiveList(#[resolve_field] pub Vec<WithSpan<IsographFieldDirective>>);

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectiveListPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographFieldDirective {
    #[resolve_field]
    pub name: WithSpan<IsographDirectiveNameWrapper>,
    #[resolve_field]
    #[parent_variant(IsographFieldDirective)]
    pub arguments: Option<WithSpan<ArgumentList>>,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectivePath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographDirectiveNameWrapper(pub common_lang_types::IsographDirectiveName);

#[derive(Debug)]
pub enum IsographFieldDirectiveListParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    Selection(SelectionPath<'a>),
}

pub type IsographFieldDirectiveListPath<'a> =
    PositionResolutionPath<&'a IsographFieldDirectiveList, IsographFieldDirectiveListParent<'a>>;

pub type IsographFieldDirectivePath<'a> =
    PositionResolutionPath<&'a IsographFieldDirective, IsographFieldDirectiveListPath<'a>>;

pub type IsographDirectiveNameWrapperPath<'a> =
    PositionResolutionPath<&'a IsographDirectiveNameWrapper, IsographFieldDirectivePath<'a>>;

fn next_is_at(cursor: &mut ItemCursor<'_>) -> bool {
    matches!(
        cursor.peek().map(|peek| peek.view().item.reference()),
        Some(ChunkContentItem::NonBracket(NonBracketToken(
            NonBracketTokenKind::At
        )))
    )
}

pub(crate) fn consume_directives(
    cursor: &mut ItemCursor<'_>,
) -> Result<Option<WithSpan<IsographFieldDirectiveList>>, WithSpan<AstError>> {
    if !next_is_at(cursor) {
        return None.wrap_ok();
    }
    cursor
        .spanning(|cursor| {
            let mut directives = Vec::new();
            while let Some(at) = cursor.consume_token_if(
                NonBracketTokenKind::At,
                IsographSemanticToken::DirectiveName,
            ) {
                directives.push(parse_directive_after_at(cursor, at.location)?);
            }
            IsographFieldDirectiveList(directives).wrap_ok()
        })
        .map(|list| list.wrap_some())
}

fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
) -> Result<WithSpan<IsographFieldDirective>, WithSpan<AstError>> {
    let name = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::DirectiveName,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let arguments = consume_argument_list(cursor);
    let end = arguments
        .as_ref()
        .map(|list| list.location.end)
        .unwrap_or(name.location.end);
    IsographFieldDirective {
        name: name.interned().map(IsographDirectiveNameWrapper),
        arguments,
    }
    .with_span(Span::new(at.start, end))
    .wrap_ok()
}
