use std::ops::ControlFlow;

use common_lang_types::{
    FieldDefinitionName, ResolverDefinitionPath, Span, StringKeyNewtype, UnvalidatedTypeName,
    WithSpan,
};
use graphql_lang_types::{
    ListTypeAnnotation, NamedTypeAnnotation, NonNullTypeAnnotation, TypeAnnotation,
};
use intern::string_key::{Intern, StringKey};
use isograph_lang_types::{
    FieldSelection, FragmentDirectiveUsage, LinkedFieldSelection, NonConstantValue,
    ResolverDeclaration, ScalarFieldSelection, Selection, SelectionFieldArgument, Unwrap,
    VariableDefinition,
};

use crate::{
    parse_optional_description, IsographLangTokenKind, IsographLiteralParseError, ParseResult,
    PeekableLexer,
};

pub fn parse_iso_literal(
    b_declare_literal_text: &str,
    definition_file_path: ResolverDefinitionPath,
    has_associated_js_function: bool,
) -> ParseResult<WithSpan<ResolverDeclaration>> {
    let mut tokens = PeekableLexer::new(b_declare_literal_text);

    let resolver_declaration = parse_resolver_declaration(
        &mut tokens,
        definition_file_path,
        has_associated_js_function,
    )?;

    if !tokens.reached_eof() {
        return Err(IsographLiteralParseError::LeftoverTokens {
            token: tokens.parse_token().item,
        });
    }

    // dbg!(Ok(resolver_declaration))
    Ok(resolver_declaration)
}

fn parse_resolver_declaration<'a>(
    tokens: &mut PeekableLexer<'a>,
    definition_file_path: ResolverDefinitionPath,
    has_associated_js_function: bool,
) -> ParseResult<WithSpan<ResolverDeclaration>> {
    let resolver_declaration = tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let parent_type = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|x| IsographLiteralParseError::from(x))?;
            tokens.parse_token_of_kind(IsographLangTokenKind::Period)?;
            let resolver_field_name = tokens
                .parse_string_key_type(IsographLangTokenKind::Identifier)
                .map_err(|x| IsographLiteralParseError::from(x))?;

            let variable_definitions = parse_variable_definitions(tokens)?;

            let directives = parse_directives(tokens)?;

            let selection_set_and_unwraps = parse_optional_selection_set_and_unwraps(tokens)?;

            // --------------------
            // TODO: use directives to:
            // - ensure only component exists
            // - it ends up in the reader AST
            // --------------------

            Ok(ResolverDeclaration {
                description,
                parent_type,
                resolver_field_name,
                selection_set_and_unwraps,
                resolver_definition_path: definition_file_path,
                directives,
                variable_definitions,
                has_associated_js_function,
            })
        })
        .transpose();
    resolver_declaration
}

fn parse_optional_selection_set_and_unwraps<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Option<(Vec<WithSpan<Selection<(), ()>>>, Vec<WithSpan<Unwrap>>)>> {
    let selection_set = parse_optional_selection_set(tokens)?;
    match selection_set {
        Some(selection_set) => {
            let unwraps = parse_unwraps(tokens);
            Ok(Some((selection_set, unwraps)))
        }
        None => Ok(None),
    }
}

fn parse_optional_selection_set<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Option<Vec<WithSpan<Selection<(), ()>>>>> {
    let open_brace = tokens.parse_token_of_kind(IsographLangTokenKind::OpenBrace);
    if open_brace.is_err() {
        return Ok(None);
    }

    let mut selections = vec![];
    while tokens
        .parse_token_of_kind(IsographLangTokenKind::CloseBrace)
        .is_err()
    {
        selections.push(parse_selection(tokens)?);
    }
    Ok(Some(selections))
}

fn parse_selection<'a>(tokens: &mut PeekableLexer<'a>) -> ParseResult<WithSpan<Selection<(), ()>>> {
    tokens
        .with_span(|tokens| {
            let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;

            // TODO distinguish field groups
            let arguments = parse_optional_arguments(tokens)?;

            // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
            let selection_set = parse_optional_selection_set(tokens)?;

            let unwraps = parse_unwraps(tokens);

            // commas are required
            tokens.parse_token_of_kind(IsographLangTokenKind::Comma)?;

            let selection = match selection_set {
                Some(selection_set) => {
                    Selection::Field(FieldSelection::LinkedField(LinkedFieldSelection {
                        name: field_name.map(|string_key| string_key.into()),
                        reader_alias: alias
                            .map(|with_span| with_span.map(|string_key| string_key.into())),
                        field: (),
                        selection_set,
                        unwraps,
                        normalization_alias:
                            HACK_combine_name_and_variables_into_normalization_alias(
                                field_name.map(|x| x.into()),
                                &arguments,
                            ),
                        arguments,
                    }))
                }
                None => Selection::Field(FieldSelection::ScalarField(ScalarFieldSelection {
                    name: field_name.map(|string_key| string_key.into()),
                    reader_alias: alias
                        .map(|with_span| with_span.map(|string_key| string_key.into())),
                    field: (),
                    unwraps,
                    normalization_alias: HACK_combine_name_and_variables_into_normalization_alias(
                        field_name.map(|x| x.into()),
                        &arguments,
                    ),
                    arguments,
                })),
            };
            Ok(selection)
        })
        .transpose()
}

