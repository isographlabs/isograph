use std::ops::ControlFlow;

use common_lang_types::{
    with_span_to_with_location, DescriptionValue, InterfaceTypeName, StringLiteralValue,
    TextSource, WithLocation, WithSpan,
};
use graphql_syntax::TokenKind;
use intern::{
    string_key::{Intern, StringKey},
    Lookup,
};

use graphql_lang_types::{
    ConstantValue, Directive, GraphQLInputObjectTypeDefinition, GraphQLInputValueDefinition,
    GraphQLInterfaceTypeDefinition, GraphQLObjectTypeDefinition, GraphQLOutputFieldDefinition,
    GraphQLScalarTypeDefinition, GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument,
    ListTypeAnnotation, NameValuePair, NamedTypeAnnotation, NonNullTypeAnnotation, TypeAnnotation,
    ValueType,
};

use crate::ParseResult;

use super::{
    description::parse_optional_description, peekable_lexer::PeekableLexer,
    schema_parse_error::SchemaParseError,
};

pub fn parse_schema(
    source: &str,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemDocument> {
    let mut tokens = PeekableLexer::new(source);

    parse_type_system_document(&mut tokens, text_source)
}

fn parse_type_system_document(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemDocument> {
    let mut type_system_definitions = vec![];
    while !tokens.reached_eof() {
        let type_system_definition = parse_type_system_definition(tokens, text_source)?;
        type_system_definitions.push(type_system_definition);
    }
    Ok(GraphQLTypeSystemDocument(type_system_definitions))
}

fn parse_type_system_definition(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemDefinition> {
    let description = parse_optional_description(tokens);
    let identifier = tokens
        .parse_token_of_kind(TokenKind::Identifier)
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;
    let identifier_source = tokens.source(identifier.span);

    match identifier_source {
        "type" => parse_object_type_definition(tokens, description, text_source)
            .map(GraphQLTypeSystemDefinition::from),
        "scalar" => parse_scalar_type_definition(tokens, description, text_source)
            .map(GraphQLTypeSystemDefinition::from),
        "interface" => parse_interface_type_definition(tokens, description, text_source)
            .map(GraphQLTypeSystemDefinition::from),
        "input" => parse_input_object_type_definition(tokens, description, text_source)
            .map(GraphQLTypeSystemDefinition::from),
        _ => Err(WithSpan::new(
            SchemaParseError::TopLevelSchemaDeclarationExpected {
                found_text: identifier_source.to_string(),
            },
            identifier.span,
        )),
    }
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLObjectTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map(|with_span| with_span_to_with_location(with_span, text_source))
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens, text_source)?;

    Ok(GraphQLObjectTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    })
}

/// The state of the PeekableLexer is that it has processed the "interface" keyword
fn parse_interface_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLInterfaceTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map(|with_span| with_span_to_with_location(with_span, text_source))
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens, text_source)?;

    Ok(GraphQLInterfaceTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    })
}

fn parse_input_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLInputObjectTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map(|with_span| with_span_to_with_location(with_span, text_source))
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;

    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_argument_definitions(
        tokens,
        text_source,
        TokenKind::OpenBrace,
        TokenKind::CloseBrace,
    )?;

    Ok(GraphQLInputObjectTypeDefinition {
        description,
        name,
        directives,
        fields,
    })
}

/// The state of the PeekableLexer is that it has processed the "scalar" keyword
fn parse_scalar_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLScalarTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map(|with_span| with_span_to_with_location(with_span, text_source))
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;

    let directives = parse_constant_directives(tokens)?;

    Ok(GraphQLScalarTypeDefinition {
        description,
        name,
        directives,
    })
}

