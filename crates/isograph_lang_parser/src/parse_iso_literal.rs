use std::{collections::HashSet, ops::ControlFlow};

use common_lang_types::{
    FilePath, Location, ScalarFieldName, Span, TextSource, UnvalidatedTypeName, WithLocation,
    WithSpan,
};
use graphql_lang_types::{
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLTypeAnnotation,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{
    ClientFieldDeclaration, ClientFieldDeclarationWithUnvalidatedDirectives, ConstantValue,
    EntrypointTypeAndField, IsographFieldDirective, LinkedFieldSelection, NonConstantValue,
    ScalarFieldSelection, Selection, SelectionFieldArgument, ServerFieldSelection,
    UnvalidatedSelectionWithUnvalidatedDirectives, Unwrap, VariableDefinition,
};

use crate::{
    parse_optional_description, IsographLangTokenKind, IsographLiteralParseError,
    ParseResultWithLocation, ParseResultWithSpan, PeekableLexer,
};

pub enum IsoLiteralExtractionResult {
    ClientFieldDeclaration(WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>),
    EntrypointDeclaration(WithSpan<EntrypointTypeAndField>),
}

pub fn parse_iso_literal(
    iso_literal_text: &str,
    definition_file_path: FilePath,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> Result<IsoLiteralExtractionResult, WithLocation<IsographLiteralParseError>> {
    let mut tokens = PeekableLexer::new(iso_literal_text, text_source);
    let discriminator = tokens
        .parse_source_of_kind(IsographLangTokenKind::Identifier)
        .map_err(|with_span| with_span.map(IsographLiteralParseError::from))
        .map_err(|err| err.to_with_location(text_source))?;
    match discriminator.item {
        "entrypoint" => Ok(IsoLiteralExtractionResult::EntrypointDeclaration(
            parse_iso_entrypoint_declaration(&mut tokens, text_source, discriminator.span)?,
        )),
        "field" => Ok(IsoLiteralExtractionResult::ClientFieldDeclaration(
            parse_iso_client_field_declaration(
                &mut tokens,
                definition_file_path,
                const_export_name,
                text_source,
                discriminator.span,
            )?,
        )),
        _ => Err(WithLocation::new(
            IsographLiteralParseError::ExpectedFieldOrEntrypoint,
            Location::new(text_source, discriminator.span),
        )),
    }
}

fn parse_iso_entrypoint_declaration(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
    entrypoint_keyword: Span,
) -> ParseResultWithLocation<WithSpan<EntrypointTypeAndField>> {
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

            Ok(EntrypointTypeAndField {
                parent_type,
                client_field_name,
                entrypoint_keyword: WithSpan::new((), entrypoint_keyword),
                dot: dot.map(|_| ()),
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
    definition_file_path: FilePath,
    const_export_name: Option<&str>,
    text_source: TextSource,
    field_keyword_span: Span,
) -> ParseResultWithLocation<WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>> {
    let client_field_declaration = parse_client_field_declaration_inner(
        tokens,
        definition_file_path,
        const_export_name,
        text_source,
        field_keyword_span,
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
    definition_file_path: FilePath,
    const_export_name: Option<&str>,
    text_source: TextSource,
    field_keyword_span: Span,
) -> ParseResultWithSpan<WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>> {
    tokens.with_span(|tokens| {
        let parent_type = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let dot = tokens
            .parse_token_of_kind(IsographLangTokenKind::Period)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let client_field_name: WithSpan<ScalarFieldName> = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

        let variable_definitions = parse_variable_definitions(tokens, text_source)?;

        let directives = parse_directives(tokens, text_source)?;

        let description = parse_optional_description(tokens);

        let (selection_set, unwraps) = parse_selection_set_and_unwraps(tokens, text_source)?;

        let const_export_name = const_export_name.ok_or_else(|| {
            WithSpan::new(
                IsographLiteralParseError::ExpectedLiteralToBeExported {
                    suggested_const_export_name: client_field_name.item,
                },
                Span::todo_generated(),
            )
        })?;

        // --------------------
        // TODO: use directives to:
        // - ensure only component exists
        // - it ends up in the reader AST
        // --------------------

        Ok(ClientFieldDeclaration {
            parent_type,
            client_field_name,
            description,
            selection_set,
            unwraps,
            definition_path: definition_file_path,
            directives,
            const_export_name: const_export_name.intern().into(),
            variable_definitions,
            field_keyword: WithSpan::new((), field_keyword_span),
            dot: dot.map(|_| ()),
        })
    })
}

// Note: for now, top-level selection sets are required
//
// TODO: perform some refactor to make type easier to read.
#[allow(clippy::type_complexity)]
fn parse_selection_set_and_unwraps(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<(
    Vec<WithSpan<UnvalidatedSelectionWithUnvalidatedDirectives>>,
    Vec<WithSpan<Unwrap>>,
)> {
    let selection_set = parse_optional_selection_set(tokens, text_source)?;
    match selection_set {
        Some(selection_set) => {
            let unwraps = parse_unwraps(tokens);
            Ok((selection_set, unwraps))
        }
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
) -> ParseResultWithSpan<Option<Vec<WithSpan<UnvalidatedSelectionWithUnvalidatedDirectives>>>> {
    let open_brace = tokens.parse_token_of_kind(IsographLangTokenKind::OpenBrace);
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
        match &selection.item {
            Selection::ServerField(server_field_selection) => {
                let selection_name_or_alias = server_field_selection.name_or_alias().item;
                if !encountered_names_or_aliases.insert(selection_name_or_alias) {
                    // We have already encountered this name or alias, so we emit
                    // an error.
                    // TODO should SelectionSet be a HashMap<FieldNameOrAlias, ...> instead of
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
            }
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
) -> ParseResultWithSpan<Vec<TResult>> {
    let mut items = vec![];

    // Handle empty list case
    if tokens.parse_token_of_kind(closing_token).is_ok() {
        return Ok(items);
    }

    loop {
        items.push(parse_item(tokens)?);

        if tokens.parse_token_of_kind(closing_token).is_ok() {
            break;
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
        if tokens.parse_token_of_kind(closing_token).is_ok() {
            break;
        }
    }

    Ok(items)
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
) -> ParseResultWithSpan<WithSpan<UnvalidatedSelectionWithUnvalidatedDirectives>> {
    tokens.with_span(|tokens| {
        let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;
        let field_name = field_name.to_with_location(text_source);
        let alias = alias.map(|alias| alias.to_with_location(text_source));

        // TODO distinguish field groups
        let arguments = parse_optional_arguments(tokens, text_source)?;

        // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
        let selection_set = parse_optional_selection_set(tokens, text_source)?;

        let unwraps = parse_unwraps(tokens);

        let directives = parse_directives(tokens, text_source)?;

        // commas are required
        parse_comma_line_break_or_curly(tokens)?;

        let selection = match selection_set {
            Some(selection_set) => {
                Selection::ServerField(ServerFieldSelection::LinkedField(LinkedFieldSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    associated_data: (),
                    selection_set,
                    unwraps,
                    arguments,
                    directives,
                }))
            }
            None => {
                Selection::ServerField(ServerFieldSelection::ScalarField(ScalarFieldSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    associated_data: (),
                    unwraps,
                    arguments,
                    directives,
                }))
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

fn parse_unwraps(tokens: &mut PeekableLexer) -> Vec<WithSpan<Unwrap>> {
    // TODO support _, etc.
    let mut unwraps = vec![];
    while let Ok(token) = tokens.parse_token_of_kind(IsographLangTokenKind::Exclamation) {
        unwraps.push(token.map(|_| Unwrap::ActualUnwrap))
    }
    unwraps
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
        )?;

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
        let value = parse_non_constant_value(tokens)?;
        Ok::<_, WithSpan<IsographLiteralParseError>>(SelectionFieldArgument { name, value })
    })?;
    Ok(argument.to_with_location(text_source))
}

fn parse_non_constant_value(
    tokens: &mut PeekableLexer,
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
            let number = tokens
                .parse_source_of_kind(IsographLangTokenKind::IntegerLiteral)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            Ok(number.map(|number| {
                NonConstantValue::Integer(number.parse().expect("Expected valid integer"))
            }))
        })?;

        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let bool = tokens
                .parse_source_of_kind(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let span = bool.span;

            bool.and_then(|bool| match bool.parse::<bool>() {
                Ok(b) => Ok(NonConstantValue::Boolean(b)),
                Err(_) => Err(WithSpan::new(
                    IsographLiteralParseError::ExpectedBoolean,
                    span,
                )),
            })
        })?;

        ControlFlow::Continue(WithSpan::new(
            IsographLiteralParseError::ExpectedNonConstantValue,
            Span::todo_generated(),
        ))
    })
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
        )?;

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
        let non_constant_value = parse_non_constant_value(tokens)?;
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
    use common_lang_types::TextSource;
    use intern::string_key::Intern;

    use crate::{IsographLangTokenKind, PeekableLexer};

    #[test]
    fn parse_literal_tests() {
        let source = "\"Description\" Query.foo { bar, baz, }";
        let mut lexer = PeekableLexer::new(
            source,
            TextSource {
                path: "path".intern().into(),
                span: None,
            },
        );

        loop {
            let token = lexer.parse_token();
            if token.item == IsographLangTokenKind::EndOfFile {
                break;
            }
        }
    }
}
