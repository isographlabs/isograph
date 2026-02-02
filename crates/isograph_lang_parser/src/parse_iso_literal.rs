use common_lang_types::{
    Diagnostic, DiagnosticResult, EmbeddedLocation, IsoLiteralText, Location,
    RelativePathToSourceFile, SelectableName, Span, TextSource, ValueKeyName, VariableName,
    WithEmbeddedLocation, WithLocationPostfix, WithSpanPostfix,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation, NameValuePair,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, ClientScalarSelectableNameWrapper,
    ConstantValue, EntityNameWrapper, EntrypointDeclaration, IsographFieldDirective,
    IsographResolvedNode, IsographSemanticToken, NonConstantValue, ObjectSelection,
    ScalarSelection, Selection, SelectionFieldArgument, SelectionSet, SelectionType,
    TypeAnnotationDeclaration, VariableDeclaration, VariableNameWrapper,
    from_isograph_field_directives, semantic_token_legend,
};
use prelude::Postfix;
use resolve_position_macros::ResolvePosition;
use std::ops::ControlFlow;

use crate::{IsographLangTokenKind, parse_optional_description, peekable_lexer::PeekableLexer};

#[derive(Debug, Clone, PartialEq, Eq, ResolvePosition)]
#[resolve_position(parent_type=(), resolved_node=IsographResolvedNode<'a>)]
pub enum IsoLiteralExtractionResult {
    ClientPointerDeclaration(WithEmbeddedLocation<ClientPointerDeclaration>),
    ClientFieldDeclaration(WithEmbeddedLocation<ClientFieldDeclaration>),
    EntrypointDeclaration(WithEmbeddedLocation<EntrypointDeclaration>),
}

impl IsoLiteralExtractionResult {
    pub fn semantic_tokens(&self) -> &[WithEmbeddedLocation<IsographSemanticToken>] {
        match self {
            IsoLiteralExtractionResult::ClientPointerDeclaration(s) => &s.item.semantic_tokens,
            IsoLiteralExtractionResult::ClientFieldDeclaration(s) => &s.item.semantic_tokens,
            IsoLiteralExtractionResult::EntrypointDeclaration(s) => &s.item.semantic_tokens,
        }
    }
}

pub fn parse_iso_literal(
    iso_literal_text: String,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<String>,
    // TODO we should not pass the text source here! Whenever the iso literal
    // moves around the page, we break memoizaton, due to this parameter.
    text_source: TextSource,
) -> DiagnosticResult<IsoLiteralExtractionResult> {
    let mut tokens = PeekableLexer::new(&iso_literal_text, text_source);
    let discriminator = tokens.peek();
    let text = tokens.source(discriminator.location.span);
    // TODO this is awkward. Entrypoint has a different isograph semantic token type than
    // field and pointer, hence we have to peek, then re-parse.

    match text {
        "entrypoint" => {
            let entrypoint_keyword = tokens.parse_source_of_kind(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_KEYWORD_USE,
            )?;
            IsoLiteralExtractionResult::EntrypointDeclaration(parse_iso_entrypoint_declaration(
                &mut tokens,
                entrypoint_keyword.location,
                (&iso_literal_text).intern().into(),
            )?)
            .wrap_ok()
        }
        "field" => {
            tokens.parse_source_of_kind(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_KEYWORD_DECLARATION,
            )?;
            IsoLiteralExtractionResult::ClientFieldDeclaration(parse_iso_client_field_declaration(
                &mut tokens,
                definition_file_path,
                const_export_name.as_deref(),
            )?)
            .wrap_ok()
        }
        "pointer" => {
            tokens.parse_source_of_kind(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_KEYWORD_DECLARATION,
            )?;
            IsoLiteralExtractionResult::ClientPointerDeclaration(
                parse_iso_client_pointer_declaration(
                    &mut tokens,
                    definition_file_path,
                    const_export_name.as_deref(),
                )?,
            )
            .wrap_ok()
        }
        _ => Diagnostic::new(
            "Isograph literals must start with on the keywords `field`, `pointer` or `entrypoint`"
                .to_string(),
            discriminator.location.to::<Location>().wrap_some(),
        )
        .wrap_err(),
    }
}

