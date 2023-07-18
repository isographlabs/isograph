use std::ops::ControlFlow;

use common_lang_types::{DescriptionValue, InterfaceTypeName, WithSpan};
use graphql_syntax::TokenKind;
use intern::string_key::StringKey;

use graphql_lang_types::{
    ConstantValue, Directive, InputValueDefinition, InterfaceTypeDefinition, ListTypeAnnotation,
    NameValuePair, NamedTypeAnnotation, NonNullTypeAnnotation, ObjectTypeDefinition,
    OutputFieldDefinition, ScalarTypeDefinition, TypeAnnotation, TypeSystemDefinition,
    TypeSystemDocument, ValueType,
};

use crate::ParseResult;

use super::{
    description::parse_optional_description, peekable_lexer::PeekableLexer,
    schema_parse_error::SchemaParseError,
};

pub fn parse_schema(source: &str) -> ParseResult<TypeSystemDocument> {
    let mut tokens = PeekableLexer::new(source);

    // dbg!(parse_type_system_document(&mut tokens))
    parse_type_system_document(&mut tokens)
}

fn parse_type_system_document(tokens: &mut PeekableLexer) -> ParseResult<TypeSystemDocument> {
    let mut type_system_definitions = vec![];
    while !tokens.reached_eof() {
        let type_system_definition = parse_type_system_definition(tokens)?;
        type_system_definitions.push(type_system_definition);
    }
    Ok(TypeSystemDocument(type_system_definitions))
}

fn parse_type_system_definition(tokens: &mut PeekableLexer) -> ParseResult<TypeSystemDefinition> {
    let description = parse_optional_description(tokens);
    let identifier = tokens.parse_token_of_kind(TokenKind::Identifier)?;
    let identifier_source = tokens.source(identifier.span);

    match identifier_source {
        "type" => parse_object_type_definition(tokens, description).map(TypeSystemDefinition::from),
        "scalar" => {
            parse_scalar_type_definition(tokens, description).map(TypeSystemDefinition::from)
        }
        "interface" => {
            parse_interface_type_definition(tokens, description).map(TypeSystemDefinition::from)
        }
        _ => Err(SchemaParseError::TopLevelSchemaDeclarationExpected {
            found_text: identifier_source.to_string(),
        }),
    }
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
) -> ParseResult<ObjectTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens)?;

    Ok(ObjectTypeDefinition {
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
) -> ParseResult<InterfaceTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens)?;

    Ok(InterfaceTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    })
}