fn parse_optional_alias_and_field_name(
    tokens: &mut PeekableLexer,
) -> Result<(WithSpan<StringKey>, Option<WithSpan<StringKey>>), IsographLiteralParseError> {
    let field_name_or_alias = tokens
        .parse_string_key_type::<StringKey>(IsographLangTokenKind::Identifier)
        .map_err(|x| IsographLiteralParseError::from(x))?;
    let colon = tokens.parse_token_of_kind(IsographLangTokenKind::Colon);
    let (field_name, alias) = if colon.is_ok() {
        (
            tokens.parse_string_key_type::<StringKey>(IsographLangTokenKind::Identifier)?,
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
) -> ParseResult<Vec<WithSpan<FragmentDirectiveUsage>>> {
    let mut directives = vec![];
    while let Ok(token) = tokens.parse_token_of_kind(IsographLangTokenKind::At) {
        let name = tokens.parse_string_key_type(IsographLangTokenKind::Identifier)?;
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
) -> ParseResult<Vec<WithSpan<SelectionFieldArgument>>> {
    if tokens
        .parse_token_of_kind(IsographLangTokenKind::OpenParen)
        .is_ok()
    {
        let mut arguments = vec![];
        while tokens
            .parse_token_of_kind(IsographLangTokenKind::CloseParen)
            .is_err()
        {
            let argument = tokens
                .with_span(|tokens| {
                    let name = tokens.parse_string_key_type(IsographLangTokenKind::Identifier)?;
                    tokens.parse_token_of_kind(IsographLangTokenKind::Colon)?;
                    let value = parse_non_constant_value(tokens)?;
                    let _comma = tokens.parse_token_of_kind(IsographLangTokenKind::Comma)?;
                    Ok::<_, IsographLiteralParseError>(SelectionFieldArgument { name, value })
                })
                .transpose()?;
            arguments.push(argument);
        }
        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_non_constant_value(tokens: &mut PeekableLexer) -> ParseResult<WithSpan<NonConstantValue>> {
    // For now, we only support variables!
    let _dollar_sign = tokens.parse_token_of_kind(IsographLangTokenKind::Dollar)?;
    let name = tokens.parse_string_key_type(IsographLangTokenKind::Identifier)?;
    Ok(name.map(NonConstantValue::Variable))
}

fn parse_variable_definitions(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>> {
    if tokens
        .parse_token_of_kind(IsographLangTokenKind::OpenParen)
        .is_ok()
    {
        let mut variable_definitions = vec![];
        while tokens
            .parse_token_of_kind(IsographLangTokenKind::CloseParen)
            .is_err()
        {
            let variable_definition = tokens
                .with_span(|tokens| {
                    let _dollar = tokens.parse_token_of_kind(IsographLangTokenKind::Dollar)?;
                    let name = tokens.parse_string_key_type(IsographLangTokenKind::Identifier)?;
                    tokens.parse_token_of_kind(IsographLangTokenKind::Colon)?;
                    let type_ = parse_type_annotation(tokens)?;
                    let _comma = tokens.parse_token_of_kind(IsographLangTokenKind::Comma)?;
                    Ok::<_, IsographLiteralParseError>(VariableDefinition { name, type_ })
                })
                .transpose()?;
            variable_definitions.push(variable_definition);
        }
        Ok(variable_definitions)
    } else {
        Ok(vec![])
    }
}

fn parse_type_annotation(
    tokens: &mut PeekableLexer,
) -> ParseResult<TypeAnnotation<UnvalidatedTypeName>> {
    from_control_flow(|| {
        to_control_flow::<_, IsographLiteralParseError>(|| {
            let type_ = tokens.parse_string_key_type(IsographLangTokenKind::Identifier)?;

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

        to_control_flow::<_, IsographLiteralParseError>(|| {
            // TODO: atomically parse everything here:
            tokens.parse_token_of_kind(IsographLangTokenKind::OpenBracket)?;

            let inner_type_annotation = parse_type_annotation(tokens)?;
            tokens.parse_token_of_kind(IsographLangTokenKind::CloseBracket)?;
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

        ControlFlow::Continue(IsographLiteralParseError::ExpectedTypeAnnotation)
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
    name: WithSpan<FieldDefinitionName>,
    arguments: &[WithSpan<SelectionFieldArgument>],
) -> Option<WithSpan<T>> {
    if arguments.is_empty() {
        None
    } else {
        let mut alias_str = name.item.to_string();

        for argument in arguments {
            alias_str.push_str(&format!(
                "__{}_{}",
                argument.item.name.item,
                &argument.item.value.item.to_string()[1..]
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
