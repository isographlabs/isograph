use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, IsoLiteralText, Location,
    RelativePathToSourceFile, Span, TextSource, UnvalidatedTypeName, ValueKeyName, WithLocation,
    WithSpan,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation, NameValuePair,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{
    from_isograph_field_directives, ClientFieldDeclaration, ClientPointerDeclaration,
    ConstantValue, EntrypointDeclaration, IsographFieldDirective, NonConstantValue,
    ObjectSelection, ScalarSelection, SelectionFieldArgument, SelectionTypeContainingSelections,
    UnvalidatedSelection, VariableDefinition,
};
use std::{collections::HashSet, ops::ControlFlow};

use crate::{
    parse_optional_description, IsographLangTokenKind, IsographLiteralParseError,
    ParseResultWithLocation, ParseResultWithSpan, PeekableLexer,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IsoLiteralExtractionResult {
    ClientPointerDeclaration(WithSpan<ClientPointerDeclaration>),
    ClientFieldDeclaration(WithSpan<ClientFieldDeclaration>),
    EntrypointDeclaration(WithSpan<EntrypointDeclaration>),
}

pub fn parse_iso_literal(
    iso_literal_text: &str,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> Result<IsoLiteralExtractionResult, WithLocation<IsographLiteralParseError>> {
    let mut tokens = PeekableLexer::new(iso_literal_text);
    let discriminator = tokens
        .parse_source_of_kind(IsographLangTokenKind::Identifier)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))
        .map_err(|err| err.to_with_location(text_source))?;
    match discriminator.item {
        "entrypoint" => Ok(IsoLiteralExtractionResult::EntrypointDeclaration(
            parse_iso_entrypoint_declaration(
                &mut tokens,
                text_source,
                discriminator.span,
                iso_literal_text.intern().into(),
            )?,
        )),
        "field" => Ok(IsoLiteralExtractionResult::ClientFieldDeclaration(
            parse_iso_client_field_declaration(
                &mut tokens,
                definition_file_path,
                const_export_name,
                text_source,
            )?,
        )),
        "pointer" => Ok(IsoLiteralExtractionResult::ClientPointerDeclaration(
            parse_iso_client_pointer_declaration(
                &mut tokens,
                definition_file_path,
                const_export_name,
                text_source,
            )?,
        )),
        _ => Err(WithLocation::new(
            IsographLiteralParseError::ExpectedFieldOrPointerOrEntrypoint,
            Location::new(text_source, discriminator.span),
        )),
    }
}

fn parse_iso_entrypoint_declaration(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
    entrypoint_keyword: Span,
    iso_literal_text: IsoLiteralText,
) -> ParseResultWithLocation<WithSpan<EntrypointDeclaration>> {
    let entrypoint_declaration = tokens
        .with_span(|tokens| {
            let parent_type = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let dot = tokens
                .parse_token_of_kind(IsographLangTokenKind::Period)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let client_field_name = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let directives = parse_directives(tokens, text_source)?;

            let entrypoint_directive_set =
                from_isograph_field_directives(&directives).map_err(|message| {
                    WithSpan::new(
                        IsographLiteralParseError::UnableToDeserializeDirectives { message },
                        directives
                            .first()
                            .map(|x| x.span)
                            .unwrap_or_else(Span::todo_generated),
                    )
                })?;
            Ok(EntrypointDeclaration {
                parent_type,
                client_field_name,
                iso_literal_text,
                entrypoint_keyword: WithSpan::new((), entrypoint_keyword),
                dot: dot.map(|_| ()),
                entrypoint_directive_set,
                semantic_tokens: tokens.semantic_tokens(),
            })
        })
        .map_err(|with_span: WithSpan<_>| with_span.to_with_location(text_source))?;

    if let Some(span) = tokens.remaining_token_span() {
        return Err(WithLocation::new(
            IsographLiteralParseError::LeftoverTokens,
            Location::new(text_source, span),
        ));
    }

    Ok(entrypoint_declaration)
}