fn parse_iso_entrypoint_declaration(
    tokens: &mut PeekableLexer<'_>,
    entrypoint_keyword_location: EmbeddedLocation,
    iso_literal_text: IsoLiteralText,
) -> DiagnosticResult<WithEmbeddedLocation<EntrypointDeclaration>> {
    let entrypoint_declaration = tokens.with_embedded_location_result(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_SERVER_OBJECT_TYPE,
            )
            .map(|with_embedded_location| with_embedded_location.map(EntityNameWrapper))?;

        let dot = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;
        let client_field_name = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_CLIENT_SELECTABLE_NAME,
            )
            .map(|with_embedded_location| {
                with_embedded_location.map(ClientScalarSelectableNameWrapper)
            })?;

        let directive_set = parse_directives(tokens)?;

        EntrypointDeclaration {
            parent_type,
            client_field_name,
            iso_literal_text,
            entrypoint_keyword: ().with_location(entrypoint_keyword_location),
            dot: dot.map(|_| ()),
            directive_set,
            semantic_tokens: tokens.semantic_tokens(),
        }
        .wrap_ok::<Diagnostic>()
    })?;

    if let Some(span) = tokens.remaining_token_span() {
        leftover_tokens_diagnostic(Location::new(tokens.text_source, span)).wrap_err()
    } else {
        entrypoint_declaration.wrap_ok()
    }
}

fn parse_iso_client_field_declaration(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
) -> DiagnosticResult<WithEmbeddedLocation<ClientFieldDeclaration>> {
    let client_field_declaration =
        parse_client_field_declaration_inner(tokens, definition_file_path, const_export_name)?;

    if let Some(span) = tokens.remaining_token_span() {
        leftover_tokens_diagnostic(Location::new(tokens.text_source, span)).wrap_err()
    } else {
        client_field_declaration.wrap_ok()
    }
}

fn parse_client_field_declaration_inner(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
) -> DiagnosticResult<WithEmbeddedLocation<ClientFieldDeclaration>> {
    tokens.with_embedded_location_result(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_SERVER_OBJECT_TYPE,
            )
            .map(|with_embedded_location| with_embedded_location.map(EntityNameWrapper))?;

        let _ = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;

        let client_field_name: WithEmbeddedLocation<SelectableName> = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_CLIENT_SELECTABLE_NAME,
            )?;

        let variable_definitions = parse_variable_definitions(tokens)?;

        let directives = parse_directives(tokens)?;

        let description = parse_optional_description(tokens);

        let selection_set = parse_optional_selection_set(tokens)?.ok_or_else(|| {
            expected_selection_set_diagnostic(Location::new(
                tokens.text_source,
                // TODO get a span
                Span::todo_generated(),
            ))
        })?;

        let const_export_name = const_export_name.ok_or_else(|| {
            expected_literal_to_be_exported_diagnostic(
                "field",
                client_field_name.item,
                // TODO use a better location
                client_field_name.location.into(),
            )
        })?;

        ClientFieldDeclaration {
            parent_type,
            client_field_name: client_field_name.map(|x| x.into()),
            description,
            selection_set,
            definition_path: definition_file_path,
            directive_set: directives,
            const_export_name: const_export_name.intern().into(),
            variable_definitions,

            semantic_tokens: tokens.semantic_tokens(),
        }
        .wrap_ok()
    })
}

fn parse_iso_client_pointer_declaration(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
) -> DiagnosticResult<WithEmbeddedLocation<ClientPointerDeclaration>> {
    let client_pointer_declaration =
        parse_client_pointer_declaration_inner(tokens, definition_file_path, const_export_name)?;

    if let Some(span) = tokens.remaining_token_span() {
        return leftover_tokens_diagnostic(Location::new(tokens.text_source, span)).wrap_err();
    }

    client_pointer_declaration.wrap_ok()
}

