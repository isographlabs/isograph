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

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsographFieldDirectiveListParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct IsographFieldDirectiveList(#[resolve_field] pub Vec<WithSpan<IsographFieldDirective>>);

#[derive(Clone, Debug, PartialEq, Eq, ResolvePosition)]
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

/// A directive name the iso parser accepts on a selection.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
pub(crate) enum SelectionDirectiveName {
    #[strum(serialize = "loadable")]
    Loadable,
}

/// A directive name the iso parser accepts on an `entrypoint` or `field` declaration. v1
/// accepts none.
#[derive(Copy, Clone, Debug, PartialEq, Eq, strum::EnumString)]
enum DeclarationDirectiveName {}

/// Where a directive list sits, which decides the names it accepts.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub(crate) enum DirectivePosition {
    Declaration,
    Selection,
}

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
    position: DirectivePosition,
) -> Result<Option<WithSpan<IsographFieldDirectiveList>>, WithSpan<AstError>> {
    if !next_is_at(cursor) {
        return None.wrap_ok();
    }
    cursor
        .spanning(|cursor| {
            let mut directives = Vec::new();
            while let Some(at) =
                cursor.consume_token_if(NonBracketTokenKind::At, IsographSemanticToken::At)
            {
                directives.push(parse_directive_after_at(cursor, at.location, position)?);
            }
            IsographFieldDirectiveList(directives).wrap_ok()
        })
        .map(|list| list.wrap_some())
}

fn parse_directive_after_at(
    cursor: &mut ItemCursor<'_>,
    at: Span,
    position: DirectivePosition,
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
    let location = Span::new(at.start, end);
    match position {
        DirectivePosition::Declaration => match name.text().parse::<DeclarationDirectiveName>() {
            Ok(supported) => match supported {},
            Err(strum::ParseError::VariantNotFound) => {
                cursor.report_error(AstError::UnexpectedDirective.with_span(location));
            }
        },
        DirectivePosition::Selection => match name.text().parse::<SelectionDirectiveName>() {
            Ok(SelectionDirectiveName::Loadable) => {
                if let Some(arguments) = arguments.as_ref() {
                    cursor.report_error(AstError::LoadableArguments.with_span(arguments.location));
                }
            }
            Err(strum::ParseError::VariantNotFound) => {
                cursor.report_error(AstError::UnexpectedDirective.with_span(location));
            }
        },
    }
    IsographFieldDirective {
        name: name.interned().map(IsographDirectiveNameWrapper),
        arguments,
    }
    .with_span(location)
    .wrap_ok()
}