fn parse_iso_client_field_declaration(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithLocation<WithSpan<ClientFieldDeclaration>> {
    let client_field_declaration = parse_client_field_declaration_inner(
        tokens,
        definition_file_path,
        const_export_name,
        text_source,
    )
    .map_err(|with_span| with_span.to_with_location(text_source))?;

    if let Some(span) = tokens.remaining_token_span() {
        return Err(WithLocation::new(
            IsographLiteralParseError::LeftoverTokens,
            Location::new(text_source, span),
        ));
    }

    Ok(client_field_declaration)
}

fn parse_client_field_declaration_inner(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<ClientFieldDeclaration>> {
    tokens.with_span(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let _ = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let client_field_name: WithSpan<ClientScalarSelectableName> = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let variable_definitions = parse_variable_definitions(tokens, text_source)?;

        let directives = parse_directives(tokens, text_source)?;

        let client_field_directive_set =
            from_isograph_field_directives(&directives).map_err(|message| {
                WithSpan::new(
                    IsographLiteralParseError::UnableToDeserializeDirectives { message },
                    directives
                        .first()
                        .map(|x| x.span)
                        .unwrap_or_else(Span::todo_generated),
                )
            })?;

        let description = parse_optional_description(tokens);

        let selection_set = parse_selection_set(tokens, text_source)?;

        let const_export_name = const_export_name.ok_or_else(|| {
            WithSpan::new(
                IsographLiteralParseError::ExpectedLiteralToBeExported {
                    literal_type: "field".to_string(),
                    suggested_const_export_name: client_field_name.item.into(),
                },
                Span::todo_generated(),
            )
        })?;

        Ok(ClientFieldDeclaration {
            parent_type,
            client_field_name,
            description,
            selection_set,
            definition_path: definition_file_path,
            client_field_directive_set,
            const_export_name: const_export_name.intern().into(),
            variable_definitions,

            semantic_tokens: tokens.semantic_tokens(),
        })
    })
}

fn parse_iso_client_pointer_declaration(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithLocation<WithSpan<ClientPointerDeclaration>> {
    let client_pointer_declaration = parse_client_pointer_declaration_inner(
        tokens,
        definition_file_path,
        const_export_name,
        text_source,
    )
    .map_err(|with_span| with_span.to_with_location(text_source))?;

    if let Some(span) = tokens.remaining_token_span() {
        return Err(WithLocation::new(
            IsographLiteralParseError::LeftoverTokens,
            Location::new(text_source, span),
        ));
    }

    Ok(client_pointer_declaration)
}

fn parse_client_pointer_target_type(
    tokens: &mut PeekableLexer<'_>,
) -> ParseResultWithSpan<GraphQLTypeAnnotation<UnvalidatedTypeName>> {
    let keyword = tokens
        .parse_source_of_kind(IsographLangTokenKind::Identifier)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

    if keyword.item != "to" {
        return Err(WithSpan::new(
            IsographLiteralParseError::ExpectedTo,
            keyword.span,
        ));
    }

    parse_type_annotation(tokens)
}

fn parse_client_pointer_declaration_inner(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<ClientPointerDeclaration>> {
    tokens.with_span(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let _dot = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let client_pointer_name: WithSpan<ClientObjectSelectableName> = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let target_type = parse_client_pointer_target_type(tokens)?;

        let variable_definitions = parse_variable_definitions(tokens, text_source)?;

        let directives = parse_directives(tokens, text_source)?;

        let description = parse_optional_description(tokens);

        let selection_set = parse_selection_set(tokens, text_source)?;

        let const_export_name = const_export_name.ok_or_else(|| {
            WithSpan::new(
                IsographLiteralParseError::ExpectedLiteralToBeExported {
                    literal_type: "pointer".to_string(),
                    suggested_const_export_name: client_pointer_name.item.into(),
                },
                Span::todo_generated(),
            )
        })?;

        Ok(ClientPointerDeclaration {
            directives,
            parent_type,
            client_pointer_name,
            target_type,
            description,
            selection_set,
            definition_path: definition_file_path,
            const_export_name: const_export_name.intern().into(),
            variable_definitions,

            semantic_tokens: tokens.semantic_tokens(),
        })
    })
}

// Note: for now, top-level selection sets are required
fn parse_selection_set(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<Vec<WithSpan<UnvalidatedSelection>>> {
    let selection_set = parse_optional_selection_set(tokens, text_source)?;
    match selection_set {
        Some(selection_set) => Ok(selection_set),
        None => Err(WithSpan::new(
            IsographLiteralParseError::ExpectedSelectionSet,
            Span::new(0, 0),
        )),
    }
}

// TODO this should not parse an optional selection set, but a required one
fn parse_optional_selection_set(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<Option<Vec<WithSpan<UnvalidatedSelection>>>> {
    let open_brace: Result<WithSpan<IsographLangTokenKind>, WithSpan<crate::LowLevelParseError>> =
        tokens.parse_token_of_kind(IsographLangTokenKind::OpenBrace);
    if open_brace.is_err() {
        return Ok(None);
    }

    let mut encountered_names_or_aliases = HashSet::new();
    let mut selections = vec![];
    while tokens
        .parse_token_of_kind(IsographLangTokenKind::CloseBrace)
        .is_err()
    {
        let selection = parse_selection(tokens, text_source)?;
        let selection_name_or_alias = selection.item.name_or_alias().item;
        if !encountered_names_or_aliases.insert(selection_name_or_alias) {
            // We have already encountered this name or alias, so we emit
            // an error.
            // TODO should SelectionSet be a HashMap<SelectableNameOrAlias, ...> instead of
            // a Vec??
            // TODO find a way to include the location of the previous field with matching
            // name or alias
            return Err(WithSpan::new(
                IsographLiteralParseError::DuplicateNameOrAlias {
                    name_or_alias: selection_name_or_alias,
                },
                selection.span,
            ));
        }
        selections.push(selection);
    }
    Ok(Some(selections))
}

/// Parse a list with a delimiter. Expect an optional final delimiter.
fn parse_delimited_list<'a, TResult>(
    tokens: &mut PeekableLexer<'a>,
    parse_item: impl Fn(&mut PeekableLexer<'a>) -> ParseResultWithSpan<TResult> + 'a,
    delimiter: IsographLangTokenKind,
    closing_token: IsographLangTokenKind,
) -> ParseResultWithSpan<WithSpan<Vec<TResult>>> {
    let mut items = vec![];

    // Handle empty list case
    if let Ok(end_span) = tokens.parse_token_of_kind(closing_token) {
        return Ok(end_span.map(|_| items));
    }

    loop {
        items.push(parse_item(tokens)?);

        if let Ok(end_span) = tokens.parse_token_of_kind(closing_token) {
            return Ok(end_span.map(|_| items));
        }

        if tokens.parse_token_of_kind(delimiter).is_err() {
            return Err(WithSpan::new(
                IsographLiteralParseError::ExpectedDelimiterOrClosingToken {
                    closing_token,
                    delimiter,
                },
                tokens.peek().span,
            ));
        }

        // Check if the next token is the closing token (allows for trailing delimiter)
        if let Ok(end_span) = tokens.parse_token_of_kind(closing_token) {
            return Ok(end_span.map(|_| items));
        }
    }
}

fn parse_comma_line_break_or_curly(tokens: &mut PeekableLexer<'_>) -> ParseResultWithSpan<()> {
    let comma = tokens.parse_token_of_kind(IsographLangTokenKind::Comma);
    if comma.is_ok()
        || tokens.source(tokens.white_space_span()).contains('\n')
        || matches!(tokens.peek().item, IsographLangTokenKind::CloseBrace)
    {
        Ok(())
    } else {
        Err(WithSpan::new(
            IsographLiteralParseError::ExpectedCommaOrLineBreak,
            tokens.peek().span,
        ))
    }
}

fn parse_selection(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<UnvalidatedSelection>> {
    tokens.with_span(|tokens| {
        let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;
        let field_name = field_name.to_with_location(text_source);
        let alias = alias.map(|alias| alias.to_with_location(text_source));

        let arguments = parse_optional_arguments(tokens, text_source)?;

        let directives = parse_directives(tokens, text_source)?;

        // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
        let selection_set = parse_optional_selection_set(tokens, text_source)?;

        parse_comma_line_break_or_curly(tokens)?;

        let selection = match selection_set {
            Some(selection_set) => {
                let object_selection_directive_set = from_isograph_field_directives(&directives)
                    .map_err(|message| {
                        WithSpan::new(
                            IsographLiteralParseError::UnableToDeserializeDirectives { message },
                            directives
                                .first()
                                .map(|x| x.span)
                                .unwrap_or_else(Span::todo_generated),
                        )
                    })?;
                SelectionTypeContainingSelections::Object(ObjectSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    object_selection_directive_set,
                    selection_set,
                    arguments,
                    associated_data: (),
                })
            }
            None => {
                let scalar_selection_directive_set = from_isograph_field_directives(&directives)
                    .map_err(|message| {
                        WithSpan::new(
                            IsographLiteralParseError::UnableToDeserializeDirectives { message },
                            directives
                                .first()
                                .map(|x| x.span)
                                .unwrap_or_else(Span::todo_generated),
                        )
                    })?;
                SelectionTypeContainingSelections::Scalar(ScalarSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    associated_data: (),
                    arguments,
                    scalar_selection_directive_set,
                })
            }
        };
        Ok(selection)
    })
}

fn parse_optional_alias_and_field_name(
    tokens: &mut PeekableLexer,
) -> ParseResultWithSpan<(WithSpan<StringKey>, Option<WithSpan<StringKey>>)> {
    let field_name_or_alias = tokens
        .parse_string_key_type::<StringKey>(IsographLangTokenKind::Identifier)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
    let colon = tokens.parse_token_of_kind(IsographLangTokenKind::Colon);
    let (field_name, alias) = if colon.is_ok() {
        (
            tokens
                .parse_string_key_type::<StringKey>(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?,
            Some(field_name_or_alias),
        )
    } else {
        (field_name_or_alias, None)
    };
    Ok((field_name, alias))
}

fn parse_directives(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResultWithSpan<Vec<WithSpan<IsographFieldDirective>>> {
    let mut directives = vec![];
    while let Ok(token) = tokens.parse_token_of_kind(IsographLangTokenKind::At) {
        let name = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        let directive_span = Span::join(token.span, name.span);

        let arguments = parse_optional_arguments(tokens, text_source)?;

        directives.push(WithSpan::new(
            IsographFieldDirective { name, arguments },
            directive_span,
        ));
    }
    Ok(directives)
}

fn parse_optional_arguments(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResultWithSpan<Vec<WithLocation<SelectionFieldArgument>>> {
    if tokens
        .parse_token_of_kind(IsographLangTokenKind::OpenParen)
        .is_ok()
    {
        let arguments = parse_delimited_list(
            tokens,
            move |tokens| parse_argument(tokens, text_source),
            IsographLangTokenKind::Comma,
            IsographLangTokenKind::CloseParen,
        )?
        .item;

        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_argument(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithLocation<SelectionFieldArgument>> {
    let argument = tokens.with_span(|tokens| {
        let name = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        tokens
            .parse_token_of_kind(IsographLangTokenKind::Colon)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        let value = parse_non_constant_value(tokens, text_source)?.to_with_location(text_source);
        Ok::<_, WithSpan<IsographLiteralParseError>>(SelectionFieldArgument { name, value })
    })?;
    Ok(argument.to_with_location(text_source))
}

fn parse_non_constant_value(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<NonConstantValue>> {
    from_control_flow(|| {
        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let _dollar_sign = tokens
                .parse_token_of_kind(IsographLangTokenKind::Dollar)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let name = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            Ok(name.map(NonConstantValue::Variable))
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let string = tokens
                .parse_source_of_kind(IsographLangTokenKind::StringLiteral)
                .map(|parsed_str| {
                    parsed_str.map(|source_with_quotes| {
                        source_with_quotes[1..source_with_quotes.len() - 1]
                            .intern()
                            .into()
                    })
                })
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            Ok(string.map(NonConstantValue::String))
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let number = tokens
                .parse_source_of_kind(IsographLangTokenKind::IntegerLiteral)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            Ok(number.map(|number| {
                NonConstantValue::Integer(number.parse().expect("Expected valid integer"))
            }))
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let open = tokens
                .parse_token_of_kind(IsographLangTokenKind::OpenBrace)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let entries = parse_delimited_list(
                tokens,
                move |tokens| parse_object_entry(tokens, text_source),
                IsographLangTokenKind::Comma,
                IsographLangTokenKind::CloseBrace,
            )?;

            Ok(WithSpan::new(
                NonConstantValue::Object(entries.item),
                Span {
                    start: open.span.start,
                    end: entries.span.end,
                },
            ))
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let bool_or_null = tokens
                .parse_source_of_kind(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let span = bool_or_null.span;

            bool_or_null.and_then(|bool_or_null| match bool_or_null {
                "null" => Ok(NonConstantValue::Null),
                bool => match bool.parse::<bool>() {
                    Ok(b) => Ok(NonConstantValue::Boolean(b)),
                    Err(_) => Err(WithSpan::new(
                        IsographLiteralParseError::ExpectedBoolean,
                        span,
                    )),
                },
            })
        })?;

        ControlFlow::Continue(WithSpan::new(
            IsographLiteralParseError::ExpectedNonConstantValue,
            Span::todo_generated(),
        ))
    })
}

fn parse_object_entry(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResultWithSpan<NameValuePair<ValueKeyName, NonConstantValue>> {
    let name = tokens
        .parse_string_key_type(IsographLangTokenKind::Identifier)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?
        .to_with_location(text_source);

    tokens
        .parse_token_of_kind(IsographLangTokenKind::Colon)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

    let value = parse_non_constant_value(tokens, text_source)?.to_with_location(text_source);

    Ok(NameValuePair { name, value })
}

fn parse_variable_definitions(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResultWithSpan<Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>> {
    if tokens
        .parse_token_of_kind(IsographLangTokenKind::OpenParen)
        .is_ok()
    {
        let variable_definitions = parse_delimited_list(
            tokens,
            move |item| parse_variable_definition(item, text_source),
            IsographLangTokenKind::Comma,
            IsographLangTokenKind::CloseParen,
        )?
        .item;

        Ok(variable_definitions)
    } else {
        Ok(vec![])
    }
}

fn parse_variable_definition(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<VariableDefinition<UnvalidatedTypeName>>> {
    let variable_definition = tokens.with_span(|tokens| {
        let _dollar = tokens
            .parse_token_of_kind(IsographLangTokenKind::Dollar)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        let name = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?
            .to_with_location(text_source);
        tokens
            .parse_token_of_kind(IsographLangTokenKind::Colon)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        let type_ = parse_type_annotation(tokens)?;

        let default_value = parse_optional_default_value(tokens, text_source)?;

        Ok::<_, WithSpan<IsographLiteralParseError>>(VariableDefinition {
            name,
            type_,
            default_value,
        })
    })?;
    Ok(variable_definition)
}

fn parse_optional_default_value(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<Option<WithLocation<ConstantValue>>> {
    if tokens
        .parse_token_of_kind(IsographLangTokenKind::Equals)
        .is_ok()
    {
        let non_constant_value = parse_non_constant_value(tokens, text_source)?;
        let constant_value: ConstantValue = non_constant_value.item.try_into().map_err(|_| {
            WithSpan::new(
                IsographLiteralParseError::UnexpectedVariable,
                non_constant_value.span,
            )
        })?;
        Ok(Some(WithLocation::new(
            constant_value,
            Location::new(text_source, non_constant_value.span),
        )))
    } else {
        Ok(None)
    }
}

fn parse_type_annotation(
    tokens: &mut PeekableLexer,
) -> ParseResultWithSpan<GraphQLTypeAnnotation<UnvalidatedTypeName>> {
    from_control_flow(|| {
        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let type_ = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let is_non_null = tokens
                .parse_token_of_kind(IsographLangTokenKind::Exclamation)
                .is_ok();
            if is_non_null {
                Ok(GraphQLTypeAnnotation::NonNull(Box::new(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(type_)),
                )))
            } else {
                Ok(GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                    type_,
                )))
            }
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            // TODO: atomically parse everything here:
            tokens
                .parse_token_of_kind(IsographLangTokenKind::OpenBracket)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let inner_type_annotation = parse_type_annotation(tokens)?;
            tokens
                .parse_token_of_kind(IsographLangTokenKind::CloseBracket)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let is_non_null = tokens
                .parse_token_of_kind(IsographLangTokenKind::Exclamation)
                .is_ok();

            if is_non_null {
                Ok(GraphQLTypeAnnotation::NonNull(Box::new(
                    GraphQLNonNullTypeAnnotation::List(GraphQLListTypeAnnotation(
                        inner_type_annotation,
                    )),
                )))
            } else {
                Ok(GraphQLTypeAnnotation::List(Box::new(
                    GraphQLListTypeAnnotation(inner_type_annotation),
                )))
            }
        })?;

        // One **cannot** add additional cases here (though of course none exist in the spec.)
        // Because, if we successfully parse the OpenBracket for a list type, we must parse the
        // entirety of the list type. Otherwise, we will have eaten the OpenBracket and will
        // leave the parser in an inconsistent state.
        //
        // We don't get a great error message with this current approach.

        ControlFlow::Continue(WithSpan::new(
            IsographLiteralParseError::ExpectedTypeAnnotation,
            tokens.peek().span,
        ))
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

#[cfg(test)]
mod test {
    use crate::{IsographLangTokenKind, PeekableLexer};

    #[test]
    fn parse_literal_tests() {
        let source = "\"Description\" Query.foo { bar, baz, }";
        let mut lexer = PeekableLexer::new(source);

        loop {
            let token = lexer.parse_token();
            if token.item == IsographLangTokenKind::EndOfFile {
                break;
            }
        }
    }
}
