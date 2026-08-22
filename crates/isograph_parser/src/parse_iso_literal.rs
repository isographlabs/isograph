use std::fmt;

use common_lang_types::SelectableName;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    AstError, BracketError, ChunkContentItem, ChunkedLevel, DECLARATION_KEYWORD, Expectation,
    ExtraChunks, Found, IsographFieldDirectiveList, IsographResolutionNode, IsographSemanticToken,
    NamedTypeAnnotationPath, NonBracketToken, NonBracketTokenKind, ParseError, SelectionSet,
    Singleton, Slot, TypeAnnotation, UnparsedChunkItems, VariableDeclarationList, chunk,
    consume_directives, consume_selection_set, consume_variable_declaration_list,
    intern_block_string_value, match_brackets, parse_singleton, parse_type_annotation, tokenize,
};

pub type IsoLiteralParse = Singleton<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;

pub type IsoLiteralSlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;

impl<'a> From<IsoLiteralSlotPath<'a>> for IsographResolutionNode<'a> {
    fn from(path: IsoLiteralSlotPath<'a>) -> Self {
        IsographResolutionNode::IsoLiteralSlot(path)
    }
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
    Selectable(SelectableDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub name: WithSpan<SelectableNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct SelectableDeclaration {
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub name: WithSpan<SelectableNameWrapper>,
    #[resolve_field]
    pub variable_definitions: Option<WithSpan<VariableDeclarationList>>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub target_type: Option<WithSpan<TypeAnnotation>>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub directive_set: Option<WithSpan<IsographFieldDirectiveList>>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(SelectableDeclaration)]
    pub selection_set: Option<WithSpan<SelectionSet>>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntityNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityNameWrapper(pub common_lang_types::EntityName);

/// The name of an entrypoint or selectable, `foo` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = SelectableNameWrapperParent<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct SelectableNameWrapper(pub common_lang_types::SelectableName);

impl fmt::Display for SelectableNameWrapper {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(f)
    }
}

/// The interned interior of a description, quotes excluded.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SelectableDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(pub common_lang_types::DescriptionValue);

#[derive(Debug)]
pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
    NamedTypeAnnotation(NamedTypeAnnotationPath<'a>),
}

#[derive(Debug)]
pub enum SelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    SelectableDeclaration(SelectableDeclarationPath<'a>),
}

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralSlotPath<'a>>;

pub type SelectableDeclarationPath<'a> =
    PositionResolutionPath<&'a SelectableDeclaration, IsoLiteralSlotPath<'a>>;

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, SelectableDeclarationPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

pub type EntityNameWrapperPath<'a> =
    PositionResolutionPath<&'a EntityNameWrapper, EntityNameWrapperParent<'a>>;

pub type SelectableNameWrapperPath<'a> =
    PositionResolutionPath<&'a SelectableNameWrapper, SelectableNameWrapperParent<'a>>;

#[derive(Debug)]
pub struct ParsedIsoLiteral {
    pub item: Option<WithSpan<IsoLiteralParse>>,
    pub errors: Vec<WithSpan<ParseError>>,
    pub tokens: Vec<WithSpan<IsographSemanticToken>>,
}

pub fn parse_iso_literal(text: &str) -> ParsedIsoLiteral {
    let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
    let (tree, comma_errors) = chunk(brackets.reference());
    let mut errors: Vec<WithSpan<ParseError>> = bracket_errors
        .into_iter()
        .map(|error| {
            let location = match error.reference() {
                BracketError::UnmatchedOpen(open) => open.location,
                BracketError::UnmatchedClose(close) => close.location,
            };
            error.to::<ParseError>().with_span(location)
        })
        .collect();
    errors.extend(
        comma_errors
            .into_iter()
            .map(|error| error.to::<ParseError>().with_span(error.0)),
    );
    let mut ast_errors = Vec::new();
    let mut tokens = Vec::new();
    let item = parse_chunked_iso_literal(text, tree, &mut ast_errors, &mut tokens);
    errors.extend(
        ast_errors
            .into_iter()
            .map(|error| error.item.to::<ParseError>().with_span(error.location)),
    );
    ParsedIsoLiteral {
        item,
        errors,
        tokens,
    }
}

pub(crate) fn parse_chunked_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<AstError>>,
    tokens: &mut Vec<WithSpan<IsographSemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        errors.push(AstError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        errors,
        Expectation::EndOfDeclaration,
        |extra| AstError::MultipleDeclarations.with_span(extra.location),
        parse_iso_literal_item,
    );
    singleton.with_span(location).wrap_some()
}

fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<AstError>> {
    let keyword = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::Keyword,
        )
        .map_err(|()| cursor.expected(DECLARATION_KEYWORD))?;
    match keyword.text() {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Selectable(parse_selectable_declaration(cursor)?).wrap_ok(),
        _ => AstError::expected(
            DECLARATION_KEYWORD,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
    }
}

fn parse_type_dot_name(
    cursor: &mut ItemCursor<'_>,
) -> Result<(WithSpan<EntityNameWrapper>, WithSpan<SelectableName>), WithSpan<AstError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, IsographSemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, IsographSemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let name = cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::FieldName,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    (
        parent_type.interned().map(EntityNameWrapper),
        name.interned(),
    )
        .wrap_ok()
}

fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<AstError>> {
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    let directive_set = consume_directives(cursor)?;
    EntrypointDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        directive_set,
    }
    .wrap_ok()
}

fn parse_selectable_declaration(
    cursor: &mut ItemCursor<'_>,
) -> Result<SelectableDeclaration, WithSpan<AstError>> {
    let (parent_type, name) = parse_type_dot_name(cursor)?;
    let variable_definitions = consume_variable_declaration_list(cursor);
    let target_type = consume_to_target(cursor)?;
    let directive_set = consume_directives(cursor)?;
    let description = consume_description(cursor);
    let selection_set = consume_selection_set(cursor);
    SelectableDeclaration {
        parent_type,
        name: name.map(SelectableNameWrapper),
        variable_definitions,
        target_type,
        directive_set,
        description,
        selection_set,
    }
    .wrap_ok()
}

fn consume_to_target(
    cursor: &mut ItemCursor<'_>,
) -> Result<Option<WithSpan<TypeAnnotation>>, WithSpan<AstError>> {
    let peek = cursor.peek();
    let Some(peek) = peek else {
        return None.wrap_ok();
    };
    let item = peek.view();
    let is_identifier = matches!(
        item.item.reference(),
        ChunkContentItem::NonBracket(NonBracketToken(NonBracketTokenKind::Identifier))
    );
    let location = item.location;
    if !is_identifier || &cursor.text()[location.as_usize_range()] != "to" {
        return None.wrap_ok();
    }
    cursor
        .require_token(
            NonBracketTokenKind::Identifier,
            IsographSemanticToken::Keyword,
        )
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let target_type = parse_type_annotation(cursor)?;
    target_type.wrap_some().wrap_ok()
}

pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    if let Some(span) = cursor.consume_token_if(
        NonBracketTokenKind::StringLiteral,
        IsographSemanticToken::String,
    ) {
        return Description(span.exclude_ends(1).interned().item)
            .with_span(span.location)
            .wrap_some();
    }
    cursor
        .consume_token_if(
            NonBracketTokenKind::BlockStringLiteral,
            IsographSemanticToken::String,
        )
        .map(|span| {
            Description(intern_block_string_value(span.exclude_ends(3).text()))
                .with_span(span.location)
        })
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use resolve_position::ResolvePosition;
    use span::{Span, WithGenericLocation, WithOptionalSpan, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        ArgumentListParent, AstError, BracketError, BracketKind, ChunkContentItemParent,
        DECLARATION_KEYWORD, Expectation, Found, IntegerValue, IsographDirectiveNameWrapper,
        IsographFieldDirectiveListParent, IsographResolutionNode, NamedTypeAnnotation,
        NonBracketTokenKind, NonConstantValue, NonConstantValueParent, NullTypeAnnotation,
        ObjectEntry, ParseError, Selection, SelectionNameWrapper, SelectionSet, SelectionSetParent,
        Slot, TypeAnnotation, TypeAnnotationParent, UnionTypeAnnotation, UnionVariant,
        UnparsedChunkItems, UnparsedChunkItemsParent, VariableDeclaration, VariableDeclarationList,
        VariableDeclarationOrUsageParent, VariableNameWrapper,
        assert_semantic_tokens::assert_semantic_tokens, chunk, match_brackets,
        parsed_items::span_of, tokenize,
    };
    use Expectation::EndOfDeclaration;
    use NonBracketTokenKind::{
        At, Comma, Dollar, ErrorNumberLiteralTrailingInvalid, Identifier, Period,
    };

    fn parsed(
        text: &str,
        expected_tokens: &[(IsographSemanticToken, &str)],
    ) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<AstError>>) {
        let parsed = parse_iso_literal(text);
        assert_semantic_tokens(text, &parsed.tokens, expected_tokens);
        let errors = parsed
            .errors
            .into_iter()
            .map(|error| match error.item {
                ParseError::Ast(ast) => ast.with_span(error.location),
                ParseError::Bracket(_) | ParseError::Comma(_) => {
                    panic!("for literal {text:?}")
                }
            })
            .collect();
        (
            parsed.item.expect("the fixture is not an empty literal"),
            errors,
        )
    }

    fn parsed_with_errors(
        text: &str,
        expected_tokens: &[(IsographSemanticToken, &str)],
    ) -> ParsedIsoLiteral {
        let parsed = parse_iso_literal(text);
        assert_semantic_tokens(text, &parsed.tokens, expected_tokens);
        parsed
    }

    fn expected(expectation: Expectation, found: Found) -> AstError {
        AstError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
    }

    fn chunked(text: &str) -> WithSpan<ChunkedLevel> {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        let (tree, comma_errors) = chunk(brackets.reference());
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        tree
    }

    fn stream_of<'a>(
        tree: &'a WithSpan<ChunkedLevel>,
        text: &'a str,
        tokens: &'a mut Vec<WithSpan<IsographSemanticToken>>,
        errors: &'a mut Vec<WithSpan<AstError>>,
    ) -> crate::chunk_stream::ChunkStream<'a> {
        tree.item.0[0].item.stream(text, tokens, errors)
    }

    fn first_slot(parse: &WithSpan<IsoLiteralParse>) -> &Slot<IsoLiteralItem, UnparsedChunkItems> {
        parse.item.item.item.reference()
    }

    fn parsed_item(parse: &WithSpan<IsoLiteralParse>) -> Option<&IsoLiteralItem> {
        first_slot(parse)
            .item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
    }

    fn as_entrypoint(parse: &WithSpan<IsoLiteralParse>) -> &EntrypointDeclaration {
        let item = parsed_item(parse).expect("the fixture's literal parsed an item");
        match item {
            IsoLiteralItem::Entrypoint(declaration) => declaration,
            item => panic!("expected an entrypoint declaration, got {item:?}"),
        }
    }

    fn as_selectable(parse: &WithSpan<IsoLiteralParse>) -> &SelectableDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Selectable(declaration) => declaration,
            item => panic!("expected a selectable declaration, got {item:?}"),
        }
    }

    fn variables_of(parse: &WithSpan<IsoLiteralParse>) -> &WithSpan<VariableDeclarationList> {
        as_selectable(parse)
            .variable_definitions
            .as_ref()
            .expect("the fixture's declaration carries variable definitions")
    }

    fn as_declared(slot: &Slot<VariableDeclaration, UnparsedChunkItems>) -> &VariableDeclaration {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a declared variable")
    }

    fn as_entry(slot: &Slot<ObjectEntry, UnparsedChunkItems>) -> &ObjectEntry {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected an object entry")
    }

    fn selections(
        selection_set: &WithSpan<SelectionSet>,
    ) -> &[WithSpan<Slot<Selection, UnparsedChunkItems>>] {
        selection_set.item.0.reference()
    }

    fn selection_set_of(declaration: &SelectableDeclaration) -> &WithSpan<SelectionSet> {
        declaration
            .selection_set
            .as_ref()
            .expect("the fixture writes a selection set")
    }

    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
    }

    fn assert_no_declaration(
        text: &str,
        reason: AstError,
        reason_span: Span,
        expected_tokens: &[(IsographSemanticToken, &str)],
    ) {
        let (parse, errors) = parsed(text, expected_tokens);
        assert!(
            parsed_item(parse.reference()).is_none(),
            "for literal {text:?}",
        );
        assert!(
            errors
                .iter()
                .any(|error| error.item == reason && error.location == reason_span),
            "for literal {text:?}, errors were {errors:?}",
        );
    }

    #[test]
    fn an_entrypoint_declaration_parses_with_tight_spans() {
        let text = "entrypoint Query.foo";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
            ],
        );
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.name.item,
            SelectableNameWrapper("foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.name.location, span_of(text, "foo"));
        assert_eq!(errors, vec![]);
        assert_eq!(parse.location, Span::from_usize(0, text.len()));
    }

    #[test]
    fn surrounding_line_breaks_and_interior_spaces_are_insignificant() {
        for text in [
            "\n  entrypoint Query.foo\n",
            "\n\nentrypoint Query.foo",
            "entrypoint Query . foo",
        ] {
            let (parse, errors) = parsed(
                text,
                &[
                    (IsographSemanticToken::Keyword, "entrypoint"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "foo"),
                ],
            );
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.parent_type.location,
                span_of(text, "Query"),
                "for literal {text:?}"
            );
            assert_eq!(
                declaration.name.location,
                span_of(text, "foo"),
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn empty_literal_is_none_with_empty_literal_error() {
        let parsed = parsed_with_errors("", &[]);
        assert!(parsed.item.is_none());
        assert!(
            parsed
                .errors
                .iter()
                .any(|error| { error.item == ParseError::Ast(AstError::EmptyLiteral) })
        );
    }

    #[test]
    fn empty_and_whitespace_only_literals_are_empty_literal_errors() {
        for text in ["", "   ", "\n\n"] {
            let parsed = parsed_with_errors(text, &[]);
            assert!(parsed.item.is_none(), "for literal {text:?}");
            assert_eq!(
                parsed.errors,
                ParseError::Ast(AstError::EmptyLiteral)
                    .with_span(Span::from_usize(0, text.len()))
                    .wrap_vec(),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn comma_mistakes_are_chunkings_errors_and_the_declaration_still_parses() {
        for (text, comma_error_count) in
            [(",entrypoint Query.foo", 1), (",,entrypoint Query.foo", 2)]
        {
            let parsed = parsed_with_errors(
                text,
                &[
                    (IsographSemanticToken::Keyword, "entrypoint"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "foo"),
                ],
            );
            assert!(
                parsed
                    .errors
                    .iter()
                    .all(|error| matches!(error.item, ParseError::Comma(_))),
                "for literal {text:?}",
            );
            assert_eq!(
                parsed.errors.len(),
                comma_error_count,
                "for literal {text:?}"
            );
            let parse = parsed.item.expect("the fixture is not an empty literal");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.parent_type.location,
                span_of(text, "Query"),
                "for literal {text:?}"
            );
        }
    }

    #[test]
    fn a_lone_comma_is_chunkings_error_and_an_empty_literal() {
        let text = ",";
        let parsed = parsed_with_errors(text, &[]);
        assert!(parsed.item.is_none());
        assert_eq!(
            parsed
                .errors
                .iter()
                .filter(|error| matches!(error.item, ParseError::Comma(_)))
                .count(),
            1,
        );
        assert!(parsed.errors.iter().any(|error| {
            error.item == ParseError::Ast(AstError::EmptyLiteral)
                && error.location == Span::from_usize(0, text.len())
        }));
        assert!(
            parsed
                .errors
                .iter()
                .all(|error| matches!(error.item, ParseError::Comma(_) | ParseError::Ast(_)))
        );
    }

    #[test]
    fn the_cut_removes_an_unmatched_bracket_and_the_declaration_parses() {
        for text in ["entrypoint Query.foo)", "entrypoint Query.foo ("] {
            let parsed = parsed_with_errors(
                text,
                &[
                    (IsographSemanticToken::Keyword, "entrypoint"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "foo"),
                ],
            );
            assert!(
                parsed
                    .errors
                    .iter()
                    .all(|error| matches!(error.item, ParseError::Bracket(_))),
                "for literal {text:?}",
            );
            assert_eq!(parsed.errors.len(), 1, "for literal {text:?}");
            let parse = parsed.item.expect("the fixture is not an empty literal");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.name.location,
                span_of(text, "foo"),
                "for literal {text:?}"
            );
        }
    }

    #[test]
    fn a_stray_close_is_a_parse_error_and_the_declaration_parses() {
        let text = "entrypoint Query.foo)";
        let parsed = parsed_with_errors(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
            ],
        );
        assert!(parsed.item.is_some());
        assert!(parsed.errors.iter().any(|error| {
            matches!(
                error.item.reference(),
                ParseError::Bracket(BracketError::UnmatchedClose(close))
                    if close.item.0 == BracketKind::Parenthesis
                        && close.location == span_of(text, ")")
            ) && error.location == span_of(text, ")")
        }));
    }

    #[test]
    fn a_final_comma_after_the_declaration_is_an_error() {
        for text in ["entrypoint Query.foo,", "\nentrypoint Query.foo,\n"] {
            let (parse, errors) = parsed(
                text,
                &[
                    (IsographSemanticToken::Keyword, "entrypoint"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "foo"),
                    (IsographSemanticToken::Content, ","),
                ],
            );
            as_entrypoint(parse.reference());
            assert_eq!(
                errors,
                expected(EndOfDeclaration, Found::Token(Comma))
                    .with_span(span_of(text, ","))
                    .wrap_vec(),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_comma_before_a_second_declaration_is_the_boundary_comma() {
        let text = "entrypoint Query.foo, field User.name";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, ","),
                (IsographSemanticToken::Content, "field"),
                (IsographSemanticToken::Content, "User"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "name"),
            ],
        );
        assert_eq!(
            as_entrypoint(parse.reference()).name.location,
            span_of(text, "foo")
        );
        assert_eq!(
            errors,
            vec![
                expected(EndOfDeclaration, Found::Token(Comma)).with_span(span_of(text, ",")),
                AstError::MultipleDeclarations.with_span(span_of(text, "field User.name")),
            ],
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "field"),
                (IsographSemanticToken::Content, "User"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "name"),
            ],
        );
        assert_eq!(
            as_entrypoint(parse.reference()).name.location,
            span_of(text, "foo")
        );
        assert_eq!(
            errors,
            AstError::MultipleDeclarations
                .with_span(span_of(text, "field User.name"))
                .wrap_vec(),
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_failed_first_chunk_is_reported_even_when_a_second_exists() {
        let text = "entrypoint\nQuery.foo";
        let keyword_end = span_of(text, "entrypoint").end;
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "foo"),
            ],
        );
        assert!(parsed_item(parse.reference()).is_none());
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(keyword_end, keyword_end)
        }));
        assert!(
            errors
                .iter()
                .any(|error| error.item == AstError::MultipleDeclarations)
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn an_unknown_keyword_is_an_error_at_the_keyword() {
        let text = "fieldd Query.foo { bar }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "fieldd"),
            &[
                (IsographSemanticToken::Keyword, "fieldd"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn a_literal_opening_with_a_group_expects_a_keyword() {
        let text = "{ bar }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Group(BracketKind::Brace)),
            span_of(text, "{ bar }"),
            &[
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn a_pointer_keyword_is_not_a_declaration() {
        let text = "pointer Pet.BestFriend to Owner { id }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "pointer"),
            &[
                (IsographSemanticToken::Keyword, "pointer"),
                (IsographSemanticToken::Content, "Pet"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "BestFriend"),
                (IsographSemanticToken::Content, "to"),
                (IsographSemanticToken::Content, "Owner"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn each_missing_entrypoint_part_reports_at_its_position() {
        let bare = "entrypoint";
        let keyword_end = span_of(bare, "entrypoint").end;
        assert_no_declaration(
            bare,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(keyword_end, keyword_end),
            &[(IsographSemanticToken::Keyword, "entrypoint")],
        );

        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(
                token(Identifier),
                Found::Token(ErrorNumberLiteralTrailingInvalid),
            ),
            span_of(numeric, "42."),
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Content, "42."),
                (IsographSemanticToken::Content, "foo"),
            ],
        );

        let dotless = "entrypoint Query foo";
        assert_no_declaration(
            dotless,
            expected(token(Period), Found::Token(Identifier)),
            span_of(dotless, "foo"),
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Content, "foo"),
            ],
        );

        let nameless = "entrypoint Query.";
        let dot_end = span_of(nameless, ".").end;
        assert_no_declaration(
            nameless,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(dot_end, dot_end),
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
            ],
        );
    }

    #[test]
    fn a_trailing_comma_after_an_entrypoint_is_extra() {
        let text = "entrypoint Query.foo,";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, ","),
            ],
        );
        as_entrypoint(parse.reference());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("the comma is extra");
        assert_eq!(extra.location, span_of(text, ","));
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn leftover_dollars_after_entrypoint_are_extra() {
        let text = "entrypoint $ $";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Content, "$"),
                (IsographSemanticToken::Content, "$"),
            ],
        );
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("$ $ is extra");
        assert_eq!(extra.location, span_of(text, "$ $"));
        assert!(
            errors
                .iter()
                .any(|error| { error.item == expected(token(Identifier), Found::Token(Dollar)) })
        );
    }

    #[test]
    fn leftover_then_a_trailing_comma_are_both_extra() {
        let text = "entrypoint Query.foo bar,";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Content, ","),
            ],
        );
        as_entrypoint(parse.reference());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("bar and the comma are extra");
        assert_eq!(extra.location, span_of(text, "bar,"));
        assert!(errors.iter().any(|error| {
            error.item == expected(EndOfDeclaration, Found::Token(Identifier))
                && error.location == span_of(text, "bar")
        }));
        assert!(errors.iter().any(|error| {
            error.item == expected(EndOfDeclaration, Found::Token(Comma))
                && error.location == span_of(text, ",")
        }));
    }

    #[test]
    fn a_failed_form_that_consumed_every_item_still_has_extra() {
        let text = "entrypoint Query.";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
            ],
        );
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("the whole contents are extra");
        assert_eq!(extra.location, span_of(text, "entrypoint Query."));
    }

    #[test]
    fn a_failed_form_puts_unread_remainder_in_extra() {
        let text = "entrypoint Foo.$ asdf";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Foo"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::Content, "$"),
                (IsographSemanticToken::Content, "asdf"),
            ],
        );
        assert!(parsed_item(parse.reference()).is_none());
        let extra = first_slot(parse.reference())
            .extra
            .as_ref()
            .expect("$ asdf is extra");
        assert_eq!(extra.location, span_of(text, "$ asdf"));
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
                && error.location == span_of(text, "$")
        }));
    }

    #[test]
    fn a_trailing_comma_after_an_entrypoint_resolves_to_the_comma() {
        let text = "entrypoint Query.foo,";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, ","),
            ],
        );
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover comma, got {node:?}"),
        }
    }

    #[test]
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "bar"),
            ],
        );
        as_entrypoint(parse.reference());
        assert!(parsed_item(parse.reference()).is_some());
        assert!(first_slot(parse.reference()).extra.as_ref().is_some());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_selection_set_on_an_entrypoint_is_leftover() {
        let text = "entrypoint Query.foo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Group(BracketKind::Brace))
                .with_span(span_of(text, "{ bar }"))
                .wrap_vec(),
        );
    }

    #[test]
    fn an_entrypoint_directive_parses() {
        let text = "entrypoint Query.foo @lazyLoad";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "lazyLoad"),
            ],
        );
        assert_eq!(errors, vec![]);
        let directives = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive");
        assert_eq!(directives.location, span_of(text, "@lazyLoad"));
        assert_eq!(directives.item.0.len(), 1);
        assert_eq!(
            directives.item.0[0].item.name.item,
            IsographDirectiveNameWrapper("lazyLoad".intern().to())
        );
        assert!(directives.item.0[0].item.arguments.is_none());
    }

    #[test]
    fn a_field_directive_sits_between_variables_and_the_description() {
        let text = "field Query.Foo($id: ID) @component \"the route\" { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::String, "\"the route\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let field = as_selectable(parse.reference());
        assert!(field.variable_definitions.is_some());
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@component")
        );
        assert!(field.description.is_some());
    }

    #[test]
    fn a_field_directive_sits_between_the_target_and_the_description() {
        let text = "field Pet.BestFriend to Owner @updatable \"x\" { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Owner"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "updatable"),
                (IsographSemanticToken::String, "\"x\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let field = as_selectable(parse.reference());
        assert!(field.target_type.is_some());
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@updatable")
        );
        assert!(field.description.is_some());
    }

    #[test]
    fn a_selection_directive_with_arguments_parses() {
        let text = "field Query.Foo { bar @loadable(lazyLoadArtifact: true) }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "lazyLoadArtifact"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::BooleanOrNull, "true"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let selection = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        );
        let directives = selection
            .directive_set
            .as_ref()
            .expect("the fixture selects with a directive");
        assert_eq!(
            directives.location,
            span_of(text, "@loadable(lazyLoadArtifact: true)")
        );
        let arguments = directives.item.0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
    }

    #[test]
    fn two_directives_on_one_selection_stay_in_one_list() {
        let text = "field Query.Foo { bar @loadable @updatable }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "updatable"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let directives = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        )
        .directive_set
        .as_ref()
        .expect("the fixture selects with directives");
        assert_eq!(directives.item.0.len(), 2);
        assert_eq!(directives.location, span_of(text, "@loadable @updatable"));
    }

    #[test]
    fn a_directive_on_the_next_line_is_its_own_failed_selection() {
        let text = "field Query.Foo { bar\n@loadable }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Content, "@"),
                (IsographSemanticToken::Content, "loadable"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert_eq!(items.len(), 2);
        as_selection(items[0].item.reference());
        assert!(items[1].item.item.is_none());
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::Selection,
                    Found::Token(NonBracketTokenKind::At),
                )
                && error.location == span_of(text, "@")
        }));
    }

    #[test]
    fn an_unknown_directive_name_parses() {
        let text = "entrypoint Query.foo @notARealDirective";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "notARealDirective"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_entrypoint(parse.reference())
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .item
                .0[0]
                .item
                .name
                .item,
            IsographDirectiveNameWrapper("notARealDirective".intern().to())
        );
    }

    #[test]
    fn at_without_a_name_fails_the_host() {
        let text = "entrypoint Query.foo @";
        let end = span_of(text, "@").end;
        assert_no_declaration(
            text,
            expected(Expectation::Token(Identifier), Found::EndOfChunk),
            Span::new(end, end),
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::DirectiveName, "@"),
            ],
        );
    }

    #[test]
    fn directive_names_resolve_through_the_host() {
        let text = "field Query.Foo { bar @loadable }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "loadable")) {
            IsographResolutionNode::IsographDirectiveNameWrapper(name) => {
                match name.parent.parent.parent {
                    IsographFieldDirectiveListParent::Selection(_) => {}
                    parent => panic!("expected a selection directive list, got {parent:?}"),
                }
            }
            node => panic!("expected the directive name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "@")) {
            IsographResolutionNode::IsographFieldDirective(_) => {}
            node => panic!("expected the directive, got {node:?}"),
        }
    }

    #[test]
    fn leftover_after_an_entrypoint_resolves_to_the_leftover_token() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "bar"),
            ],
        );
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
    }

    #[test]
    fn a_gap_after_the_item_resolves_to_the_slot() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::Content, "bar"),
            ],
        );
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::IsoLiteralSlot(path) => {
                assert!(path.inner.item.is_some());
                assert!(path.inner.extra.is_some());
            }
            node => panic!("expected IsoLiteralSlot, got {node:?}"),
        }
    }

    #[test]
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
            ],
        );
        match parse.resolve((), span_of(text, "Query")) {
            IsographResolutionNode::EntityNameWrapper(name) => {
                let declaration = match name.parent {
                    EntityNameWrapperParent::EntrypointDeclaration(declaration) => declaration,
                    parent => panic!("expected an entrypoint parent, got {parent:?}"),
                };
                assert_eq!(declaration.inner.name.location, span_of(text, "foo"));
            }
            node => panic!("expected the entity name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::SelectableNameWrapper(_) => {}
            node => panic!("expected the selectable name leaf, got {node:?}"),
        }
        for span in [
            span_of(text, "entrypoint"),
            span_of(text, "."),
            Span::new(
                span_of(text, "entrypoint").end,
                span_of(text, "Query").start,
            ),
        ] {
            match parse.resolve((), span) {
                IsographResolutionNode::EntrypointDeclaration(_) => {}
                node => panic!("expected the declaration leaf at {span}, got {node:?}"),
            }
        }
    }

    #[test]
    fn positions_inside_a_failed_first_chunk_resolve_through_the_cloned_chunk() {
        let text = "fieldd Query.foo { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "fieldd"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(token) => {
                assert_eq!(token.inner.0, NonBracketTokenKind::Identifier);
            }
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn the_unrecognized_keyword_resolves_as_a_token_in_the_failed_chunk() {
        let text = "fieldd Query.foo { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "fieldd"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "fieldd")) {
            IsographResolutionNode::Singleton(_) => {}
            node => panic!("expected the singleton, got {node:?}"),
        }
    }

    #[test]
    fn a_selectable_declaration_parses_with_selections() {
        let text = "field Query.Foo {\n  bar,\n  baz\n}";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::FieldName, "baz"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let declaration = as_selectable(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.name.item,
            SelectableNameWrapper("Foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.name.location, span_of(text, "Foo"));
        assert_eq!(
            selection_set_of(declaration).location,
            Span::new(span_of(text, "{").start, span_of(text, "}").end)
        );
        let items = selections(selection_set_of(declaration));
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
        assert_eq!(errors, vec![]);
    }

    #[test]
    fn empty_selection_sets_hold_zero_selections() {
        for text in [
            "field Query.Foo {}",
            "field Query.Foo { }",
            "field Query.Foo {\n}",
        ] {
            let (parse, errors) = parsed(
                text,
                &[
                    (IsographSemanticToken::Keyword, "field"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "Foo"),
                    (IsographSemanticToken::Brace, "{"),
                    (IsographSemanticToken::Brace, "}"),
                ],
            );
            assert_eq!(
                selections(selection_set_of(as_selectable(parse.reference()))).len(),
                0,
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_field_declaration_without_a_selection_set_parses() {
        let text = "field Query.Foo";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
    }

    #[test]
    fn a_selection_set_on_its_own_line_is_a_second_declaration() {
        let text = "field Query.Foo\n{ bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
        assert_eq!(
            errors,
            AstError::MultipleDeclarations
                .with_span(span_of(text, "{ bar }"))
                .wrap_vec(),
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_final_comma_after_the_selectable_declaration_is_an_error() {
        let text = "field Query.Foo { bar },";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Content, ","),
            ],
        );
        as_selectable(parse.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn tokens_after_the_selection_set_are_leftover() {
        let text = "field Query.Foo { bar } junk";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Content, "junk"),
            ],
        );
        as_selectable(parse.reference());
        assert!(first_slot(parse.reference()).extra.is_some());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "junk"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_field_without_a_description_has_none() {
        let text = "field Query.Foo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(as_selectable(parse.reference()).description, None);
    }

    #[test]
    fn a_single_line_description_parses_with_its_quotes() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"the home route\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let description = as_selectable(parse.reference())
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            Description("the home route".intern().to())
        );
    }

    #[test]
    fn a_block_string_description_spans_lines_without_splitting_the_chunk() {
        let text = "field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (
                    IsographSemanticToken::String,
                    "\"\"\"\n  the home\n  route\n\"\"\"",
                ),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        let description = declaration
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(selections(selection_set_of(declaration)).len(), 1);
    }

    #[test]
    fn a_description_after_the_selection_set_is_leftover() {
        let text = "field Query.Foo { bar } \"too late\"";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::String, "\"too late\""),
            ],
        );
        as_selectable(parse.reference());
        assert!(first_slot(parse.reference()).extra.is_some());
        assert_eq!(
            errors,
            expected(
                Expectation::EndOfDeclaration,
                Found::Token(NonBracketTokenKind::StringLiteral)
            )
            .with_span(span_of(text, "\"too late\""))
            .wrap_vec(),
        );
    }

    #[test]
    fn an_entrypoint_carries_no_description() {
        let text = "entrypoint Query.foo \"nope\"";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::String, "\"nope\""),
            ],
        );
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(
                Expectation::EndOfDeclaration,
                Found::Token(NonBracketTokenKind::StringLiteral)
            )
            .with_span(span_of(text, "\"nope\""))
            .wrap_vec(),
        );
    }

    #[test]
    fn a_field_declaration_with_only_a_description_parses() {
        let text = "field Query.Foo \"the home route\"";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"the home route\""),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.description.is_some());
        assert_eq!(declaration.selection_set, None);
    }

    #[test]
    fn a_field_without_to_has_no_target_type() {
        let text = "field Query.Foo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(as_selectable(parse.reference()).target_type, None);
    }

    fn named_union_member(name: &str, span: Span) -> WithOptionalSpan<UnionVariant> {
        WithGenericLocation::new(
            UnionVariant::Named(NamedTypeAnnotation {
                name: EntityNameWrapper(name.intern().to()).with_span(span),
            }),
            span.wrap_some(),
        )
    }

    #[test]
    fn an_empty_union_has_no_members() {
        let union = UnionTypeAnnotation(vec![]);
        assert_eq!(union.0.len(), 0);
    }

    #[test]
    fn a_one_member_union_is_a_named_type() {
        let span = Span::new(0, 3);
        let union = UnionTypeAnnotation(named_union_member("Foo", span).wrap_vec());
        assert_eq!(union.0.len(), 1);
        match union.0[0].item.reference() {
            UnionVariant::Named(named) => {
                assert_eq!(named.name.item, EntityNameWrapper("Foo".intern().to()));
                assert_eq!(named.name.location, span);
            }
            variant => panic!("expected Named, got {variant:?}"),
        }
        assert_eq!(union.0[0].location, span.wrap_some());
    }

    #[test]
    fn a_one_member_union_can_be_only_null() {
        let union = UnionTypeAnnotation(
            WithGenericLocation::new(UnionVariant::Null(NullTypeAnnotation), None).wrap_vec(),
        );
        assert_eq!(union.0.len(), 1);
        assert!(matches!(
            union.0[0].item,
            UnionVariant::Null(NullTypeAnnotation)
        ));
        assert_eq!(union.0[0].location, None);
    }

    #[test]
    fn a_three_member_union_keeps_order() {
        let foo = Span::new(0, 3);
        let bar = Span::new(4, 7);
        let union = UnionTypeAnnotation(vec![
            named_union_member("Foo", foo),
            named_union_member("Bar", bar),
            WithGenericLocation::new(UnionVariant::Null(NullTypeAnnotation), None),
        ]);
        assert_eq!(union.0.len(), 3);
        match union.0[0].item.reference() {
            UnionVariant::Named(named) => {
                assert_eq!(named.name.item, EntityNameWrapper("Foo".intern().to()));
            }
            variant => panic!("expected Named Foo, got {variant:?}"),
        }
        match union.0[1].item.reference() {
            UnionVariant::Named(named) => {
                assert_eq!(named.name.item, EntityNameWrapper("Bar".intern().to()));
            }
            variant => panic!("expected Named Bar, got {variant:?}"),
        }
        assert!(matches!(
            union.0[2].item,
            UnionVariant::Null(NullTypeAnnotation)
        ));
        assert_eq!(union.0[2].location, None);
    }

    #[test]
    fn a_field_with_to_parses_the_target_type() {
        let text = "field Pet.BestFriend to Owner { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Owner"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert_eq!(
            declaration.name.item,
            SelectableNameWrapper("BestFriend".intern().to())
        );
        assert_eq!(declaration.name.location, span_of(text, "BestFriend"));
        let target = declaration
            .target_type
            .as_ref()
            .expect("the fixture writes to Owner");
        assert_eq!(target.location, span_of(text, "Owner"));
        match target.item.reference() {
            TypeAnnotation::Union(union) => {
                assert_eq!(union.0.len(), 2);
                match union.0[0].item.reference() {
                    UnionVariant::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Owner"));
                    }
                    variant => panic!("expected Named, got {variant:?}"),
                }
                assert_eq!(union.0[0].location, span_of(text, "Owner").wrap_some());
                assert!(matches!(
                    union.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(union.0[1].location, None);
            }
            annotation => panic!("expected Union, got {annotation:?}"),
        }
        assert_eq!(selections(selection_set_of(declaration)).len(), 1);
    }

    #[test]
    fn a_full_field_parses_in_order() {
        let text = "field Pet.Owner($limit: Int) to Person! \"the owner\" { name }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Owner"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "limit"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Int"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Person"),
                (IsographSemanticToken::String, "\"the owner\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "name"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.variable_definitions.is_some());
        assert_eq!(
            declaration
                .target_type
                .as_ref()
                .expect("the fixture writes to Person!")
                .location,
            span_of(text, "Person")
        );
        assert!(declaration.description.is_some());
    }

    #[test]
    fn a_to_target_accepts_every_type_annotation_form() {
        for (text, target) in [
            ("field Query.Foo to Pet { id }", "Pet"),
            ("field Query.Foo to Pet! { id }", "Pet"),
            ("field Query.Foo to [Pet] { id }", "[Pet]"),
            ("field Query.Foo to [Pet!]! { id }", "[Pet!]"),
            ("field Query.Foo to [[Pet]] { id }", "[[Pet]]"),
        ] {
            let expected_tokens: &[(IsographSemanticToken, &str)] = if text.contains("[[") {
                &[
                    (IsographSemanticToken::Keyword, "field"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "Foo"),
                    (IsographSemanticToken::Keyword, "to"),
                    (IsographSemanticToken::GraphQLTypeName, "["),
                    (IsographSemanticToken::GraphQLTypeName, "["),
                    (IsographSemanticToken::GraphQLTypeName, "Pet"),
                    (IsographSemanticToken::GraphQLTypeName, "]"),
                    (IsographSemanticToken::GraphQLTypeName, "]"),
                    (IsographSemanticToken::Brace, "{"),
                    (IsographSemanticToken::FieldName, "id"),
                    (IsographSemanticToken::Brace, "}"),
                ]
            } else if text.contains('[') {
                &[
                    (IsographSemanticToken::Keyword, "field"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "Foo"),
                    (IsographSemanticToken::Keyword, "to"),
                    (IsographSemanticToken::GraphQLTypeName, "["),
                    (IsographSemanticToken::GraphQLTypeName, "Pet"),
                    (IsographSemanticToken::GraphQLTypeName, "]"),
                    (IsographSemanticToken::Brace, "{"),
                    (IsographSemanticToken::FieldName, "id"),
                    (IsographSemanticToken::Brace, "}"),
                ]
            } else {
                &[
                    (IsographSemanticToken::Keyword, "field"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "Foo"),
                    (IsographSemanticToken::Keyword, "to"),
                    (IsographSemanticToken::GraphQLTypeName, "Pet"),
                    (IsographSemanticToken::Brace, "{"),
                    (IsographSemanticToken::FieldName, "id"),
                    (IsographSemanticToken::Brace, "}"),
                ]
            };
            let (parse, errors) = parsed(text, expected_tokens);
            assert_eq!(errors, vec![], "for literal {text:?}");
            assert_eq!(
                as_selectable(parse.reference())
                    .target_type
                    .as_ref()
                    .expect("the fixture writes a target")
                    .location,
                span_of(text, target),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_bracketed_target_is_a_list_annotation() {
        let text = "field Query.Friends to [Pet!]! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Friends"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes a list target");
        assert_eq!(target.location, span_of(text, "[Pet!]"));
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                match inner.item.reference() {
                    TypeAnnotation::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Pet"));
                    }
                    annotation => panic!("expected a named inner type, got {annotation:?}"),
                }
            }
            annotation => panic!("expected a list target, got {annotation:?}"),
        }
    }

    #[test]
    fn a_named_target_without_bang_is_a_union_with_null() {
        let text = "field Query.Foo to Pet { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet");
        assert_eq!(target.location, span_of(text, "Pet"));
        match target.item.reference() {
            TypeAnnotation::Union(union) => {
                assert_eq!(union.0.len(), 2);
                match union.0[0].item.reference() {
                    UnionVariant::Named(named) => {
                        assert_eq!(named.name.location, span_of(text, "Pet"));
                    }
                    variant => panic!("expected Named, got {variant:?}"),
                }
                assert_eq!(union.0[0].location, span_of(text, "Pet").wrap_some());
                assert!(matches!(
                    union.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(union.0[1].location, None);
            }
            annotation => panic!("expected Union, got {annotation:?}"),
        }
    }

    #[test]
    fn a_named_target_with_bang_is_not_null_wrapped() {
        let text = "field Query.Foo to Pet! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to Pet!");
        assert_eq!(target.location, span_of(text, "Pet"));
        match target.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected Named, got {annotation:?}"),
        }
    }

    #[test]
    fn a_list_target_maps_graphql_nullability() {
        let text = "field Query.Foo to [Pet] { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet]");
        match target.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.0.len(), 2);
                match outer.0[0].item.reference() {
                    UnionVariant::List(list) => {
                        let inner = list.inner.as_ref().expect("the list holds a type");
                        match inner.item.reference() {
                            TypeAnnotation::Union(elem) => {
                                assert_eq!(elem.0.len(), 2);
                                assert!(matches!(elem.0[0].item, UnionVariant::Named(_)));
                                assert!(matches!(
                                    elem.0[1].item,
                                    UnionVariant::Null(NullTypeAnnotation)
                                ));
                                assert_eq!(elem.0[1].location, None);
                            }
                            annotation => panic!("expected Union element, got {annotation:?}"),
                        }
                    }
                    variant => panic!("expected List, got {variant:?}"),
                }
                assert!(matches!(
                    outer.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(outer.0[1].location, None);
            }
            annotation => panic!("expected Union list, got {annotation:?}"),
        }
    }

    #[test]
    fn a_non_null_list_of_non_null_named_is_list_of_named() {
        let text = "field Query.Foo to [Pet!]! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet!]!");
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                assert_eq!(inner.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected List, got {annotation:?}"),
        }
    }

    #[test]
    fn a_second_bang_is_leftover() {
        let text = "field Query.Foo to Pet!! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Content, "!"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        as_selectable(parse.reference());
        let second_bang = Span::new(
            span_of(text, "Pet!!").start + 4,
            span_of(text, "Pet!!").start + 5,
        );
        assert_eq!(
            errors,
            expected(
                EndOfDeclaration,
                Found::Token(NonBracketTokenKind::Exclamation)
            )
            .with_span(second_bang)
            .wrap_vec(),
        );
    }

    #[test]
    fn a_variable_type_without_bang_is_a_union_with_null() {
        let text = "field Query.Foo($x: ID) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        match declared.type_.item.reference() {
            TypeAnnotation::Union(union) => {
                assert_eq!(union.0.len(), 2);
                assert!(matches!(union.0[0].item, UnionVariant::Named(_)));
                assert!(matches!(
                    union.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(union.0[1].location, None);
            }
            annotation => panic!("expected Union, got {annotation:?}"),
        }
    }

    #[test]
    fn a_variable_type_with_bang_is_not_null_wrapped() {
        let text = "field Query.Foo($x: ID!) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        match declared.type_.item.reference() {
            TypeAnnotation::Named(_) => {}
            annotation => panic!("expected Named, got {annotation:?}"),
        }
    }

    #[test]
    fn a_nested_list_target_is_a_union_at_every_layer() {
        let text = "field Query.Foo to [[Pet]] { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [[Pet]]");
        match target.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.0.len(), 2);
                match outer.0[0].item.reference() {
                    UnionVariant::List(outer_list) => {
                        let mid = outer_list
                            .inner
                            .as_ref()
                            .expect("the outer list holds a type");
                        match mid.item.reference() {
                            TypeAnnotation::Union(mid_union) => {
                                assert_eq!(mid_union.0.len(), 2);
                                match mid_union.0[0].item.reference() {
                                    UnionVariant::List(inner_list) => {
                                        let elem = inner_list
                                            .inner
                                            .as_ref()
                                            .expect("the inner list holds a type");
                                        match elem.item.reference() {
                                            TypeAnnotation::Union(elem_union) => {
                                                assert_eq!(elem_union.0.len(), 2);
                                                assert!(matches!(
                                                    elem_union.0[0].item,
                                                    UnionVariant::Named(_)
                                                ));
                                                assert!(matches!(
                                                    elem_union.0[1].item,
                                                    UnionVariant::Null(NullTypeAnnotation)
                                                ));
                                                assert_eq!(elem_union.0[1].location, None);
                                            }
                                            annotation => {
                                                panic!("expected Union named, got {annotation:?}")
                                            }
                                        }
                                    }
                                    variant => panic!("expected inner List, got {variant:?}"),
                                }
                                assert!(matches!(
                                    mid_union.0[1].item,
                                    UnionVariant::Null(NullTypeAnnotation)
                                ));
                                assert_eq!(mid_union.0[1].location, None);
                            }
                            annotation => {
                                panic!("expected Union around inner List, got {annotation:?}")
                            }
                        }
                    }
                    variant => panic!("expected outer List, got {variant:?}"),
                }
                assert!(matches!(
                    outer.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(outer.0[1].location, None);
            }
            annotation => panic!("expected Union around outer List, got {annotation:?}"),
        }
    }

    #[test]
    fn a_nullable_list_of_non_null_named() {
        let text = "field Query.Foo to [Pet!] { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet!]");
        match target.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.0.len(), 2);
                match outer.0[0].item.reference() {
                    UnionVariant::List(list) => {
                        let inner = list.inner.as_ref().expect("the list holds a type");
                        assert!(matches!(inner.item, TypeAnnotation::Named(_)));
                    }
                    variant => panic!("expected List, got {variant:?}"),
                }
                assert!(matches!(
                    outer.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(outer.0[1].location, None);
            }
            annotation => panic!("expected Union list, got {annotation:?}"),
        }
    }

    #[test]
    fn a_non_null_list_of_nullable_named() {
        let text = "field Query.Foo to [Pet]! { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let target = as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes to [Pet]!");
        match target.item.reference() {
            TypeAnnotation::List(list) => {
                let inner = list.inner.as_ref().expect("the list holds a type");
                match inner.item.reference() {
                    TypeAnnotation::Union(elem) => {
                        assert_eq!(elem.0.len(), 2);
                        assert!(matches!(elem.0[0].item, UnionVariant::Named(_)));
                        assert!(matches!(
                            elem.0[1].item,
                            UnionVariant::Null(NullTypeAnnotation)
                        ));
                        assert_eq!(elem.0[1].location, None);
                    }
                    annotation => panic!("expected Union element, got {annotation:?}"),
                }
            }
            annotation => panic!("expected List, got {annotation:?}"),
        }
    }

    #[test]
    fn an_empty_list_target_fails_as_a_type() {
        let text = "field Query.Foo to [] { id }";
        let interior = span_of(text, "[]").start + 1;
        assert_no_declaration(
            text,
            expected(Expectation::TypeAnnotation, Found::EndOfChunk),
            Span::new(interior, interior),
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn a_non_to_identifier_is_not_consumed_as_to() {
        let text = "field Query.Foo Owner { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Content, "Owner"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        as_selectable(parse.reference());
        assert_eq!(as_selectable(parse.reference()).selection_set, None);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "Owner"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_missing_target_type_reports_after_to() {
        let text = "field Pet.BestFriend to { id }";
        assert_no_declaration(
            text,
            expected(
                Expectation::TypeAnnotation,
                Found::Group(BracketKind::Brace),
            ),
            span_of(text, "{ id }"),
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn a_to_at_the_end_of_the_chunk_expects_a_type() {
        let text = "field Pet.BestFriend to";
        let end = span_of(text, "to").end;
        assert_no_declaration(
            text,
            expected(Expectation::TypeAnnotation, Found::EndOfChunk),
            Span::new(end, end),
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
            ],
        );
    }

    #[test]
    fn a_to_after_the_description_is_not_a_target() {
        let text = "field Query.Foo \"x\" to Owner { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"x\""),
                (IsographSemanticToken::Content, "to"),
                (IsographSemanticToken::Content, "Owner"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        let declaration = as_selectable(parse.reference());
        assert!(declaration.description.is_some());
        assert_eq!(declaration.target_type, None);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "to"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_final_comma_after_a_field_with_to_is_an_error() {
        let text = "field Pet.BestFriend to Owner { id },";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Owner"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Content, ","),
            ],
        );
        as_selectable(parse.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfDeclaration, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn to_and_the_target_resolve_with_their_ancestry() {
        let text = "field Pet.BestFriend to Owner { id }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "BestFriend"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Owner"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "id"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "to")) {
            IsographResolutionNode::SelectableDeclaration(_) => {}
            node => panic!("expected the declaration at `to`, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "BestFriend")) {
            IsographResolutionNode::SelectableNameWrapper(name) => match name.parent {
                SelectableNameWrapperParent::SelectableDeclaration(declaration) => {
                    assert_eq!(
                        declaration
                            .inner
                            .target_type
                            .as_ref()
                            .map(|target| target.location),
                        span_of(text, "Owner").wrap_some(),
                    );
                }
                parent => panic!("expected a field parent, got {parent:?}"),
            },
            node => panic!("expected the field name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "Owner")) {
            IsographResolutionNode::EntityNameWrapper(name) => match name.parent {
                EntityNameWrapperParent::NamedTypeAnnotation(named) => match named.parent {
                    TypeAnnotationParent::Union(union) => {
                        let union = union.as_ref();
                        assert!(matches!(
                            union.inner.0[1].item,
                            UnionVariant::Null(NullTypeAnnotation)
                        ));
                        assert_eq!(union.inner.0[1].location, None);
                        match union.parent.reference() {
                            TypeAnnotationParent::SelectableDeclaration(_) => {}
                            parent => panic!("expected the field as type parent, got {parent:?}"),
                        }
                    }
                    parent => panic!("expected Union, got {parent:?}"),
                },
                parent => panic!("expected a named type annotation, got {parent:?}"),
            },
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                match name.parent.parent.parent.parent {
                    SelectionSetParent::SelectableDeclaration(_) => {}
                    parent => panic!("expected the field at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_description_resolves_to_its_leaf() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"the home route\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "home")) {
            IsographResolutionNode::Description(description) => {
                assert_eq!(description.parent.inner.name.location, span_of(text, "Foo"));
            }
            node => panic!("expected the description leaf, got {node:?}"),
        }
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "pet"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "name"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                assert_eq!(name.parent.inner.name.location, span_of(text, "name"));
                let object = match name.parent.parent.parent.parent {
                    SelectionSetParent::Selection(object) => object,
                    parent => panic!("expected a nested-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent.parent.parent {
                    SelectionSetParent::SelectableDeclaration(declaration) => {
                        assert_eq!(declaration.inner.name.location, span_of(text, "Foo"));
                    }
                    parent => panic!("expected the declaration at the top, got {parent:?}"),
                }
            }
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn leftover_positions_resolve_to_the_leftover_token() {
        let text = "field Query.Foo { bar baz }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Content, "baz"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "baz")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::SelectionNameWrapper(_) => {}
            node => panic!("expected the selection name leaf, got {node:?}"),
        }
    }

    #[test]
    fn positions_inside_a_failed_selection_resolve_through_unparsed_items() {
        let text = "field Query.Foo { 42 }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::Integer, "42"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "42")) {
            IsographResolutionNode::NonBracketToken(token) => match token.parent {
                ChunkContentItemParent::Unparsed(unparsed) => match unparsed.parent {
                    UnparsedChunkItemsParent::SelectionSlot(_) => {}
                    parent => panic!("expected a selection-slot parent, got {parent:?}"),
                },
                parent => panic!("expected an unparsed parent, got {parent:?}"),
            },
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn whitespace_and_separators_inside_a_selection_set_resolve_to_the_set() {
        let text = "field Query.Foo { bar, baz }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::FieldName, "baz"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::SelectionSet(_) => {}
            node => panic!("expected the selection set, got {node:?}"),
        }
    }

    #[test]
    fn argument_names_resolve_through_the_selection() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::ArgumentNameWrapper(name) => {
                match name.parent.parent.parent.parent {
                    ArgumentListParent::Selection(selection) => {
                        assert_eq!(selection.inner.name.location, span_of(text, "bar"));
                    }
                    parent => panic!("expected a selection argument list, got {parent:?}"),
                }
            }
            node => panic!("expected the argument name, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Usage(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
    }

    #[test]
    fn a_dollar_in_a_use_resolves_to_declaration_or_usage_with_usage_parent() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Usage(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "x")) {
            IsographResolutionNode::VariableNameWrapper(name) => {
                assert_eq!(
                    name.inner.dereference(),
                    VariableNameWrapper("x".intern().to())
                );
            }
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_dollar_in_a_declaration_resolves_to_declaration_or_usage_with_declaration_parent() {
        let text = "field Query.Foo($id: ID) { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Declaration(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::VariableNameWrapper(_) => {}
            node => panic!("expected the name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_single_line_description_drops_the_quotes() {
        let text = "\"the home route\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description =
            consume_description(stream.cursor()).expect("the fixture is a string description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            Description("the home route".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_semantic_tokens(
            text,
            &tokens,
            &[(IsographSemanticToken::String, "\"the home route\"")],
        );
    }

    #[test]
    fn an_empty_string_is_a_description() {
        let text = "\"\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description =
            consume_description(stream.cursor()).expect("the fixture is a string description");
        assert_eq!(description.location, span_of(text, "\"\""));
        assert_eq!(description.item, Description("".intern().to()));
        assert_semantic_tokens(text, &tokens, &[(IsographSemanticToken::String, "\"\"")]);
    }

    #[test]
    fn a_block_string_description_is_one_token_including_line_breaks() {
        let text = "\"\"\"\n  the home\n  route\n\"\"\"";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a block-string description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(
            description.item,
            Description("the home\nroute".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_semantic_tokens(
            text,
            &tokens,
            &[(
                IsographSemanticToken::String,
                "\"\"\"\n  the home\n  route\n\"\"\"",
            )],
        );
    }

    #[test]
    fn a_block_string_with_line_breaks_does_not_split_the_chunk() {
        let text = "Foo \"\"\"\n  the home\n  route\n\"\"\" Bar";
        let tree = chunked(text);
        assert_eq!(tree.item.0.len(), 1);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(
            cursor
                .consume_token_if(
                    NonBracketTokenKind::Identifier,
                    IsographSemanticToken::FieldName
                )
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
        let description =
            consume_description(cursor).expect("the fixture carries a block-string description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(
            cursor
                .consume_token_if(
                    NonBracketTokenKind::Identifier,
                    IsographSemanticToken::FieldName
                )
                .map(|token| token.location),
            span_of(text, "Bar").wrap_some(),
        );
        assert_semantic_tokens(
            text,
            &tokens,
            &[
                (IsographSemanticToken::FieldName, "Foo"),
                (
                    IsographSemanticToken::String,
                    "\"\"\"\n  the home\n  route\n\"\"\"",
                ),
                (IsographSemanticToken::FieldName, "Bar"),
            ],
        );
    }

    #[test]
    fn a_description_does_not_consume_the_following_item() {
        let text = "\"hi\" Foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        let description =
            consume_description(cursor).expect("the fixture starts with a description");
        assert_eq!(description.location, span_of(text, "\"hi\""));
        assert_eq!(
            cursor
                .consume_token_if(
                    NonBracketTokenKind::Identifier,
                    IsographSemanticToken::FieldName
                )
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
        assert_semantic_tokens(
            text,
            &tokens,
            &[
                (IsographSemanticToken::String, "\"hi\""),
                (IsographSemanticToken::FieldName, "Foo"),
            ],
        );
    }

    #[test]
    fn a_non_string_is_not_a_description() {
        let text = "Foo";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(consume_description(stream.cursor()), None);
        }
        assert_semantic_tokens(text, &tokens, &[]);
        assert_eq!(errors, vec![]);
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_token_if(
                    NonBracketTokenKind::Identifier,
                    IsographSemanticToken::FieldName
                )
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
        assert_semantic_tokens(text, &tokens, &[(IsographSemanticToken::FieldName, "Foo")]);
    }

    #[test]
    fn a_brace_group_is_not_a_description() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(consume_description(stream.cursor()), None);
        }
        assert_semantic_tokens(text, &tokens, &[]);
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_group_if(BracketKind::Brace, IsographSemanticToken::Brace, |_, _| ())
                .map(|group| group.location),
            span_of(text, "{ bar }").wrap_some(),
        );
        assert_semantic_tokens(
            text,
            &tokens,
            &[
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
    }

    #[test]
    fn an_unterminated_string_is_not_a_description() {
        let text = "\"unterminated";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        {
            let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
            assert_eq!(consume_description(stream.cursor()), None);
        }
        assert_semantic_tokens(text, &tokens, &[]);
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_token_if(
                    NonBracketTokenKind::ErrorUnterminatedString,
                    IsographSemanticToken::Error
                )
                .map(|token| token.location),
            span_of(text, "\"unterminated").wrap_some(),
        );
        assert_semantic_tokens(
            text,
            &tokens,
            &[(IsographSemanticToken::Error, "\"unterminated")],
        );
    }

    #[test]
    fn an_empty_block_string_is_a_description() {
        let text = "\"\"\"\"\"\"";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let description = consume_description(stream.cursor())
            .expect("the fixture is a block-string description");
        assert_eq!(description.location, span_of(text, "\"\"\"\"\"\""));
        assert_eq!(description.item, Description("".intern().to()));
        assert_semantic_tokens(
            text,
            &tokens,
            &[(IsographSemanticToken::String, "\"\"\"\"\"\"")],
        );
    }

    #[test]
    fn a_multi_line_variable_list_parses_in_the_demo_style() {
        let text = "field Query.PetCheckinListRoute(\n  $id: ID !\n) {\n  pets\n}";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "PetCheckinListRoute"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "pets"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let variables = variables_of(parse.reference());
        assert_eq!(variables.item.0.len(), 1);
        let declared = as_declared(variables.item.0[0].item.reference());
        assert_eq!(
            declared.name.item.0.item,
            VariableNameWrapper("id".intern().to())
        );
        assert_eq!(declared.name.item.0.location, span_of(text, "id"));
        assert_eq!(declared.name.location, span_of(text, "$id"));
        match declared.type_.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "ID"));
                assert_eq!(declared.type_.location, span_of(text, "ID"));
            }
            annotation => panic!("expected a named type, got {annotation:?}"),
        }
    }

    #[test]
    fn list_types_nest_with_non_null_markers() {
        let text = "field Query.Foo($pets: [Pet!]!) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "pets"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert_eq!(declared.type_.location, span_of(text, "[Pet!]"));
        let list = match declared.type_.item.reference() {
            TypeAnnotation::List(list) => list.as_ref(),
            annotation => panic!("expected a list type, got {annotation:?}"),
        };
        let inner = list.inner.as_ref().expect("the list holds an element type");
        match inner.item.reference() {
            TypeAnnotation::Named(named) => {
                assert_eq!(named.name.location, span_of(text, "Pet"));
                assert_eq!(inner.location, span_of(text, "Pet"));
            }
            annotation => panic!("expected the named element type, got {annotation:?}"),
        }
    }

    #[test]
    fn defaults_parse_including_variables() {
        let text = "field Query.Foo($limit: Int = 10) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "limit"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Int"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Integer, "10"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        let default = declared
            .default_value
            .as_ref()
            .expect("the fixture declares a default");
        assert!(matches!(
            default.item,
            NonConstantValue::Integer(IntegerValue(10))
        ));

        let shallow = "field Query.Foo($limit: Int = $other) { bar }";
        let (parse, errors) = parsed(
            shallow,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "limit"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Int"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "other"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        let default = declared
            .default_value
            .as_ref()
            .expect("the fixture declares a default");
        assert!(matches!(default.item, NonConstantValue::Variable(_)));

        let list = "field Query.Foo($ids: [ID!] = [1, $x]) { bar }";
        let (parse, errors) = parsed(
            list,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "ids"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Bracket, "["),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "x"),
                (IsographSemanticToken::Bracket, "]"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        let default = declared
            .default_value
            .as_ref()
            .expect("the fixture declares a list default");
        match default.item.reference() {
            NonConstantValue::List(list) => {
                assert_eq!(list.0.len(), 2);
                assert!(matches!(
                    list.0[0]
                        .item
                        .item
                        .as_ref()
                        .expect("the fixture's first element parsed")
                        .item
                        .value
                        .item,
                    NonConstantValue::Integer(IntegerValue(1))
                ));
                assert!(matches!(
                    list.0[1]
                        .item
                        .item
                        .as_ref()
                        .expect("the fixture's second element parsed")
                        .item
                        .value
                        .item,
                    NonConstantValue::Variable(_)
                ));
            }
            value => panic!("expected a list default, got {value:?}"),
        }

        let deep = "field Query.Foo($input: Input = { pet: $pet }) { bar }";
        let (parse, errors) = parsed(
            deep,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "input"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Input"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::ObjectKey, "pet"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "pet"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        let default = declared
            .default_value
            .as_ref()
            .expect("the fixture declares a default");
        match default.item.reference() {
            NonConstantValue::Object(object) => {
                let entry = as_entry(object.0[0].item.reference());
                assert!(matches!(entry.value.item, NonConstantValue::Variable(_)));
            }
            value => panic!("expected an object default, got {value:?}"),
        }

        let cross = "field Query.Foo($foo: String = \"foo\", $bar: Input = { foo: $foo }) { baz }";
        let (parse, errors) = parsed(
            cross,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "foo"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "String"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::String, "\"foo\""),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "bar"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Input"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::ObjectKey, "foo"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "foo"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "baz"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let variables = variables_of(parse.reference());
        assert_eq!(variables.item.0.len(), 2);
        let bar = as_declared(variables.item.0[1].item.reference());
        match bar
            .default_value
            .as_ref()
            .expect("bar has a default")
            .item
            .reference()
        {
            NonConstantValue::Object(object) => {
                let entry = as_entry(object.0[0].item.reference());
                assert!(matches!(entry.value.item, NonConstantValue::Variable(_)));
            }
            value => panic!("expected an object default, got {value:?}"),
        }
    }

    #[test]
    fn a_default_variable_resolves_through_variable_default() {
        let text = "field Query.Foo($limit: Int = $other) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "limit"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Int"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "other"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        match parse.resolve((), span_of(text, "other")) {
            IsographResolutionNode::VariableNameWrapper(name) => match name.parent.parent {
                VariableDeclarationOrUsageParent::Usage(variable_use) => {
                    match variable_use.parent {
                        NonConstantValueParent::VariableDefault(declaration) => {
                            assert_eq!(declaration.inner.name.location, span_of(text, "$limit"));
                        }
                        parent => panic!("expected VariableDefault, got {parent:?}"),
                    }
                }
                parent => panic!("expected Usage, got {parent:?}"),
            },
            node => panic!("expected the variable name leaf, got {node:?}"),
        }
        let use_dollar = Span::new(
            span_of(text, "$other").start,
            span_of(text, "$other").start + 1,
        );
        match parse.resolve((), use_dollar) {
            IsographResolutionNode::VariableDeclarationOrUsage(node) => {
                assert!(matches!(
                    node.parent,
                    VariableDeclarationOrUsageParent::Usage(_)
                ));
            }
            node => panic!("expected VariableDeclarationOrUsage, got {node:?}"),
        }
    }

    #[test]
    fn each_malformed_variable_declaration_degrades_alone() {
        let text = "field Query.Foo($a Int, $b: , id: ID, $c: Float) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "a"),
                (IsographSemanticToken::Content, "Int"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Content, ":"),
                (IsographSemanticToken::Content, "ID"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "c"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Float"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let variables = variables_of(parse.reference());
        assert_eq!(variables.item.0.len(), 4);
        assert!(variables.item.0[0].item.item.is_none());
        assert!(variables.item.0[1].item.item.is_none());
        assert!(variables.item.0[2].item.item.is_none());
        as_declared(variables.item.0[3].item.reference());
        assert_eq!(errors.len(), 3);
        assert_eq!(errors[0].location, span_of(text, "Int"));
        assert!(errors[1].item == expected(Expectation::TypeAnnotation, Found::EndOfChunk));
        assert_eq!(errors[2].location, span_of(text, "id"));
    }

    #[test]
    fn a_final_comma_inside_a_list_type_is_end_of_type() {
        let text = "field Query.Foo($pets: [Pet,]) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "pets"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Content, ","),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        assert_eq!(
            errors,
            expected(Expectation::EndOfType, Found::Token(Comma))
                .with_span(span_of(text, ","))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_line_break_inside_a_list_type_does_not_attach_bang() {
        let text = "field Query.Foo($pets: [Pet\n!]) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "pets"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::Content, "!"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let declared = as_declared(variables_of(parse.reference()).item.0[0].item.reference());
        match declared.type_.item.reference() {
            TypeAnnotation::Union(outer) => {
                assert_eq!(outer.0.len(), 2);
                match outer.0[0].item.reference() {
                    UnionVariant::List(list) => {
                        let inner = list.inner.as_ref().expect("chunk 0 parsed Pet");
                        match inner.item.reference() {
                            TypeAnnotation::Union(elem) => {
                                assert!(matches!(elem.0[0].item, UnionVariant::Named(_)));
                                assert_eq!(elem.0[0].location, span_of(text, "Pet").wrap_some());
                                assert!(matches!(
                                    elem.0[1].item,
                                    UnionVariant::Null(NullTypeAnnotation)
                                ));
                                assert_eq!(elem.0[1].location, None);
                            }
                            annotation => panic!("expected Union element, got {annotation:?}"),
                        }
                    }
                    variant => panic!("expected List, got {variant:?}"),
                }
                assert!(matches!(
                    outer.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(outer.0[1].location, None);
            }
            annotation => panic!("expected Union list, got {annotation:?}"),
        }
        assert!(errors.iter().any(|error| {
            error.item
                == expected(
                    Expectation::EndOfType,
                    Found::Token(NonBracketTokenKind::Exclamation),
                )
                && error.location == span_of(text, "!")
        }));
    }

    #[test]
    fn type_names_resolve_through_their_annotation_ancestry() {
        let text = "field Query.Foo($pets: [Pet]) { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "pets"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "["),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
                (IsographSemanticToken::GraphQLTypeName, "]"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "Pet")) {
            IsographResolutionNode::EntityNameWrapper(name) => {
                let named = match name.parent.reference() {
                    EntityNameWrapperParent::NamedTypeAnnotation(named) => named,
                    parent => panic!("expected a named type annotation, got {parent:?}"),
                };
                let inner_union = match named.parent.reference() {
                    TypeAnnotationParent::Union(union) => union.as_ref(),
                    parent => panic!("expected Union around Named, got {parent:?}"),
                };
                assert!(matches!(
                    inner_union.inner.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(inner_union.inner.0[1].location, None);
                let list = match inner_union.parent.reference() {
                    TypeAnnotationParent::List(list) => list.as_ref(),
                    parent => panic!("expected a list parent, got {parent:?}"),
                };
                let outer_union = match list.parent.reference() {
                    TypeAnnotationParent::Union(union) => union.as_ref(),
                    parent => panic!("expected Union around List, got {parent:?}"),
                };
                assert!(matches!(
                    outer_union.inner.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(outer_union.inner.0[1].location, None);
                match outer_union.parent.reference() {
                    TypeAnnotationParent::Variable(variable) => {
                        assert_eq!(variable.inner.name.location, span_of(text, "$pets"));
                    }
                    parent => panic!("expected the declared variable, got {parent:?}"),
                }
            }
            node => panic!("expected the type name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "pets")) {
            IsographResolutionNode::VariableNameWrapper(name) => {
                assert!(matches!(
                    name.parent.parent,
                    VariableDeclarationOrUsageParent::Declaration(_)
                ));
            }
            node => panic!("expected the variable name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_bang_resolves_to_the_variable_declaration() {
        let text = "field Query.Foo($id: ID!) { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "!")) {
            IsographResolutionNode::VariableDeclaration(declaration) => {
                assert_eq!(declaration.inner.name.location, span_of(text, "$id"));
            }
            node => panic!("expected the variable declaration, got {node:?}"),
        }
    }

    #[test]
    fn an_empty_variable_list_is_some_and_empty() {
        let text = "field Query.Foo() { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let variables = as_selectable(parse.reference())
            .variable_definitions
            .as_ref()
            .expect("() is a present list");
        assert_eq!(variables.item.0.len(), 0);
        assert!(as_selectable(parse.reference()).selection_set.is_some());
    }

    #[test]
    fn a_to_without_a_selection_set_parses() {
        let text = "field Query.Foo to Pet";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Pet"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert_eq!(
            declaration
                .target_type
                .as_ref()
                .expect("the fixture writes to Pet")
                .location,
            span_of(text, "Pet")
        );
        assert_eq!(declaration.selection_set, None);
    }

    #[test]
    fn a_directive_without_vars_or_to_parses() {
        let text = "field Query.Foo @component { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let field = as_selectable(parse.reference());
        assert!(field.variable_definitions.is_none());
        assert_eq!(field.target_type, None);
        assert_eq!(
            field
                .directive_set
                .as_ref()
                .expect("the fixture carries a directive")
                .location,
            span_of(text, "@component")
        );
    }

    #[test]
    fn a_full_header_includes_directives() {
        let text = "field Pet.Owner($limit: Int) to Person! @updatable \"the owner\" { name }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Pet"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Owner"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "limit"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "Int"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "Person"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "updatable"),
                (IsographSemanticToken::String, "\"the owner\""),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "name"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert!(declaration.variable_definitions.is_some());
        assert!(declaration.target_type.is_some());
        assert!(declaration.directive_set.is_some());
        assert!(declaration.description.is_some());
        assert!(declaration.selection_set.is_some());
    }

    #[test]
    fn two_directives_on_a_field_stay_in_one_list() {
        let text = "field Query.Foo @a @b { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "a"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "b"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let directives = as_selectable(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries directives");
        assert_eq!(directives.item.0.len(), 2);
        assert_eq!(directives.location, span_of(text, "@a @b"));
    }

    #[test]
    fn two_directives_on_an_entrypoint_stay_in_one_list() {
        let text = "entrypoint Query.foo @a @b";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "a"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "b"),
            ],
        );
        assert_eq!(errors, vec![]);
        let directives = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries directives");
        assert_eq!(directives.item.0.len(), 2);
    }

    #[test]
    fn a_directive_after_the_description_is_leftover() {
        let text = "field Query.Foo \"x\" @component { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::String, "\"x\""),
                (IsographSemanticToken::Content, "@"),
                (IsographSemanticToken::Content, "component"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        let declaration = as_selectable(parse.reference());
        assert!(declaration.description.is_some());
        assert_eq!(declaration.directive_set, None);
        assert_eq!(
            errors[0],
            expected(EndOfDeclaration, Found::Token(At)).with_span(span_of(text, "@")),
        );
    }

    #[test]
    fn a_to_after_a_directive_is_leftover() {
        let text = "field Query.Foo @component to Pet { id }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::Content, "to"),
                (IsographSemanticToken::Content, "Pet"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "id"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        let declaration = as_selectable(parse.reference());
        assert!(declaration.directive_set.is_some());
        assert_eq!(declaration.target_type, None);
        assert_eq!(
            errors[0],
            expected(EndOfDeclaration, Found::Token(Identifier)).with_span(span_of(text, "to")),
        );
    }

    #[test]
    fn a_dollar_without_a_name_fails_that_variable() {
        let text = "field Query.Foo($) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        as_selectable(parse.reference());
        let variables = variables_of(parse.reference());
        assert!(variables.item.0[0].item.item.is_none());
        let dollar_end = span_of(text, "$").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(dollar_end, dollar_end)
        }));
    }

    #[test]
    fn an_equals_without_a_default_fails_that_variable() {
        let text = "field Query.Foo($id: ID =) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Equals, "="),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        as_selectable(parse.reference());
        let variables = variables_of(parse.reference());
        assert!(variables.item.0[0].item.item.is_none());
        let eq_end = span_of(text, "=").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(Expectation::Value, Found::EndOfChunk)
                && error.location == Span::new(eq_end, eq_end)
        }));
    }

    #[test]
    fn an_alias_without_a_name_fails_that_selection() {
        let text = "field Query.Foo { b: }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "b"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert!(items[0].item.item.is_none());
        let colon_end = span_of(text, ":").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(colon_end, colon_end)
        }));
    }

    #[test]
    fn to_as_a_selectable_name_is_the_name() {
        let text = "field Query.to { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "to"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).name.location,
            span_of(text, "to")
        );
        assert_eq!(as_selectable(parse.reference()).target_type, None);
    }

    #[test]
    fn to_as_a_target_type_name_parses() {
        let text = "field Query.Foo to to { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Keyword, "to"),
                (IsographSemanticToken::GraphQLTypeName, "to"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).name.location,
            span_of(text, "Foo")
        );
        match as_selectable(parse.reference())
            .target_type
            .as_ref()
            .expect("the fixture writes a target")
            .item
            .reference()
        {
            TypeAnnotation::Union(union) => {
                assert_eq!(union.0.len(), 2);
                match union.0[0].item.reference() {
                    UnionVariant::Named(named) => {
                        assert_eq!(named.name.item, EntityNameWrapper("to".intern().to()));
                    }
                    variant => panic!("expected Named, got {variant:?}"),
                }
                assert!(matches!(
                    union.0[1].item,
                    UnionVariant::Null(NullTypeAnnotation)
                ));
                assert_eq!(union.0[1].location, None);
            }
            annotation => panic!("expected Union, got {annotation:?}"),
        }
    }

    #[test]
    fn uppercase_to_is_not_the_keyword() {
        let text = "field Query.Foo TO Pet { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Content, "TO"),
                (IsographSemanticToken::Content, "Pet"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
        as_selectable(parse.reference());
        assert_eq!(as_selectable(parse.reference()).target_type, None);
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "TO"))
                .wrap_vec(),
        );
    }

    #[test]
    fn true_false_and_null_as_selection_names_are_selections() {
        for name in ["true", "false", "null"] {
            let text = format!("field Query.Foo {{ {name} }}");
            let (parse, errors) = parsed(
                text.reference(),
                &[
                    (IsographSemanticToken::Keyword, "field"),
                    (IsographSemanticToken::Type, "Query"),
                    (IsographSemanticToken::Period, "."),
                    (IsographSemanticToken::FieldName, "Foo"),
                    (IsographSemanticToken::Brace, "{"),
                    (IsographSemanticToken::FieldName, name),
                    (IsographSemanticToken::Brace, "}"),
                ],
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
            assert_eq!(
                as_selection(
                    selections(selection_set_of(as_selectable(parse.reference())))[0]
                        .item
                        .reference()
                )
                .name
                .item,
                SelectionNameWrapper(name.intern().to()),
                "for literal {text:?}",
            );
        }
    }

    #[test]
    fn a_directive_with_empty_arguments_parses() {
        let text = "field Query.Foo { bar @loadable() }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let arguments = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        )
        .directive_set
        .as_ref()
        .expect("the fixture selects with a directive")
        .item
        .0[0]
            .item
            .arguments
            .as_ref()
            .expect("() is a present list");
        assert_eq!(arguments.item.0.len(), 0);
    }

    #[test]
    fn a_selection_with_alias_arguments_directives_and_a_nested_set_parses() {
        let text = "field Query.Foo { a: bar(id: $id) @loadable { baz } }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "a"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "loadable"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "baz"),
                (IsographSemanticToken::Brace, "}"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let selection = as_selection(
            selections(selection_set_of(as_selectable(parse.reference())))[0]
                .item
                .reference(),
        );
        assert!(selection.reader_alias.is_some());
        assert!(selection.arguments.is_some());
        assert!(selection.directive_set.is_some());
        assert!(selection.selection_set.is_some());
    }

    #[test]
    fn field_directive_names_resolve_through_the_declaration() {
        let text = "field Query.Foo @component { bar }";
        let (parse, _) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        match parse.resolve((), span_of(text, "component")) {
            IsographResolutionNode::IsographDirectiveNameWrapper(name) => {
                match name.parent.parent.parent {
                    IsographFieldDirectiveListParent::SelectableDeclaration(_) => {}
                    parent => panic!("expected a field directive list, got {parent:?}"),
                }
            }
            node => panic!("expected the directive name leaf, got {node:?}"),
        }
    }

    #[test]
    fn a_field_with_only_variables_parses() {
        let text = "field Query.Foo($id: ID)";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Variable, "$"),
                (IsographSemanticToken::Variable, "id"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::GraphQLTypeName, "ID"),
                (IsographSemanticToken::Parenthesis, ")"),
            ],
        );
        assert_eq!(errors, vec![]);
        let declaration = as_selectable(parse.reference());
        assert_eq!(variables_of(parse.reference()).item.0.len(), 1);
        assert_eq!(declaration.selection_set, None);
    }

    #[test]
    fn to_as_a_parent_type_name_is_the_parent() {
        let text = "field to.Foo { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "to"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            as_selectable(parse.reference()).parent_type.item,
            EntityNameWrapper("to".intern().to())
        );
        assert_eq!(
            as_selectable(parse.reference()).parent_type.location,
            span_of(text, "to")
        );
        assert_eq!(as_selectable(parse.reference()).target_type, None);
    }

    #[test]
    fn uppercase_field_is_not_the_keyword() {
        let text = "FIELD Query.Foo { bar }";
        assert_no_declaration(
            text,
            expected(DECLARATION_KEYWORD, Found::Token(Identifier)),
            span_of(text, "FIELD"),
            &[
                (IsographSemanticToken::Keyword, "FIELD"),
                (IsographSemanticToken::Content, "Query"),
                (IsographSemanticToken::Content, "."),
                (IsographSemanticToken::Content, "Foo"),
                (IsographSemanticToken::Bracket, "{"),
                (IsographSemanticToken::Content, "bar"),
                (IsographSemanticToken::Bracket, "}"),
            ],
        );
    }

    #[test]
    fn at_without_a_name_fails_that_selection() {
        let text = "field Query.Foo { bar @ }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        let items = selections(selection_set_of(as_selectable(parse.reference())));
        assert_eq!(items.len(), 1);
        assert!(items[0].item.item.is_none());
        let at_end = span_of(text, "@").end;
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(at_end, at_end)
        }));
    }

    #[test]
    fn an_entrypoint_directive_with_arguments_parses() {
        let text = "entrypoint Query.foo @lazyLoad(x: 1)";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "entrypoint"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "lazyLoad"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Parenthesis, ")"),
            ],
        );
        assert_eq!(errors, vec![]);
        let arguments = as_entrypoint(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive")
            .item
            .0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
        assert_eq!(arguments.location, span_of(text, "(x: 1)"));
    }

    #[test]
    fn a_field_directive_with_arguments_parses() {
        let text = "field Query.Foo @component(x: 1) { bar }";
        let (parse, errors) = parsed(
            text,
            &[
                (IsographSemanticToken::Keyword, "field"),
                (IsographSemanticToken::Type, "Query"),
                (IsographSemanticToken::Period, "."),
                (IsographSemanticToken::FieldName, "Foo"),
                (IsographSemanticToken::DirectiveName, "@"),
                (IsographSemanticToken::DirectiveName, "component"),
                (IsographSemanticToken::Parenthesis, "("),
                (IsographSemanticToken::Argument, "x"),
                (IsographSemanticToken::Colon, ":"),
                (IsographSemanticToken::Integer, "1"),
                (IsographSemanticToken::Parenthesis, ")"),
                (IsographSemanticToken::Brace, "{"),
                (IsographSemanticToken::FieldName, "bar"),
                (IsographSemanticToken::Brace, "}"),
            ],
        );
        assert_eq!(errors, vec![]);
        let arguments = as_selectable(parse.reference())
            .directive_set
            .as_ref()
            .expect("the fixture carries a directive")
            .item
            .0[0]
            .item
            .arguments
            .as_ref()
            .expect("the fixture passes arguments");
        assert_eq!(arguments.item.0.len(), 1);
        assert_eq!(arguments.location, span_of(text, "(x: 1)"));
    }
}
