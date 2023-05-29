use boulton_lang_types::{
    FieldSelection, LinkedFieldSelection, ResolverDeclaration, ScalarFieldSelection, Selection,
    SelectionSetAndUnwraps, Unwrap,
};
use common_lang_types::{LinkedFieldName, ResolverDefinitionPath, ScalarFieldName, WithSpan};
use intern::string_key::StringKey;

use crate::{
    parse_optional_description, BoultonLangTokenKind, BoultonLiteralParseError, ParseResult,
    PeekableLexer,
};

pub fn parse_bdeclare_literal(
    b_declare_contents: &str,
    definition_file_path: ResolverDefinitionPath,
) -> ParseResult<WithSpan<ResolverDeclaration>> {
    let mut tokens = PeekableLexer::new(b_declare_contents);

    let resolver_declaration = parse_resolver_declaration(&mut tokens, definition_file_path)?;

    if !tokens.reached_eof() {
        return Err(BoultonLiteralParseError::LeftoverTokens {
            token: tokens.parse_token().item,
        });
    }

    // dbg!(Ok(resolver_declaration))
    Ok(resolver_declaration)
}

fn parse_resolver_declaration<'a>(
    tokens: &mut PeekableLexer<'a>,
    definition_file_path: ResolverDefinitionPath,
) -> ParseResult<WithSpan<ResolverDeclaration>> {
    let resolver_declaration = tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let parent_type = tokens
                .parse_string_key_type(BoultonLangTokenKind::Identifier)
                .map_err(|x| BoultonLiteralParseError::from(x))?;
            tokens.parse_token_of_kind(BoultonLangTokenKind::Period)?;
            let resolver_field_name = tokens
                .parse_string_key_type(BoultonLangTokenKind::Identifier)
                .map_err(|x| BoultonLiteralParseError::from(x))?;
            let selection_set_and_unwraps = parse_optional_selection_set_and_unwraps(tokens)?;

            Ok(ResolverDeclaration {
                description,
                parent_type,
                resolver_field_name,
                selection_set_and_unwraps,
                resolver_definition_path: definition_file_path,
            })
        })
        .transpose();
    resolver_declaration
}

fn parse_optional_selection_set_and_unwraps<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Option<SelectionSetAndUnwraps<ScalarFieldName, LinkedFieldName>>> {
    let selection_set = parse_optional_selection_set(tokens)?;
    match selection_set {
        Some(selection_set) => {
            let unwraps = parse_unwraps(tokens);
            Ok(Some(SelectionSetAndUnwraps {
                selection_set,
                unwraps,
            }))
        }
        None => Ok(None),
    }
}

fn parse_optional_selection_set<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Option<Vec<WithSpan<Selection<ScalarFieldName, LinkedFieldName>>>>> {
    let open_brace = tokens.parse_token_of_kind(BoultonLangTokenKind::OpenBrace);
    if open_brace.is_err() {
        return Ok(None);
    }

    let mut selections = vec![];
    while tokens
        .parse_token_of_kind(BoultonLangTokenKind::CloseBrace)
        .is_err()
    {
        selections.push(parse_selection(tokens)?);
    }
    Ok(Some(selections))
}

fn parse_selection<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<WithSpan<Selection<ScalarFieldName, LinkedFieldName>>> {
    tokens
        .with_span(|tokens| {
            let (field_name, alias) = parse_optional_alias_and_field_name(tokens)?;

            // TODO distinguish field groups

            // If we encounter a selection set, we are parsing a linked field. Otherwise, a scalar field.
            let selection_set = parse_optional_selection_set(tokens)?;

            let unwraps = parse_unwraps(tokens);

            // commas are required
            tokens.parse_token_of_kind(BoultonLangTokenKind::Comma)?;

            let selection = match selection_set {
                Some(selection_set) => {
                    Selection::Field(FieldSelection::LinkedField(LinkedFieldSelection {
                        alias: alias.map(|with_span| with_span.map(|string_key| string_key.into())),
                        field: field_name.map(|string_key| string_key.into()),
                        selection_set_and_unwraps: SelectionSetAndUnwraps {
                            unwraps,
                            selection_set,
                        },
                    }))
                }
                None => Selection::Field(FieldSelection::ScalarField(ScalarFieldSelection {
                    alias: alias.map(|with_span| with_span.map(|string_key| string_key.into())),
                    field: field_name.map(|string_key| string_key.into()),
                    unwraps,
                })),
            };
            Ok(selection)
        })
        .transpose()
}

fn parse_optional_alias_and_field_name(
    tokens: &mut PeekableLexer,
) -> Result<(WithSpan<StringKey>, Option<WithSpan<StringKey>>), BoultonLiteralParseError> {
    let field_name_or_alias = tokens
        .parse_string_key_type::<StringKey>(BoultonLangTokenKind::Identifier)
        .map_err(|x| BoultonLiteralParseError::from(x))?;
    let colon = tokens.parse_token_of_kind(BoultonLangTokenKind::Colon);
    let (field_name, alias) = if colon.is_ok() {
        (
            tokens.parse_string_key_type::<StringKey>(BoultonLangTokenKind::Identifier)?,
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
    while let Ok(token) = tokens.parse_token_of_kind(BoultonLangTokenKind::Exclamation) {
        unwraps.push(token.map(|_| Unwrap::ActualUnwrap))
    }
    unwraps
}

#[cfg(test)]
mod test {
    use crate::{BoultonLangTokenKind, PeekableLexer};

    #[test]
    fn parse_literal_tests() {
        let source = "\"Description\" Query.foo { bar, baz, }";
        let mut lexer = PeekableLexer::new(source);

        loop {
            let token = lexer.parse_token();
            eprintln!("found token {}", token);
            if token.item == BoultonLangTokenKind::EndOfFile {
                break;
            }
        }
    }
}
