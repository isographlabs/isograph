use std::{collections::HashSet, ops::ControlFlow};

use common_lang_types::{
    Location, ResolverDefinitionPath, ScalarFieldName, SelectableFieldName, Span, StringKeyNewtype,
    TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    ListTypeAnnotation, NamedTypeAnnotation, NonNullTypeAnnotation, TypeAnnotation,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{
    EntrypointTypeAndField, FragmentDirectiveUsage, LinkedFieldSelection, NonConstantValue,
    ResolverDeclaration, ScalarFieldSelection, Selection, SelectionFieldArgument,
    ServerFieldSelection, UnvalidatedSelection, Unwrap, VariableDefinition,
};

use crate::{
    parse_optional_description, IsographLangTokenKind, IsographLiteralParseError,
    ParseResultWithLocation, ParseResultWithSpan, PeekableLexer,
};

pub enum IsoLiteralExtractionResult {
    ClientFieldDeclaration(WithSpan<ResolverDeclaration>),
    EntrypointDeclaration(WithSpan<EntrypointTypeAndField>),
}

pub fn parse_iso_literal(
    iso_literal_text: &str,
    definition_file_path: ResolverDefinitionPath,
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
            parse_iso_fetch(&mut tokens, text_source)?,
        )),
        "field" => Ok(IsoLiteralExtractionResult::ClientFieldDeclaration(
            parse_iso_client_field(
                &mut tokens,
                definition_file_path,
                const_export_name,
                text_source,
            )?,
        )),
        _ => Err(WithLocation::new(
            IsographLiteralParseError::ExpectedFieldOrEntrypoint,
            Location::new(text_source, discriminator.span),
        )),
    }
}

