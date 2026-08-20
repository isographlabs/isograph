use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ChunkedLevel, Expectation, ExtraChunks, Found, IsographResolutionNode, NonBracketTokenKind,
    ParseError, SelectionSet, SemanticToken, Singleton, Slot, UnparsedChunkItems, parse_singleton,
    require_selection_set,
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
    Field(ClientFieldDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(EntrypointDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = IsoLiteralSlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldDeclaration {
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub parent_type: WithSpan<EntityNameWrapper>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub client_field_name: WithSpan<ClientScalarSelectableNameWrapper>,
    #[resolve_field]
    pub description: Option<WithSpan<Description>>,
    #[resolve_field]
    #[parent_variant(ClientFieldDeclaration)]
    pub selection_set: WithSpan<SelectionSet>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntityNameWrapperParent<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityNameWrapper(common_lang_types::EntityName);

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(
    parent_type = ClientScalarSelectableNameWrapperParent<'a>,
    resolved_node = IsographResolutionNode<'a>
)]
pub struct ClientScalarSelectableNameWrapper(common_lang_types::SelectableName);

/// The interned source slice of a description, quotes included.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = ClientFieldDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct Description(common_lang_types::DescriptionValue);

#[derive(Debug)]
pub enum EntityNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}

#[derive(Debug)]
pub enum ClientScalarSelectableNameWrapperParent<'a> {
    EntrypointDeclaration(EntrypointDeclarationPath<'a>),
    ClientFieldDeclaration(ClientFieldDeclarationPath<'a>),
}

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, IsoLiteralSlotPath<'a>>;

pub type ClientFieldDeclarationPath<'a> =
    PositionResolutionPath<&'a ClientFieldDeclaration, IsoLiteralSlotPath<'a>>;

pub type DescriptionPath<'a> =
    PositionResolutionPath<&'a Description, ClientFieldDeclarationPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

pub type EntityNameWrapperPath<'a> =
    PositionResolutionPath<&'a EntityNameWrapper, EntityNameWrapperParent<'a>>;

pub type ClientScalarSelectableNameWrapperPath<'a> = PositionResolutionPath<
    &'a ClientScalarSelectableNameWrapper,
    ClientScalarSelectableNameWrapperParent<'a>,
>;

pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    errors: &mut Vec<WithSpan<ParseError>>,
    tokens: &mut Vec<WithSpan<SemanticToken>>,
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        errors.push(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        tokens,
        errors,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        parse_iso_literal_item,
    );
    singleton.with_span(location).wrap_some()
}

fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Keyword)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match keyword.text() {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" => IsoLiteralItem::Field(parse_field(cursor)?).wrap_ok(),
        "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword.location)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword.location)
        .wrap_err(),
    }
}

fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    EntrypointDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
    }
    .wrap_ok()
}

fn parse_field(
    cursor: &mut ItemCursor<'_>,
) -> Result<ClientFieldDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::Type)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period, SemanticToken::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    let description = consume_description(cursor);
    let selection_set = require_selection_set(cursor)?;
    ClientFieldDeclaration {
        parent_type: parent_type.interned().map(EntityNameWrapper),
        client_field_name: client_field_name
            .interned()
            .map(ClientScalarSelectableNameWrapper),
        description,
        selection_set,
    }
    .wrap_ok()
}