fn parse_client_pointer_target_type(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLTypeAnnotation>> {
    let keyword = tokens.parse_source_of_kind(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_TO,
    )?;

    if keyword.item != "to" {
        Diagnostic::new(
            "Expected the keyword `to`".to_string(),
            keyword.location.to::<Location>().wrap_some(),
        )
        .wrap_err()
    } else {
        parse_type_annotation(tokens)
    }
}

fn parse_client_pointer_declaration_inner(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
) -> DiagnosticResult<WithEmbeddedLocation<ClientPointerDeclaration>> {
    tokens.with_embedded_location_result(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_SERVER_OBJECT_TYPE,
            )
            .map(|with_embedded_location| with_embedded_location.map(EntityNameWrapper))?;

        let _dot = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period, semantic_token_legend::ST_DOT)?;

        let client_pointer_name: WithEmbeddedLocation<SelectableName> = tokens
            .parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_CLIENT_SELECTABLE_NAME,
            )?;

        let variable_definitions = parse_variable_definitions(tokens)?;

        let target_type = parse_client_pointer_target_type(tokens)?
            .map(TypeAnnotationDeclaration::from_graphql_type_annotation);

        let directives = parse_directives(tokens)?;

        let description = parse_optional_description(tokens);

        let selection_set = parse_optional_selection_set(tokens)?.ok_or_else(|| {
            expected_selection_set_diagnostic(Location::new(
                tokens.text_source,
                // TODO get a real span!
                Span::todo_generated(),
            ))
        })?;

        let const_export_name = const_export_name.ok_or_else(|| {
            expected_literal_to_be_exported_diagnostic(
                "pointer",
                client_pointer_name.item,
                client_pointer_name.location.into(),
            )
        })?;

        ClientPointerDeclaration {
            directives,
            parent_type,
            client_pointer_name: client_pointer_name.map(|x| x.into()),
            target_type,
            description,
            selection_set,
            definition_path: definition_file_path,
            const_export_name: const_export_name.intern().into(),
            variable_definitions,

            semantic_tokens: tokens.semantic_tokens(),
        }
        .wrap_ok()
    })
}