/// The state of the PeekableLexer is that we have not parsed the "implements" keyword.
fn parse_implements_interfaces_if_present(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<WithSpan<InterfaceTypeName>>> {
    if tokens.parse_matching_identifier("implements").is_ok() {
        let interfaces = parse_interfaces(tokens)?;
        Ok(interfaces)
    } else {
        Ok(vec![])
    }
}

/// The state of the PeekableLexer is that it has parsed the "implements"
/// keyword already.
///
/// For ease of implementation, we non-meaningfully deviate from the spec, in that if
/// we parse "Foo &" we return an Err if what follows the & is not an identifier.
/// So, Foo & & would error here.
///
/// In the spec, this would error later, e.g. after an ObjectTypeDefinition
/// with only "Foo", no directives and no fields was successfully parsed.
fn parse_interfaces(tokens: &mut PeekableLexer) -> ParseResult<Vec<WithSpan<InterfaceTypeName>>> {
    let _optional_ampersand = tokens.parse_token_of_kind(TokenKind::Ampersand);

    let first_interface = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;

    let mut interfaces = vec![first_interface];

    while tokens.parse_token_of_kind(TokenKind::Ampersand).is_ok() {
        interfaces.push(
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?,
        );
    }

    Ok(interfaces)
}

fn parse_constant_directives(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<Directive<ConstantValue>>> {
    let mut directives = vec![];
    while tokens.parse_token_of_kind(TokenKind::At).is_ok() {
        directives.push(Directive {
            name: tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?,
            arguments: parse_optional_constant_arguments(tokens)?,
        })
    }
    Ok(directives)
}

// Parse constant arguments passed to a directive used in a schema definition.
fn parse_optional_constant_arguments<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<NameValuePair<T, ConstantValue>>> {
    if tokens.parse_token_of_kind(TokenKind::OpenParen).is_ok() {
        let first_name_value_pair = parse_constant_name_value_pair(tokens, parse_constant_value)?;

        let mut arguments = vec![first_name_value_pair];

        while tokens.parse_token_of_kind(TokenKind::CloseParen).is_err() {
            arguments.push(parse_constant_name_value_pair(
                tokens,
                parse_constant_value,
            )?);
        }

        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

/// The state of the PeekableLexer is that it is about to parse the "foo" in "foo: bar"
fn parse_constant_name_value_pair<T: From<StringKey>, TValue: ValueType>(
    tokens: &mut PeekableLexer,
    parse_value: impl Fn(&mut PeekableLexer) -> ParseResult<WithSpan<TValue>>,
) -> ParseResult<NameValuePair<T, TValue>> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;
    tokens
        .parse_token_of_kind(TokenKind::Colon)
        .map_err(|with_span| with_span.map(SchemaParseError::from))?;
    let value = parse_value(tokens)?;

    Ok(NameValuePair { name, value })
}

fn parse_constant_value(tokens: &mut PeekableLexer) -> ParseResult<WithSpan<ConstantValue>> {
    from_control_flow(|| {
        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::IntegerLiteral)
                .map_err(|with_span| with_span.map(SchemaParseError::from))
                .and_then(|int_literal_string| {
                    int_literal_string.and_then(|raw_int_value| {
                        match raw_int_value.parse::<i64>() {
                            Ok(value) => Ok(ConstantValue::Int(value)),
                            Err(_) => Err(WithSpan::new(
                                SchemaParseError::InvalidIntValue {
                                    text: raw_int_value.to_string(),
                                },
                                int_literal_string.span,
                            )),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::FloatLiteral)
                .map_err(|with_span| with_span.map(SchemaParseError::from))
                .and_then(|float_literal_string| {
                    float_literal_string.and_then(|raw_float_value| {
                        match raw_float_value.parse::<f64>() {
                            Ok(value) => Ok(ConstantValue::Float(value.into())),
                            Err(_) => Err(WithSpan::new(
                                SchemaParseError::InvalidFloatValue {
                                    text: raw_float_value.to_string(),
                                },
                                float_literal_string.span,
                            )),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens.parse_string_key_type(TokenKind::StringLiteral).map(
                |with_quotes: WithSpan<StringLiteralValue>| {
                    // This seems very hacky
                    let without_quotes = with_quotes.map(|string_literal| {
                        let inner_str = &string_literal.lookup();
                        let len = inner_str.len();
                        let without_quotes = (&inner_str[1..(len - 1)]).intern().into();
                        without_quotes
                    });
                    without_quotes.map(ConstantValue::String)
                },
            )
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("true")
                .map(|x| x.map(|_| ConstantValue::Boolean(true)))
        })?;
        to_control_flow(|| {
            tokens
                .parse_matching_identifier("false")
                .map(|x| x.map(|_| ConstantValue::Boolean(false)))
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("null")
                .map(|x| x.map(|_| ConstantValue::Null))
        })?;

        // All remaining identifiers are treated as enums. It is recommended, but not enforced,
        // that enum values be all caps.
        to_control_flow(|| {
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map(|x| x.map(|s| ConstantValue::Enum(s)))
        })?;

        to_control_flow(|| {
            let x: ParseResult<_> = tokens
                .with_span(|tokens| {
                    tokens
                        .parse_token_of_kind(TokenKind::OpenBracket)
                        .map_err(|with_span| with_span.map(SchemaParseError::from))?;
                    let mut values = vec![];
                    while tokens.parse_token_of_kind(TokenKind::CloseBracket).is_err() {
                        values.push(parse_constant_value(tokens)?);
                    }
                    Ok(ConstantValue::List(values))
                })
                .transpose();
            x
        })?;

        to_control_flow(|| {
            let x: ParseResult<_> = tokens
                .with_span(|tokens| {
                    tokens
                        .parse_token_of_kind(TokenKind::OpenBrace)
                        .map_err(|with_span| with_span.map(SchemaParseError::from))?;
                    let mut values = vec![];
                    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
                        let name = tokens
                            .parse_string_key_type(TokenKind::Identifier)
                            .map_err(|with_span| with_span.map(SchemaParseError::from))?;
                        tokens
                            .parse_token_of_kind(TokenKind::Colon)
                            .map_err(|with_span| with_span.map(SchemaParseError::from))?;
                        let value = parse_constant_value(tokens)?;
                        values.push(NameValuePair { name, value });
                    }
                    Ok(ConstantValue::Object(values))
                })
                .transpose();
            x
        })?;

        ControlFlow::Continue(WithSpan::new(
            SchemaParseError::UnableToParseConstantValue,
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

fn parse_optional_fields<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLOutputFieldDefinition>>> {
    let brace = tokens.parse_token_of_kind(TokenKind::OpenBrace);
    if brace.is_err() {
        return Ok(vec![]);
    }

    let field = parse_field(tokens, text_source)?;
    let mut fields = vec![field];

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        fields.push(parse_field(tokens, text_source)?);
    }
    Ok(fields)
}

fn parse_field<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResult<WithLocation<GraphQLOutputFieldDefinition>> {
    let with_span = tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map(|with_span| with_span_to_with_location(with_span, text_source))
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;

            let arguments = parse_optional_argument_definitions(
                tokens,
                text_source,
                TokenKind::OpenParen,
                TokenKind::CloseParen,
            )?;

            tokens
                .parse_token_of_kind(TokenKind::Colon)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;
            let type_ = parse_type_annotation(tokens)?;

            let directives = parse_constant_directives(tokens)?;

            Ok(GraphQLOutputFieldDefinition {
                name,
                type_,
                description,
                arguments,
                directives,
            })
        })
        .transpose()?;
    Ok(with_span_to_with_location(with_span, text_source))
}

fn parse_type_annotation<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
) -> ParseResult<TypeAnnotation<T>> {
    from_control_flow(|| {
        to_control_flow::<_, WithSpan<SchemaParseError>>(|| {
            let type_ = tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;

            let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();
            if is_non_null {
                Ok(TypeAnnotation::NonNull(Box::new(
                    NonNullTypeAnnotation::Named(NamedTypeAnnotation(type_)),
                )))
            } else {
                Ok(TypeAnnotation::Named(NamedTypeAnnotation(type_)))
            }
        })?;

        to_control_flow::<_, WithSpan<SchemaParseError>>(|| {
            // TODO: atomically parse everything here:
            tokens
                .parse_token_of_kind(TokenKind::OpenBracket)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;

            let inner_type_annotation = parse_type_annotation(tokens)?;
            tokens
                .parse_token_of_kind(TokenKind::CloseBracket)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;
            let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();

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
            SchemaParseError::ExpectedTypeAnnotation,
            tokens.peek().span,
        ))
    })
}

fn parse_optional_argument_definitions<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
    open_token: TokenKind,
    close_token: TokenKind,
) -> ParseResult<Vec<WithLocation<GraphQLInputValueDefinition>>> {
    let paren = tokens.parse_token_of_kind(open_token);

    if paren.is_ok() {
        let argument = with_span_to_with_location(
            parse_argument_definition(tokens, text_source)?,
            text_source,
        );
        let mut arguments = vec![argument];

        while tokens.parse_token_of_kind(close_token).is_err() {
            arguments.push(with_span_to_with_location(
                parse_argument_definition(tokens, text_source)?,
                text_source,
            ));
        }
        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_argument_definition<'a>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
) -> ParseResult<WithSpan<GraphQLInputValueDefinition>> {
    tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map(|with_span| with_span_to_with_location(with_span, text_source))
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;
            tokens
                .parse_token_of_kind(TokenKind::Colon)
                .map_err(|with_span| with_span.map(SchemaParseError::from))?;
            let type_ = parse_type_annotation(tokens)?;
            let default_value = parse_optional_constant_default_value(tokens)?;
            let directives = parse_constant_directives(tokens)?;

            Ok(GraphQLInputValueDefinition {
                description,
                name,
                type_,
                default_value,
                directives,
            })
        })
        .transpose()
}

fn parse_optional_constant_default_value<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Option<WithSpan<ConstantValue>>> {
    let equal = tokens.parse_token_of_kind(TokenKind::Equals);
    if equal.is_err() {
        return Ok(None);
    }

    let constant_value = parse_constant_value(tokens)?;
    Ok(Some(constant_value))
}