pub(crate) fn consume_description(cursor: &mut ItemCursor<'_>) -> Option<WithSpan<Description>> {
    let span = cursor
        .consume_token_if(NonBracketTokenKind::StringLiteral, SemanticToken::String)
        .or_else(|| {
            cursor.consume_token_if(
                NonBracketTokenKind::BlockStringLiteral,
                SemanticToken::String,
            )
        })?;
    span.interned().map(Description).wrap_some()
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use resolve_position::ResolvePosition;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        BracketError, BracketKind, ChunkContentItemParent, CommaWithoutItem, Expectation, Found,
        IsographResolutionNode, NonBracketTokenKind, ParseError, Selection, SelectionNameWrapper,
        SelectionSet, SelectionSetParent, Slot, UnparsedChunkItems, UnparsedChunkItemsParent,
        chunk, match_brackets, tokenize,
    };
    use Expectation::{DeclarationKeyword, EndOfDeclaration};
    use NonBracketTokenKind::{
        At, Comma, Dollar, ErrorNumberLiteralTrailingInvalid, Identifier, Period,
    };

    type ParsedWithErrors = (
        Option<WithSpan<IsoLiteralParse>>,
        Vec<WithSpan<ParseError>>,
        Vec<BracketError>,
        Vec<CommaWithoutItem>,
    );

    type ParsedWithTokens = (
        Option<WithSpan<IsoLiteralParse>>,
        Vec<WithSpan<ParseError>>,
        Vec<BracketError>,
        Vec<CommaWithoutItem>,
        Vec<WithSpan<SemanticToken>>,
    );

    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (parse.expect("the fixture is not an empty literal"), errors)
    }

    fn parsed_with_errors(text: &str) -> ParsedWithErrors {
        let (parse, errors, bracket_errors, comma_errors, _) = parsed_with_tokens(text);
        (parse, errors, bracket_errors, comma_errors)
    }

    fn parsed_with_tokens(text: &str) -> ParsedWithTokens {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let mut tokens = Vec::new();
        let parse = parse_iso_literal(text, tree, &mut errors, &mut tokens);
        (parse, errors, bracket_errors, comma_errors, tokens)
    }

    fn expected(expectation: Expectation, found: Found) -> ParseError {
        ParseError::expected(expectation, found)
    }

    fn token(kind: NonBracketTokenKind) -> Expectation {
        Expectation::Token(kind)
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
        tokens: &'a mut Vec<WithSpan<SemanticToken>>,
        errors: &'a mut Vec<WithSpan<ParseError>>,
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

    fn as_field(parse: &WithSpan<IsoLiteralParse>) -> &ClientFieldDeclaration {
        match parsed_item(parse).expect("the fixture's literal parsed an item") {
            IsoLiteralItem::Field(declaration) => declaration,
            item => panic!("expected a field declaration, got {item:?}"),
        }
    }

    fn selections(
        selection_set: &WithSpan<SelectionSet>,
    ) -> &[WithSpan<Slot<Selection, UnparsedChunkItems>>] {
        selection_set.item.0.reference()
    }

    fn as_selection(slot: &Slot<Selection, UnparsedChunkItems>) -> &Selection {
        slot.item
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
            .expect("expected a selection")
    }

    fn assert_no_declaration(text: &str, reason: ParseError, reason_span: Span) {
        let (parse, errors) = parsed(text);
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
        let (parse, errors) = parsed(text);
        let declaration = as_entrypoint(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.client_field_name.item,
            ClientScalarSelectableNameWrapper("foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "foo"));
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
            let (parse, errors) = parsed(text);
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.parent_type.location,
                span_of(text, "Query"),
                "for literal {text:?}"
            );
            assert_eq!(
                declaration.client_field_name.location,
                span_of(text, "foo"),
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn empty_and_whitespace_only_literals_are_empty_literal_errors() {
        for text in ["", "   ", "\n\n"] {
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            assert!(parse.is_none(), "for literal {text:?}");
            assert_eq!(
                errors,
                ParseError::EmptyLiteral
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
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert!(bracket_errors.is_empty(), "for literal {text:?}");
            assert_eq!(
                comma_errors.len(),
                comma_error_count,
                "for literal {text:?}"
            );
            let parse = parse.expect("the fixture is not an empty literal");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.parent_type.location,
                span_of(text, "Query"),
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_lone_comma_is_chunkings_error_and_an_empty_literal() {
        let text = ",";
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors.len(), 1);
        assert!(parse.is_none());
        assert_eq!(
            errors,
            ParseError::EmptyLiteral
                .with_span(Span::from_usize(0, text.len()))
                .wrap_vec(),
        );
    }

    #[test]
    fn the_cut_removes_an_unmatched_bracket_and_the_declaration_parses() {
        for text in ["entrypoint Query.foo)", "entrypoint Query.foo ("] {
            let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
            assert_eq!(bracket_errors.len(), 1, "for literal {text:?}");
            assert_eq!(comma_errors, vec![], "for literal {text:?}");
            let parse = parse.expect("the fixture is not an empty literal");
            let declaration = as_entrypoint(parse.reference());
            assert_eq!(
                declaration.client_field_name.location,
                span_of(text, "foo"),
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_final_comma_after_the_declaration_is_an_error() {
        for text in ["entrypoint Query.foo,", "\nentrypoint Query.foo,\n"] {
            let (parse, errors) = parsed(text);
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
        let (parse, errors) = parsed(text);
        assert_eq!(
            as_entrypoint(parse.reference()).client_field_name.location,
            span_of(text, "foo")
        );
        assert_eq!(
            errors,
            vec![
                expected(EndOfDeclaration, Found::Token(Comma)).with_span(span_of(text, ",")),
                ParseError::MultipleDeclarations.with_span(span_of(text, "field User.name")),
            ],
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_second_contentful_chunk_is_multiple_declarations() {
        let text = "entrypoint Query.foo\nfield User.name";
        let (parse, errors) = parsed(text);
        assert_eq!(
            as_entrypoint(parse.reference()).client_field_name.location,
            span_of(text, "foo")
        );
        assert_eq!(
            errors,
            ParseError::MultipleDeclarations
                .with_span(span_of(text, "field User.name"))
                .wrap_vec(),
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn a_failed_first_chunk_is_reported_even_when_a_second_exists() {
        let text = "entrypoint\nQuery.foo";
        let keyword_end = span_of(text, "entrypoint").end;
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::EndOfChunk)
                && error.location == Span::new(keyword_end, keyword_end)
        }));
        assert!(
            errors
                .iter()
                .any(|error| error.item == ParseError::MultipleDeclarations)
        );
        assert!(parse.item.extra_chunks.as_ref().is_some());
    }

    #[test]
    fn an_unknown_keyword_is_an_error_at_the_keyword() {
        let text = "fieldd Query.foo { bar }";
        assert_no_declaration(
            text,
            expected(DeclarationKeyword, Found::Token(Identifier)),
            span_of(text, "fieldd"),
        );
    }

    #[test]
    fn a_literal_opening_with_a_group_expects_a_keyword() {
        let text = "{ bar }";
        assert_no_declaration(
            text,
            expected(DeclarationKeyword, Found::Group(BracketKind::Brace)),
            span_of(text, "{ bar }"),
        );
    }

    #[test]
    fn field_and_pointer_declarations_do_not_parse_yet() {
        let pointer = "pointer Query.foo to Bar { id }";
        assert_no_declaration(
            pointer,
            ParseError::UnsupportedDeclarationType,
            span_of(pointer, "pointer"),
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
        );

        let numeric = "entrypoint 42.foo";
        assert_no_declaration(
            numeric,
            expected(
                token(Identifier),
                Found::Token(ErrorNumberLiteralTrailingInvalid),
            ),
            span_of(numeric, "42."),
        );

        let dotless = "entrypoint Query foo";
        assert_no_declaration(
            dotless,
            expected(token(Period), Found::Token(Identifier)),
            span_of(dotless, "foo"),
        );

        let nameless = "entrypoint Query.";
        let dot_end = span_of(nameless, ".").end;
        assert_no_declaration(
            nameless,
            expected(token(Identifier), Found::EndOfChunk),
            Span::new(dot_end, dot_end),
        );
    }

    #[test]
    fn a_failed_form_keeps_the_whole_chunk_as_remaining() {
        let text = "entrypoint Foo.$ asdf";
        let (parse, errors) = parsed(text);
        assert!(parsed_item(parse.reference()).is_none());
        match first_slot(parse.reference())
            .extra_tokens
            .as_ref()
            .map(|wrapped| wrapped.item.reference())
        {
            Some(items) => {
                assert_eq!(items.0.first().location, span_of(text, "entrypoint"));
                assert_eq!(items.0.last().location, span_of(text, "asdf"));
            }
            None => panic!("expected remaining items covering the whole chunk"),
        }
        assert!(errors.iter().any(|error| {
            error.item == expected(token(Identifier), Found::Token(Dollar))
                && error.location == span_of(text, "$")
        }));
    }

    #[test]
    fn tokens_after_a_complete_entrypoint_are_leftover() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert!(parsed_item(parse.reference()).is_some());
        assert!(
            first_slot(parse.reference())
                .extra_tokens
                .as_ref()
                .is_some()
        );
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
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Group(BracketKind::Brace))
                .with_span(span_of(text, "{ bar }"))
                .wrap_vec(),
        );
    }

    #[test]
    fn a_directive_is_an_ordinary_unexpected_token() {
        let text = "entrypoint Query.foo @lazy";
        let (parse, errors) = parsed(text);
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(At))
                .with_span(span_of(text, "@"))
                .wrap_vec(),
        );
    }

    #[test]
    fn leftover_after_an_entrypoint_resolves_to_the_leftover_token() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
    }

    #[test]
    fn a_gap_after_the_item_resolves_to_the_slot() {
        let text = "entrypoint Query.foo bar";
        let (parse, _) = parsed(text);
        let gap = Span::new(span_of(text, "foo").end, span_of(text, "bar").start);
        match parse.resolve((), gap) {
            IsographResolutionNode::IsoLiteralSlot(path) => {
                assert!(path.inner.item.is_some());
                assert!(path.inner.extra_tokens.is_some());
            }
            node => panic!("expected IsoLiteralSlot, got {node:?}"),
        }
    }

    #[test]
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Query")) {
            IsographResolutionNode::EntityNameWrapper(name) => {
                let declaration = match name.parent {
                    EntityNameWrapperParent::EntrypointDeclaration(declaration) => declaration,
                    parent => panic!("expected an entrypoint parent, got {parent:?}"),
                };
                assert_eq!(
                    declaration.inner.client_field_name.location,
                    span_of(text, "foo")
                );
            }
            node => panic!("expected the entity name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::ClientScalarSelectableNameWrapper(_) => {}
            node => panic!("expected the client field name leaf, got {node:?}"),
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
        let (parse, _) = parsed(text);
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
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "fieldd")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the token leaf, got {node:?}"),
        }
    }

    #[test]
    fn an_entrypoint_records_keyword_type_period_field_name() {
        let text = "entrypoint Query.foo";
        let (parse, errors, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        let parse = parse.expect("the fixture is not an empty literal");
        as_entrypoint(parse.reference());
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "entrypoint")),
                SemanticToken::Type.with_span(span_of(text, "Query")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
            ],
        );
    }

    #[test]
    fn a_failed_prefix_keeps_the_tokens_it_committed() {
        let text = "entrypoint Foo.$ asdf";
        let (_, _, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "entrypoint")),
                SemanticToken::Type.with_span(span_of(text, "Foo")),
                SemanticToken::Period.with_span(span_of(text, ".")),
            ],
        );
    }

    #[test]
    fn an_unknown_keyword_records_keyword_at_that_identifier() {
        let text = "fieldd Query.foo { bar }";
        let (_, _, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        assert_eq!(
            tokens,
            SemanticToken::Keyword
                .with_span(span_of(text, "fieldd"))
                .wrap_vec(),
        );
    }

    #[test]
    fn leftover_after_an_entrypoint_is_not_recorded() {
        let text = "entrypoint Query.foo bar";
        let (parse, errors, bracket_errors, comma_errors, tokens) = parsed_with_tokens(text);
        assert!(bracket_errors.is_empty());
        assert_eq!(comma_errors, vec![]);
        let parse = parse.expect("the fixture is not an empty literal");
        as_entrypoint(parse.reference());
        assert_eq!(
            errors,
            expected(EndOfDeclaration, Found::Token(Identifier))
                .with_span(span_of(text, "bar"))
                .wrap_vec(),
        );
        assert_eq!(
            tokens,
            vec![
                SemanticToken::Keyword.with_span(span_of(text, "entrypoint")),
                SemanticToken::Type.with_span(span_of(text, "Query")),
                SemanticToken::Period.with_span(span_of(text, ".")),
                SemanticToken::FieldName.with_span(span_of(text, "foo")),
            ],
        );
    }

    #[test]
    fn a_field_declaration_parses_with_scalar_selections() {
        let text = "field Query.Foo {\n  bar,\n  baz\n}";
        let (parse, errors) = parsed(text);
        let declaration = as_field(parse.reference());
        assert_eq!(
            declaration.parent_type.item,
            EntityNameWrapper("Query".intern().to())
        );
        assert_eq!(
            declaration.client_field_name.item,
            ClientScalarSelectableNameWrapper("Foo".intern().to())
        );
        assert_eq!(declaration.parent_type.location, span_of(text, "Query"));
        assert_eq!(declaration.client_field_name.location, span_of(text, "Foo"));
        assert_eq!(
            declaration.selection_set.location,
            Span::new(span_of(text, "{").start, span_of(text, "}").end)
        );
        let items = selections(declaration.selection_set.reference());
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
            let (parse, errors) = parsed(text);
            assert_eq!(
                selections(as_field(parse.reference()).selection_set.reference()).len(),
                0,
                "for literal {text:?}"
            );
            assert_eq!(errors, vec![], "for literal {text:?}");
        }
    }

    #[test]
    fn a_field_declaration_without_a_selection_set_is_a_failed_item() {
        let text = "field Query.Foo";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_selection_set_split_onto_its_own_line_is_a_failed_item() {
        let text = "field Query.Foo\n{ bar }";
        let end = span_of(text, "Foo").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_final_comma_after_the_field_declaration_is_an_error() {
        let text = "field Query.Foo { bar },";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
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
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert!(first_slot(parse.reference()).extra_tokens.is_some());
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
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        assert_eq!(as_field(parse.reference()).description, None);
    }

    #[test]
    fn a_single_line_description_parses_with_its_quotes() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let description = as_field(parse.reference())
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(description.location, span_of(text, "\"the home route\""));
        assert_eq!(
            description.item,
            Description("\"the home route\"".intern().to())
        );
    }

    #[test]
    fn a_block_string_description_spans_lines_without_splitting_the_chunk() {
        let text = "field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let (parse, errors) = parsed(text);
        assert_eq!(errors, vec![]);
        let declaration = as_field(parse.reference());
        let description = declaration
            .description
            .as_ref()
            .expect("the fixture carries a description");
        assert_eq!(
            description.location,
            span_of(text, "\"\"\"\n  the home\n  route\n\"\"\"")
        );
        assert_eq!(selections(declaration.selection_set.reference()).len(), 1);
    }

    #[test]
    fn a_description_after_the_selection_set_is_leftover() {
        let text = "field Query.Foo { bar } \"too late\"";
        let (parse, errors) = parsed(text);
        as_field(parse.reference());
        assert!(first_slot(parse.reference()).extra_tokens.is_some());
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
        let (parse, errors) = parsed(text);
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
    fn a_description_without_a_selection_set_is_a_failed_item() {
        let text = "field Query.Foo \"the home route\"";
        let end = span_of(text, "\"the home route\"").end;
        assert_no_declaration(
            text,
            expected(Expectation::SelectionSet, Found::EndOfChunk),
            Span::new(end, end),
        );
    }

    #[test]
    fn a_description_resolves_to_its_leaf() {
        let text = "field Query.Foo \"the home route\" { bar }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "home")) {
            IsographResolutionNode::Description(description) => {
                assert_eq!(
                    description.parent.inner.client_field_name.location,
                    span_of(text, "Foo")
                );
            }
            node => panic!("expected the description leaf, got {node:?}"),
        }
    }

    #[test]
    fn selection_names_resolve_with_their_ancestry() {
        let text = "field Query.Foo { pet { name } }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "name")) {
            IsographResolutionNode::SelectionNameWrapper(name) => {
                assert_eq!(name.parent.inner.name.location, span_of(text, "name"));
                let object = match name.parent.parent.parent.parent {
                    SelectionSetParent::Selection(object) => object,
                    parent => panic!("expected a nested-selection parent, got {parent:?}"),
                };
                assert_eq!(object.inner.name.location, span_of(text, "pet"));
                match object.parent.parent.parent {
                    SelectionSetParent::ClientFieldDeclaration(declaration) => {
                        assert_eq!(
                            declaration.inner.client_field_name.location,
                            span_of(text, "Foo")
                        );
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
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "baz")) {
            IsographResolutionNode::NonBracketToken(_) => {}
            node => panic!("expected the leftover token, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "bar")) {
            IsographResolutionNode::SelectionNameWrapper(_) => {}
            node => panic!("expected the selection name, got {node:?}"),
        }
    }

    #[test]
    fn positions_inside_a_failed_selection_resolve_through_unparsed_items() {
        let text = "field Query.Foo { 42 }";
        let (parse, _) = parsed(text);
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
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, ",")) {
            IsographResolutionNode::SelectionSet(_) => {}
            node => panic!("expected the selection set, got {node:?}"),
        }
    }

    #[test]
    fn argument_names_resolve_through_the_selection() {
        let text = "field Query.Foo { bar(id: $x) }";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "id")) {
            IsographResolutionNode::FieldArgumentNameWrapper(name) => {
                assert_eq!(
                    name.parent.parent.parent.parent.inner.name.location,
                    span_of(text, "bar")
                );
            }
            node => panic!("expected the argument name, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "$")) {
            IsographResolutionNode::VariableUse(_) => {}
            node => panic!("expected the variable use, got {node:?}"),
        }
    }

    #[test]
    fn a_single_line_description_is_the_source_slice_including_quotes() {
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
            Description("\"the home route\"".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            SemanticToken::String
                .with_span(span_of(text, "\"the home route\""))
                .wrap_vec(),
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
        assert_eq!(description.item, Description("\"\"".intern().to()));
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
            Description("\"\"\"\n  the home\n  route\n\"\"\"".intern().to())
        );
        assert_eq!(errors, vec![]);
        assert_eq!(
            tokens,
            SemanticToken::String
                .with_span(span_of(text, "\"\"\"\n  the home\n  route\n\"\"\""))
                .wrap_vec(),
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
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
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
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Bar").wrap_some(),
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
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
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
        assert_eq!(tokens, vec![]);
        assert_eq!(errors, vec![]);
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Identifier, SemanticToken::FieldName)
                .map(|token| token.location),
            span_of(text, "Foo").wrap_some(),
        );
    }

    #[test]
    fn a_brace_group_is_not_a_description() {
        let text = "{ bar }";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_group_if(BracketKind::Brace, SemanticToken::Brace, |_, _| ())
                .map(|group| group.location),
            span_of(text, "{ bar }").wrap_some(),
        );
    }

    #[test]
    fn an_unterminated_string_is_not_a_description() {
        let text = "\"unterminated";
        let tree = chunked(text);
        let mut tokens = Vec::new();
        let mut errors = Vec::new();
        let mut stream = stream_of(tree.reference(), text, &mut tokens, &mut errors);
        let cursor = stream.cursor();
        assert_eq!(consume_description(cursor), None);
        assert_eq!(
            cursor
                .consume_token_if(NonBracketTokenKind::Error, SemanticToken::Error)
                .map(|token| token.location),
            span_of(text, "\"").wrap_some(),
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
        assert_eq!(description.item, Description("\"\"\"\"\"\"".intern().to()));
    }
}