// Note: for now, top-level selection sets are required
fn parse_optional_selection_set(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<Option<WithEmbeddedLocation<SelectionSet>>> {
    tokens.with_embedded_location_optional_result(|tokens| {
        let selections = parse_optional_selection_set_inner(tokens)?;
        selections
            .map(|selections| SelectionSet { selections })
            .wrap_ok()
    })
}

// TODO this should not parse an optional selection set, but a required one
fn parse_optional_selection_set_inner(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<Option<Vec<WithEmbeddedLocation<Selection>>>> {
    let open_brace: DiagnosticResult<WithEmbeddedLocation<IsographLangTokenKind>> = tokens
        .parse_token_of_kind(
            IsographLangTokenKind::OpenBrace,
            semantic_token_legend::ST_OPEN_BRACE,
        );
    if open_brace.is_err() {
        return Ok(None);
    }

    let mut selections = vec![];
    while tokens
        .parse_token_of_kind(
            IsographLangTokenKind::CloseBrace,
            semantic_token_legend::ST_CLOSE_BRACE,
        )
        .is_err()
    {
        let selection = parse_selection(tokens)?;
        selections.push(selection);
    }
    selections.wrap_some().wrap_ok()
}

/// Parse a list with a delimiter. Expect an optional final delimiter.
fn parse_delimited_list<'a, TResult>(
    tokens: &mut PeekableLexer<'a>,
    parse_item: impl Fn(&mut PeekableLexer<'a>) -> DiagnosticResult<TResult> + 'a,
    parse_delimiter: impl Fn(&mut PeekableLexer<'a>) -> DiagnosticResult<()> + 'a,
    closing_token: IsographLangTokenKind,
    closing_isograph_semantic_token: IsographSemanticToken,
) -> DiagnosticResult<WithEmbeddedLocation<Vec<TResult>>> {
    let mut items = vec![];

    // Handle empty list case
    if let Ok(end_span) = tokens.parse_token_of_kind(closing_token, closing_isograph_semantic_token)
    {
        return end_span.map(|_| items).wrap_ok();
    }

    loop {
        items.push(parse_item(tokens)?);

        if let Ok(end_span) =
            tokens.parse_token_of_kind(closing_token, closing_isograph_semantic_token)
        {
            return end_span.map(|_| items).wrap_ok();
        }

        parse_delimiter(tokens)?;

        // Check if the next token is the closing token (allows for trailing delimiter)
        if let Ok(end_span) =
            tokens.parse_token_of_kind(closing_token, closing_isograph_semantic_token)
        {
            return end_span.map(|_| items).wrap_ok();
        }
    }
}

fn parse_line_break(tokens: &mut PeekableLexer<'_>) -> DiagnosticResult<()> {
    if tokens.source(tokens.white_space_span()).contains('\n') {
        Ok(())
    } else {
        Diagnostic::new(
            "Expected a line break.".to_string(),
            tokens.peek().location.to::<Location>().wrap_some(),
        )
        .wrap_err()
    }
}

fn parse_comma(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<IsographLangTokenKind>> {
    tokens.parse_token_of_kind(
        IsographLangTokenKind::Comma,
        semantic_token_legend::ST_COMMA,
    )
}

/// Parse at least one of comma or line break
fn parse_comma_or_line_break(tokens: &mut PeekableLexer<'_>) -> DiagnosticResult<()> {
    let parsed_comma_or_line_break =
        parse_comma(tokens).is_ok() || parse_line_break(tokens).is_ok();

    if parsed_comma_or_line_break {
        ().wrap_ok()
    } else {
        Diagnostic::new(
            "Expected comma or line break".to_string(),
            tokens.peek().location.to::<Location>().wrap_some(),
        )
        .wrap_err()
    }
}

fn parse_selection(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<Selection>> {
    tokens.with_embedded_location_result(|tokens| {
        // Special case periods here, in order to emit a good error
        if let Some(location) = parse_up_to_three_dots(tokens) {
            return fragment_spread_diagnostic(location).wrap_err();
        }

        let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;

        let arguments = parse_optional_arguments(tokens)?;

        let directives = parse_directives(tokens)?;

        // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
        let selection_set = parse_optional_selection_set(tokens)?;

        parse_comma_or_line_break(tokens)?;

        match selection_set {
            Some(selection_set) => {
                let object_selection_directive_set = from_isograph_field_directives(&directives)?;
                SelectionType::Object(ObjectSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    object_selection_directive_set,
                    selection_set,
                    arguments,
                })
            }
            None => {
                let scalar_selection_directive_set = from_isograph_field_directives(&directives)?;

                SelectionType::Scalar(ScalarSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    arguments,
                    scalar_selection_directive_set,
                })
            }
        }
        .wrap_ok()
    })
}

fn parse_optional_alias_and_field_name(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<(
    WithEmbeddedLocation<StringKey>,
    Option<WithEmbeddedLocation<StringKey>>,
)> {
    let field_name_or_alias = tokens.parse_string_key_type::<StringKey>(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_SELECTION_NAME_OR_ALIAS,
    )?;
    let colon = tokens.parse_token_of_kind(
        IsographLangTokenKind::Colon,
        semantic_token_legend::ST_COLON,
    );
    let (field_name, alias) = if colon.is_ok() {
        (
            tokens.parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_SELECTION_NAME_OR_ALIAS_POST_COLON,
            )?,
            field_name_or_alias.wrap_some(),
        )
    } else {
        (field_name_or_alias, None)
    };
    (field_name, alias).wrap_ok()
}

fn parse_directives(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<Vec<WithEmbeddedLocation<IsographFieldDirective>>>> {
    tokens
        .with_embedded_location_optional_result(|tokens| {
            let mut directives = vec![];
            while let Ok(token) = tokens.parse_token_of_kind(
                IsographLangTokenKind::At,
                semantic_token_legend::ST_DIRECTIVE_AT,
            ) {
                let name = tokens.parse_string_key_type(
                    IsographLangTokenKind::Identifier,
                    semantic_token_legend::ST_DIRECTIVE,
                )?;
                let directive_span = Span::join(token.location.span, name.location.span);

                let arguments = parse_optional_arguments(tokens)?;

                directives.push(
                    IsographFieldDirective { name, arguments }
                        .with_span(directive_span)
                        .to_with_embedded_location(tokens.text_source),
                );
            }

            if directives.is_empty() {
                None.wrap_ok::<Diagnostic>()
            } else {
                directives.wrap_some().wrap_ok()
            }
        })?
        .unwrap_or_else(|| {
            vec![]
                .with_generated_span()
                .to_with_embedded_location(tokens.text_source)
        })
        .wrap_ok()
}

fn parse_optional_arguments(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<SelectionFieldArgument>>> {
    if tokens
        .parse_token_of_kind(
            IsographLangTokenKind::OpenParen,
            semantic_token_legend::ST_OPEN_PAREN,
        )
        .is_ok()
    {
        let arguments = parse_delimited_list(
            tokens,
            parse_argument,
            parse_comma_or_line_break,
            IsographLangTokenKind::CloseParen,
            semantic_token_legend::ST_CLOSE_PAREN,
        )?
        .item;

        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_argument(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<SelectionFieldArgument>> {
    tokens.with_embedded_location_result(|tokens| {
        let name = tokens.parse_string_key_type(
            IsographLangTokenKind::Identifier,
            semantic_token_legend::ST_ARGUMENT_NAME,
        )?;
        tokens.parse_token_of_kind(
            IsographLangTokenKind::Colon,
            semantic_token_legend::ST_COLON,
        )?;
        let value = parse_non_constant_value(tokens)?;
        Ok(SelectionFieldArgument { name, value })
    })
}

fn parse_non_constant_value(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<NonConstantValue>> {
    from_control_flow(|| {
        to_control_flow::<_, Diagnostic>(|| {
            let _dollar_sign = tokens.parse_token_of_kind(
                IsographLangTokenKind::Dollar,
                semantic_token_legend::ST_VARIABLE_DOLLAR_USAGE,
            )?;

            let name: WithEmbeddedLocation<VariableName> = tokens.parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_VARIABLE,
            )?;
            let name = name.map(VariableNameWrapper::from);

            name.map(NonConstantValue::Variable).wrap_ok()
        })?;

        to_control_flow::<_, Diagnostic>(|| {
            let string = tokens
                .parse_source_of_kind(
                    IsographLangTokenKind::StringLiteral,
                    semantic_token_legend::ST_STRING_LITERAL,
                )
                .map(|parsed_str| {
                    parsed_str.map(|source_with_quotes| {
                        source_with_quotes[1..source_with_quotes.len() - 1]
                            .intern()
                            .into()
                    })
                })?;

            string.map(NonConstantValue::String).wrap_ok()
        })?;

        to_control_flow::<_, Diagnostic>(|| {
            let number = tokens.parse_source_of_kind(
                IsographLangTokenKind::IntegerLiteral,
                semantic_token_legend::ST_NUMBER_LITERAL,
            )?;
            number
                .map(|number| {
                    NonConstantValue::Integer(number.parse().expect("Expected valid integer"))
                })
                .wrap_ok()
        })?;

        to_control_flow::<_, Diagnostic>(|| {
            let open = tokens.parse_token_of_kind(
                IsographLangTokenKind::OpenBrace,
                semantic_token_legend::ST_OPEN_BRACE,
            )?;

            let entries = parse_delimited_list(
                tokens,
                parse_object_entry,
                parse_comma_or_line_break,
                IsographLangTokenKind::CloseBrace,
                semantic_token_legend::ST_CLOSE_BRACE,
            )?;

            NonConstantValue::Object(entries.item)
                .with_span(Span::join(open.location.span, entries.location.span))
                .to_with_embedded_location(tokens.text_source)
                .wrap_ok()
        })?;

        to_control_flow(|| {
            let bool_or_null = tokens.parse_source_of_kind(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_BOOL_OR_NULL,
            )?;

            let embedded_location = bool_or_null.location;

            bool_or_null.and_then(|bool_or_null| match bool_or_null {
                "null" => NonConstantValue::Null.wrap_ok(),
                bool => match bool.parse::<bool>() {
                    Ok(b) => NonConstantValue::Boolean(b).wrap_ok(),
                    Err(_) => Diagnostic::new(
                        "Expected null or a boolean value (true or false)".to_string(),
                        embedded_location.to::<Location>().wrap_some(),
                    )
                    .wrap_err(),
                },
            })
        })?;

        ControlFlow::Continue(Diagnostic::new(
            "Expected a valid value, like $foo, 42, \"bar\", true or false".to_string(),
            // TODO get location
            Location::Generated.wrap_some(),
        ))
    })
}

fn parse_object_entry(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<NameValuePair<ValueKeyName, NonConstantValue>> {
    let name = tokens.parse_string_key_type(
        IsographLangTokenKind::Identifier,
        semantic_token_legend::ST_OBJECT_LITERAL_KEY,
    )?;

    tokens.parse_token_of_kind(
        IsographLangTokenKind::Colon,
        semantic_token_legend::ST_COLON,
    )?;

    let value = parse_non_constant_value(tokens)?;

    NameValuePair { name, value }.wrap_ok()
}

fn parse_variable_definitions(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<VariableDeclaration>>> {
    if tokens
        .parse_token_of_kind(
            IsographLangTokenKind::OpenParen,
            semantic_token_legend::ST_OPEN_PAREN,
        )
        .is_ok()
    {
        parse_delimited_list(
            tokens,
            move |item| parse_variable_definition(item),
            parse_comma_or_line_break,
            IsographLangTokenKind::CloseParen,
            semantic_token_legend::ST_CLOSE_PAREN,
        )?
        .item
        .wrap_ok()
    } else {
        Ok(vec![])
    }
}

fn parse_variable_definition(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<VariableDeclaration>> {
    tokens
        .with_embedded_location_result(|tokens| {
            let _dollar = tokens.parse_token_of_kind(
                IsographLangTokenKind::Dollar,
                semantic_token_legend::ST_VARIABLE_DOLLAR_DECLARATION,
            )?;

            let name: WithEmbeddedLocation<VariableName> = tokens.parse_string_key_type(
                IsographLangTokenKind::Identifier,
                semantic_token_legend::ST_VARIABLE,
            )?;
            let name = name.map(VariableNameWrapper::from);

            tokens.parse_token_of_kind(
                IsographLangTokenKind::Colon,
                semantic_token_legend::ST_COLON,
            )?;
            let type_ = parse_type_annotation(tokens)?
                .map(TypeAnnotationDeclaration::from_graphql_type_annotation);

            let default_value = parse_optional_default_value(tokens)?;

            VariableDeclaration {
                name,
                type_,
                default_value,
            }
            .wrap_ok::<Diagnostic>()
        })?
        .wrap_ok()
}

fn parse_optional_default_value(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<Option<WithEmbeddedLocation<ConstantValue>>> {
    if tokens
        .parse_token_of_kind(
            IsographLangTokenKind::Equals,
            semantic_token_legend::ST_VARIABLE_EQUALS,
        )
        .is_ok()
    {
        let non_constant_value = parse_non_constant_value(tokens)?;
        let constant_value: ConstantValue = non_constant_value.item.try_into().map_err(|_| {
            Diagnostic::new(
                "Found a variable, like $foo, in a context where variables are not allowed"
                    .to_string(),
                non_constant_value.location.to::<Location>().wrap_some(),
            )
        })?;
        constant_value
            .with_location(non_constant_value.location)
            .wrap_some()
            .wrap_ok()
    } else {
        Ok(None)
    }
}

fn parse_type_annotation(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLTypeAnnotation>> {
    tokens.with_embedded_location_result(|tokens| {
        from_control_flow(|| {
            to_control_flow::<_, Diagnostic>(|| {
                let entity_name = tokens
                    .parse_string_key_type(
                        IsographLangTokenKind::Identifier,
                        semantic_token_legend::ST_TYPE_ANNOTATION,
                    )?
                    .item;

                let is_non_null = tokens
                    .parse_token_of_kind(
                        IsographLangTokenKind::Exclamation,
                        semantic_token_legend::ST_TYPE_ANNOTATION,
                    )
                    .is_ok();
                if is_non_null {
                    GraphQLTypeAnnotation::NonNull(
                        GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                            entity_name,
                        ))
                        .boxed(),
                    )
                    .wrap_ok()
                } else {
                    GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(entity_name)).wrap_ok()
                }
            })?;

            to_control_flow::<_, Diagnostic>(|| {
                // TODO: atomically parse everything here:
                tokens.parse_token_of_kind(
                    IsographLangTokenKind::OpenBracket,
                    semantic_token_legend::ST_TYPE_ANNOTATION,
                )?;

                let inner_type_annotation = parse_type_annotation(tokens)?;
                tokens.parse_token_of_kind(
                    IsographLangTokenKind::CloseBracket,
                    semantic_token_legend::ST_TYPE_ANNOTATION,
                )?;
                let is_non_null = tokens
                    .parse_token_of_kind(
                        IsographLangTokenKind::Exclamation,
                        semantic_token_legend::ST_TYPE_ANNOTATION,
                    )
                    .is_ok();

                if is_non_null {
                    GraphQLTypeAnnotation::NonNull(
                        GraphQLNonNullTypeAnnotation::List(GraphQLListTypeAnnotation(
                            inner_type_annotation,
                        ))
                        .boxed(),
                    )
                    .wrap_ok()
                } else {
                    GraphQLTypeAnnotation::List(
                        GraphQLListTypeAnnotation(inner_type_annotation).boxed(),
                    )
                    .wrap_ok()
                }
            })?;

            // One **cannot** add additional cases here (though of course none exist in the spec.)
            // Because, if we successfully parse the OpenBracket for a list type, we must parse the
            // entirety of the list type. Otherwise, we will have eaten the OpenBracket and will
            // leave the parser in an inconsistent state.
            //
            // We don't get a great error message with this current approach.

            ControlFlow::Continue(Diagnostic::new(
                "Expected a type (e.g. String, [String], or String!)".to_string(),
                tokens.peek().location.to::<Location>().wrap_some(),
            ))
        })
    })
}

fn to_control_flow<T, E>(result: impl FnOnce() -> Result<T, E>) -> ControlFlow<T, E> {
    match result() {
        Ok(t) => ControlFlow::Break(t),
        Err(e) => ControlFlow::Continue(e),
    }
}

fn from_control_flow<T, E>(control_flow: impl FnOnce() -> ControlFlow<T, E>) -> Result<T, E> {
    match control_flow() {
        ControlFlow::Break(t) => Ok(t),
        ControlFlow::Continue(e) => Err(e),
    }
}

fn leftover_tokens_diagnostic(location: Location) -> Diagnostic {
    Diagnostic::new(
        "Leftover tokens remaining".to_string(),
        location.wrap_some(),
    )
}

fn expected_selection_set_diagnostic(location: Location) -> Diagnostic {
    Diagnostic::new(
        "Selection sets are required. If you do not want to \
        select any fields, write an empty selection set: {{}}"
            .to_string(),
        location.wrap_some(),
    )
}

fn expected_literal_to_be_exported_diagnostic(
    literal_type: &str,
    suggested_const_export_name: SelectableName,
    location: Location,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "This isograph {literal_type} literal must be exported as a named export, for example \
            as `export const {suggested_const_export_name}`"
        ),
        location.wrap_some(),
    )
}

fn fragment_spread_diagnostic(location: EmbeddedLocation) -> Diagnostic {
    Diagnostic::new(
        "Unexpectedly found a period where a field should be selected.\n\
        - If you are attempting to spread a fragment, note that each client field and pointer \
        defines a field, which you can select directly. Instead of `...UserAvatar`, select `UserAvatar`.\n\
        - If you are attempting to refine to another type with the use of an inline fragment, \
        you can instead select an `asConcreteType` field. So, instead of `... on User {`, select \
        `asUser {`. These fields are optional, and will be non-null if the typename matches."
            .to_string(),
        location.to::<Location>().wrap_some(),
    )
}

// We can do better. We can attempt to parse `... on Foo` or `...Foo`
fn parse_up_to_three_dots(tokens: &mut PeekableLexer) -> Option<EmbeddedLocation> {
    tokens
        .with_embedded_location_result(|tokens| {
            tokens.parse_source_of_kind(
                IsographLangTokenKind::Period,
                semantic_token_legend::ST_DOT,
            )?;

            if tokens.peek().item == IsographLangTokenKind::Period {
                tokens.parse_source_of_kind(
                    IsographLangTokenKind::Period,
                    semantic_token_legend::ST_DOT,
                )?;

                if tokens.peek().item == IsographLangTokenKind::Period {
                    tokens.parse_source_of_kind(
                        IsographLangTokenKind::Period,
                        semantic_token_legend::ST_DOT,
                    )?;
                }
            }

            ().wrap_ok::<Diagnostic>()
        })
        .ok()
        .map(|x| x.location)
}