fn parse_iso_fetch(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithLocation<WithSpan<EntrypointTypeAndField>> {
    let resolver_fetch = tokens
        .with_span(|tokens| {
            let parent_type = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            tokens
                .parse_token_of_kind(IsographLangTokenKind::Period)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let resolver_field_name = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            Ok(EntrypointTypeAndField {
                parent_type,
                resolver_field_name,
            })
        })
        .transpose()
        .map_err(|with_span: WithSpan<_>| with_span.to_with_location(text_source))?;

    if let Some(span) = tokens.remaining_token_span() {
        return Err(WithLocation::new(
            IsographLiteralParseError::LeftoverTokens,
            Location::new(text_source, span),
        ));
    }

    Ok(resolver_fetch)
}

fn parse_iso_client_field(
    tokens: &mut PeekableLexer<'_>,
    definition_file_path: ResolverDefinitionPath,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithLocation<WithSpan<ResolverDeclaration>> {
    let resolver_declaration =
        parse_resolver_declaration(tokens, definition_file_path, const_export_name, text_source)
            .map_err(|with_span| with_span.to_with_location(text_source))?;

    if let Some(span) = tokens.remaining_token_span() {
        return Err(WithLocation::new(
            IsographLiteralParseError::LeftoverTokens,
            Location::new(text_source, span),
        ));
    }

    Ok(resolver_declaration)
}

fn parse_resolver_declaration<'a>(
    tokens: &mut PeekableLexer<'a>,
    definition_file_path: ResolverDefinitionPath,
    const_export_name: Option<&str>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<ResolverDeclaration>> {
    let resolver_declaration = tokens
        .with_span(|tokens| {
            let _description = parse_optional_description(tokens)?;
            let parent_type = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            tokens
                .parse_token_of_kind(IsographLangTokenKind::Period)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let resolver_field_name: WithSpan<ScalarFieldName> = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let variable_definitions = parse_variable_definitions(tokens, text_source)?;

            let directives = parse_directives(tokens)?;

            let selection_set_and_unwraps = parse_selection_set_and_unwraps(tokens, text_source)?;

            let const_export_name = const_export_name.ok_or_else(|| {
                WithSpan::new(
                    IsographLiteralParseError::ExpectedLiteralToBeExported {
                        suggested_const_export_name: resolver_field_name.item.into(),
                    },
                    Span::todo_generated(),
                )
            })?;

            // --------------------
            // TODO: use directives to:
            // - ensure only component exists
            // - it ends up in the reader AST
            // --------------------

            Ok(ResolverDeclaration {
                parent_type,
                resolver_field_name,
                selection_set_and_unwraps,
                resolver_definition_path: definition_file_path,
                directives,
                const_export_name: const_export_name.intern().into(),
                variable_definitions,
            })
        })
        .transpose();

    resolver_declaration
}

// Note: for now, top-level selection sets are required
fn parse_selection_set_and_unwraps<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResultWithSpan<Option<(Vec<WithSpan<UnvalidatedSelection>>, Vec<WithSpan<Unwrap>>)>> {
    let selection_set = parse_optional_selection_set(tokens, text_source)?;
    match selection_set {
        Some(selection_set) => {
            let unwraps = parse_unwraps(tokens);
            Ok(Some((selection_set, unwraps)))
        }
        None => Err(WithSpan::new(
            IsographLiteralParseError::ExpectedSelectionSet,
            Span::new(0, 0),
        )),
    }
}

fn parse_optional_selection_set<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResultWithSpan<Option<Vec<WithSpan<UnvalidatedSelection>>>> {
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
) -> ParseResultWithSpan<Vec<TResult>> {
    let mut items = vec![];
    items.push(parse_item(tokens)?);
    while tokens.parse_token_of_kind(delimiter).is_ok() {
        // Note: this is not ideal. parse_item can consume items off of the token stream, so
        // if it (for example) parsed a closing parentheses *then* errored, we would be in
        // an invalid state. In practice, this isn't an issue, but we should clean up the
        // code to not do this.
        let result = parse_item(tokens);
        if let Ok(result) = result {
            items.push(result);
        } else {
            break;
        }
    }
    Ok(items)
}

fn parse_comma_or_line_break<'a>(tokens: &mut PeekableLexer<'a>) -> ParseResultWithSpan<()> {
    let comma = tokens.parse_token_of_kind(IsographLangTokenKind::Comma);
    if comma.is_err() {
        let white_space_text = tokens.source(tokens.white_space_span());
        if white_space_text.contains('\n') {
            Ok(())
        } else {
            Err(WithSpan::new(
                IsographLiteralParseError::ExpectedCommaOrLineBreak,
                tokens.peek().span,
            ))
        }
    } else {
        Ok(())
    }
}

fn parse_selection<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<UnvalidatedSelection>> {
    tokens
        .with_span(|tokens| {
            let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;
            let field_name = field_name.to_with_location(text_source);
            let alias = alias.map(|alias| alias.to_with_location(text_source));

            // TODO distinguish field groups
            let arguments = parse_optional_arguments(tokens, text_source)?;

            // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
            let selection_set = parse_optional_selection_set(tokens, text_source)?;

            let unwraps = parse_unwraps(tokens);

            // commas are required
            parse_comma_or_line_break(tokens)?;

            let selection = match selection_set {
                Some(selection_set) => Selection::ServerField(ServerFieldSelection::LinkedField(
                    LinkedFieldSelection {
                        name: field_name.map(|string_key| string_key.into()),
                        reader_alias: alias
                            .map(|with_span| with_span.map(|string_key| string_key.into())),
                        associated_data: (),
                        selection_set,
                        unwraps,
                        normalization_alias:
                            HACK_combine_name_and_variables_into_normalization_alias(
                                field_name.map(|x| x.into()),
                                &arguments,
                            ),
                        arguments,
                    },
                )),
                None => Selection::ServerField(ServerFieldSelection::ScalarField(
                    ScalarFieldSelection {
                        name: field_name.map(|string_key| string_key.into()),
                        reader_alias: alias
                            .map(|with_span| with_span.map(|string_key| string_key.into())),
                        associated_data: (),
                        unwraps,
                        normalization_alias:
                            HACK_combine_name_and_variables_into_normalization_alias(
                                field_name.map(|x| x.into()),
                                &arguments,
                            ),
                        arguments,
                    },
                )),
            };
            Ok(selection)
        })
        .transpose()
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
) -> ParseResultWithSpan<Vec<WithSpan<FragmentDirectiveUsage>>> {
    let mut directives = vec![];
    while let Ok(token) = tokens.parse_token_of_kind(IsographLangTokenKind::At) {
        let name = tokens
            .parse_string_key_type(IsographLangTokenKind::Identifier)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        let directive_span = Span::join(token.span, name.span);
        directives.push(WithSpan::new(
            FragmentDirectiveUsage { name },
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
        )?;
        tokens
            .parse_token_of_kind(IsographLangTokenKind::CloseParen)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_argument(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithLocation<SelectionFieldArgument>> {
    let argument = tokens
        .with_span(|tokens| {
            let name = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            tokens
                .parse_token_of_kind(IsographLangTokenKind::Colon)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
            let value = parse_non_constant_value(tokens)?;
            Ok::<_, WithSpan<IsographLiteralParseError>>(SelectionFieldArgument { name, value })
        })
        .transpose()?;
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
        )?;
        tokens
            .parse_token_of_kind(IsographLangTokenKind::CloseParen)
            .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;
        Ok(variable_definitions)
    } else {
        Ok(vec![])
    }
}

fn parse_variable_definition(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResultWithSpan<WithSpan<VariableDefinition<UnvalidatedTypeName>>> {
    let variable_definition = tokens
        .with_span(|tokens| {
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

            Ok::<_, WithSpan<IsographLiteralParseError>>(VariableDefinition { name, type_ })
        })
        .transpose()?;
    Ok(variable_definition)
}

fn parse_type_annotation(
    tokens: &mut PeekableLexer,
) -> ParseResultWithSpan<TypeAnnotation<UnvalidatedTypeName>> {
    from_control_flow(|| {
        to_control_flow::<_, WithSpan<IsographLiteralParseError>>(|| {
            let type_ = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|with_span| with_span.map(IsographLiteralParseError::from))?;

            let is_non_null = tokens
                .parse_token_of_kind(IsographLangTokenKind::Exclamation)
                .is_ok();
            if is_non_null {
                Ok(TypeAnnotation::NonNull(Box::new(
                    NonNullTypeAnnotation::Named(NamedTypeAnnotation(type_)),
                )))
            } else {
                Ok(TypeAnnotation::Named(NamedTypeAnnotation(type_)))
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
                Ok(TypeAnnotation::NonNull(Box::new(
                    NonNullTypeAnnotation::List(ListTypeAnnotation(inner_type_annotation)),
                )))
            } else {
                Ok(TypeAnnotation::List(Box::new(ListTypeAnnotation(
                    inner_type_annotation,
                ))))
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

/// In order to avoid requiring a normalization AST, we write the variables
/// used in the alias. Once we have a normalization AST, we can remove this.
#[allow(non_snake_case)]
fn HACK_combine_name_and_variables_into_normalization_alias<T: StringKeyNewtype>(
    name: WithLocation<SelectableFieldName>,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> Option<WithLocation<T>> {
    if arguments.is_empty() {
        None
    } else {
        let mut alias_str = name.item.to_string();

        for argument in arguments {
            alias_str.push_str(&format!(
                // this will not be necessary once we have normalization ASTs
                "____{}",
                argument.item.to_alias_str_chunk()
            ));
        }
        Some(name.map(|_| T::from(alias_str.intern())))
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
