use intern::string_key::Intern;
use prelude::Postfix;
use resolve_position::PositionResolutionPath;
use resolve_position_macros::ResolvePosition;
use span::{WithSpan, WithSpanPostfix};

use crate::chunk_stream::ItemCursor;
use crate::{
    ChunkedLevel, Expectation, ExtraChunks, Found, IsographResolutionNode, NonBracketTokenKind,
    ParseError, Singleton, Slot, UnparsedChunkItems, parse_singleton,
};

pub type IsoLiteralParse = Singleton<Slot<IsoLiteralItem, UnparsedChunkItems>, ExtraChunks>;

pub type IsoLiteralParsePath<'a> = PositionResolutionPath<&'a IsoLiteralParse, ()>;

pub type SlotPath<'a> =
    PositionResolutionPath<&'a Slot<IsoLiteralItem, UnparsedChunkItems>, IsoLiteralParsePath<'a>>;

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub enum IsoLiteralItem {
    Entrypoint(EntrypointDeclaration),
}

#[derive(Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = SlotPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntrypointDeclaration {
    #[resolve_field]
    pub parent_type: WithSpan<EntityName>,
    #[resolve_field]
    pub client_field_name: WithSpan<ClientFieldName>,
}

/// The name of a schema type, `Query` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct EntityName(common_lang_types::EntityName);

impl From<intern::string_key::StringKey> for EntityName {
    fn from(key: intern::string_key::StringKey) -> Self {
        EntityName(key.to())
    }
}

/// The name of the client field an entrypoint targets, `foo` in `entrypoint Query.foo`.
#[derive(Copy, Clone, Debug, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type = EntrypointDeclarationPath<'a>, resolved_node = IsographResolutionNode<'a>)]
pub struct ClientFieldName(common_lang_types::SelectableName);

impl From<intern::string_key::StringKey> for ClientFieldName {
    fn from(key: intern::string_key::StringKey) -> Self {
        ClientFieldName(key.to())
    }
}

pub type EntrypointDeclarationPath<'a> =
    PositionResolutionPath<&'a EntrypointDeclaration, SlotPath<'a>>;

pub type ExtraChunksPath<'a> = PositionResolutionPath<&'a ExtraChunks, IsoLiteralParsePath<'a>>;

pub type UnparsedChunkItemsPath<'a> = PositionResolutionPath<&'a UnparsedChunkItems, SlotPath<'a>>;

pub type EntityNamePath<'a> = PositionResolutionPath<&'a EntityName, EntrypointDeclarationPath<'a>>;

pub type ClientFieldNamePath<'a> =
    PositionResolutionPath<&'a ClientFieldName, EntrypointDeclarationPath<'a>>;

pub fn parse_iso_literal(
    text: &str,
    root: WithSpan<ChunkedLevel>,
    mut push_error: impl FnMut(WithSpan<ParseError>),
) -> Option<WithSpan<IsoLiteralParse>> {
    let location = root.location;
    if root.item.len() == 0 {
        push_error(ParseError::EmptyLiteral.with_span(location));
        return None;
    }
    let singleton = parse_singleton(
        root.reference(),
        text,
        Expectation::EndOfDeclaration,
        |extra| ParseError::MultipleDeclarations.with_span(extra.location),
        |cursor, _| parse_iso_literal_item(cursor),
        &mut push_error,
    );
    singleton.with_span(location).wrap_some()
}

fn parse_iso_literal_item(
    cursor: &mut ItemCursor<'_>,
) -> Result<IsoLiteralItem, WithSpan<ParseError>> {
    let keyword = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::DeclarationKeyword))?;
    match cursor.token_text(keyword) {
        "entrypoint" => IsoLiteralItem::Entrypoint(parse_entrypoint(cursor)?).wrap_ok(),
        "field" | "pointer" => ParseError::UnsupportedDeclarationType
            .with_span(keyword)
            .wrap_err(),
        _ => ParseError::expected(
            Expectation::DeclarationKeyword,
            Found::Token(NonBracketTokenKind::Identifier),
        )
        .with_span(keyword)
        .wrap_err(),
    }
}

fn parse_entrypoint(
    cursor: &mut ItemCursor<'_>,
) -> Result<EntrypointDeclaration, WithSpan<ParseError>> {
    let parent_type = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    cursor
        .require_token(NonBracketTokenKind::Period)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Period)))?;
    let client_field_name = cursor
        .require_token(NonBracketTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(NonBracketTokenKind::Identifier)))?;
    EntrypointDeclaration {
        parent_type: cursor
            .token_text(parent_type)
            .intern()
            .to::<EntityName>()
            .with_span(parent_type),
        client_field_name: cursor
            .token_text(client_field_name)
            .intern()
            .to::<ClientFieldName>()
            .with_span(client_field_name),
    }
    .wrap_ok()
}

#[cfg(test)]
mod tests {
    use intern::string_key::Intern;
    use prelude::Postfix;
    use resolve_position::ResolvePosition;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::*;
    use crate::{
        BracketError, BracketKind, CommaWithoutItem, Expectation, Found, IsographResolutionNode,
        NonBracketTokenKind, ParseError, Slot, UnparsedChunkItems, chunk, match_brackets, tokenize,
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

    fn parsed(text: &str) -> (WithSpan<IsoLiteralParse>, Vec<WithSpan<ParseError>>) {
        let (parse, errors, bracket_errors, comma_errors) = parsed_with_errors(text);
        assert!(bracket_errors.is_empty(), "for literal {text:?}");
        assert_eq!(comma_errors, vec![], "for literal {text:?}");
        (parse.expect("the fixture is not an empty literal"), errors)
    }

    fn parsed_with_errors(text: &str) -> ParsedWithErrors {
        let (brackets, bracket_errors) = match_brackets(tokenize(text), text.len() as u32);
        let (tree, comma_errors) = chunk(brackets.reference());
        let mut errors = Vec::new();
        let parse = parse_iso_literal(text, tree, |error| errors.push(error));
        (parse, errors, bracket_errors, comma_errors)
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
        }
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
        assert_eq!(declaration.parent_type.item, "Query".intern().to());
        assert_eq!(declaration.client_field_name.item, "foo".intern().to());
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
        let field = "field Query.foo { bar }";
        assert_no_declaration(
            field,
            ParseError::UnsupportedDeclarationType,
            span_of(field, "field"),
        );
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
    fn names_resolve_to_their_leaves_and_the_rest_to_the_declaration() {
        let text = "entrypoint Query.foo";
        let (parse, _) = parsed(text);
        match parse.resolve((), span_of(text, "Query")) {
            IsographResolutionNode::EntityName(name) => {
                assert_eq!(
                    name.parent.inner.client_field_name.location,
                    span_of(text, "foo")
                );
            }
            node => panic!("expected the entity name leaf, got {node:?}"),
        }
        match parse.resolve((), span_of(text, "foo")) {
            IsographResolutionNode::ClientFieldName(_) => {}
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
}