/// The state of the PeekableLexer is that it has processed the "scalar" keyword
fn parse_scalar_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
) -> ParseResult<ScalarTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let directives = parse_constant_directives(tokens)?;

    Ok(ScalarTypeDefinition {
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

    let first_interface = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let mut interfaces = vec![first_interface];

    while tokens.parse_token_of_kind(TokenKind::Ampersand).is_ok() {
        interfaces.push(tokens.parse_string_key_type(TokenKind::Identifier)?);
    }

    Ok(interfaces)
}

fn parse_constant_directives(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<Directive<ConstantValue>>> {
    let mut directives = vec![];
    while tokens.parse_token_of_kind(TokenKind::At).is_ok() {
        directives.push(Directive {
            name: tokens.parse_string_key_type(TokenKind::Identifier)?,
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
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
    tokens.parse_token_of_kind(TokenKind::Colon)?;
    let value = parse_value(tokens)?;

    Ok(NameValuePair { name, value })
}

fn parse_constant_value(tokens: &mut PeekableLexer) -> ParseResult<WithSpan<ConstantValue>> {
    from_control_flow(|| {
        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::IntegerLiteral)
                .map_err(|x| x.into())
                .and_then(|int_literal_string| {
                    int_literal_string.and_then(|raw_int_value| {
                        match raw_int_value.parse::<i64>() {
                            Ok(value) => Ok(ConstantValue::Int(value)),
                            Err(_) => Err(SchemaParseError::InvalidIntValue {
                                text: raw_int_value.to_string(),
                            }),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::FloatLiteral)
                .map_err(|x| x.into())
                .and_then(|float_literal_string| {
                    float_literal_string.and_then(|raw_float_value| {
                        match raw_float_value.parse::<f64>() {
                            Ok(value) => Ok(ConstantValue::Float(value.into())),
                            Err(_) => Err(SchemaParseError::InvalidFloatValue {
                                text: raw_float_value.to_string(),
                            }),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens
                .parse_string_key_type(TokenKind::StringLiteral)
                .map(|x| x.map(ConstantValue::String))
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
                    tokens.parse_token_of_kind(TokenKind::OpenBracket)?;
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
                    tokens.parse_token_of_kind(TokenKind::OpenBrace)?;
                    let mut values = vec![];
                    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
                        let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
                        tokens.parse_token_of_kind(TokenKind::Colon)?;
                        let value = parse_constant_value(tokens)?;
                        values.push(NameValuePair { name, value });
                    }
                    Ok(ConstantValue::Object(values))
                })
                .transpose();
            x
        })?;

        ControlFlow::Continue(SchemaParseError::UnableToParseConstantValue)
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
) -> ParseResult<Vec<WithSpan<OutputFieldDefinition>>> {
    let brace = tokens.parse_token_of_kind(TokenKind::OpenBrace);
    if brace.is_err() {
        return Ok(vec![]);
    }

    let field = parse_field(tokens)?;
    let mut fields = vec![field];

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        fields.push(parse_field(tokens)?);
    }
    Ok(fields)
}

fn parse_field<'a>(tokens: &mut PeekableLexer<'a>) -> ParseResult<WithSpan<OutputFieldDefinition>> {
    tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

            let arguments = parse_optional_argument_definitions(tokens)?;

            tokens.parse_token_of_kind(TokenKind::Colon)?;
            let type_ = parse_type_annotation(tokens)?;

            let directives = parse_constant_directives(tokens)?;

            Ok(OutputFieldDefinition {
                name,
                type_,
                description,
                arguments,
                directives,
            })
        })
        .transpose()
}

fn parse_type_annotation<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
) -> ParseResult<TypeAnnotation<T>> {
    from_control_flow(|| {
        to_control_flow::<_, SchemaParseError>(|| {
            let type_ = tokens.parse_string_key_type(TokenKind::Identifier)?;

            let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();
            if is_non_null {
                Ok(TypeAnnotation::NonNull(Box::new(
                    NonNullTypeAnnotation::Named(NamedTypeAnnotation(type_)),
                )))
            } else {
                Ok(TypeAnnotation::Named(NamedTypeAnnotation(type_)))
            }
        })?;

        to_control_flow::<_, SchemaParseError>(|| {
            // TODO: atomically parse everything here:
            tokens.parse_token_of_kind(TokenKind::OpenBracket)?;

            let inner_type_annotation = parse_type_annotation(tokens)?;
            tokens.parse_token_of_kind(TokenKind::CloseBracket)?;
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

        ControlFlow::Continue(SchemaParseError::ExpectedTypeAnnotation)
    })
}

fn parse_optional_argument_definitions<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<Vec<WithSpan<InputValueDefinition>>> {
    let paren = tokens.parse_token_of_kind(TokenKind::OpenParen);

    if paren.is_ok() {
        let argument = parse_argument_definition(tokens)?;
        let mut arguments = vec![argument];

        while tokens.parse_token_of_kind(TokenKind::CloseParen).is_err() {
            arguments.push(parse_argument_definition(tokens)?);
        }
        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

fn parse_argument_definition<'a>(
    tokens: &mut PeekableLexer<'a>,
) -> ParseResult<WithSpan<InputValueDefinition>> {
    tokens
        .with_span(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
            tokens.parse_token_of_kind(TokenKind::Colon)?;
            let type_ = parse_type_annotation(tokens)?;
            let default_value = parse_optional_constant_default_value(tokens)?;
            let directives = parse_constant_directives(tokens)?;

            Ok(InputValueDefinition {
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
