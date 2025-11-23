use std::{ops::ControlFlow, str::FromStr};

use common_lang_types::{
    DescriptionValue, Diagnostic, EnumLiteralValue, GraphQLInterfaceTypeName,
    GraphQLObjectTypeName, Location, StringLiteralValue, TextSource, WithLocation, WithSpan,
    WithSpanPostfix,
};
use graphql_syntax::TokenKind;
use intern::{
    Lookup,
    string_key::{Intern, StringKey},
};

use graphql_lang_types::{
    DirectiveLocation, GraphQLConstantValue, GraphQLDirective, GraphQLDirectiveDefinition,
    GraphQLEnumDefinition, GraphQLEnumValueDefinition, GraphQLFieldDefinition,
    GraphQLInputObjectTypeDefinition, GraphQLInputValueDefinition, GraphQLInterfaceTypeDefinition,
    GraphQLListTypeAnnotation, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLObjectTypeDefinition, GraphQLObjectTypeExtension, GraphQLScalarTypeDefinition,
    GraphQLSchemaDefinition, GraphQLTypeAnnotation, GraphQLTypeSystemDefinition,
    GraphQLTypeSystemDocument, GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionDocument,
    GraphQLTypeSystemExtensionOrDefinition, GraphQLUnionTypeDefinition, NameValuePair,
    RootOperationKind,
};
use prelude::Postfix;

use crate::ParseResult;

use super::{
    description::parse_optional_description, peekable_lexer::PeekableLexer,
    schema_parse_error::SchemaParseError,
};

pub fn parse_schema(
    source: &str,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemDocument> {
    let mut tokens = PeekableLexer::new(source, text_source);

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
    GraphQLTypeSystemDocument(type_system_definitions).ok()
}

pub fn parse_schema_extensions(
    source: &str,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemExtensionDocument> {
    let mut tokens = PeekableLexer::new(source, text_source);

    parse_type_system_extension_document(&mut tokens, text_source)
}

fn parse_type_system_extension_document(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<GraphQLTypeSystemExtensionDocument> {
    let mut definitions_or_extensions = vec![];
    while !tokens.reached_eof() {
        let definition_or_extension = match peek_type_system_doc_type(tokens) {
            Ok(type_system_document_kind) => match type_system_document_kind {
                TypeSystemDocType::Definition => {
                    let with_loc = parse_type_system_definition(tokens, text_source)?;
                    with_loc
                        .map(GraphQLTypeSystemExtensionOrDefinition::Definition)
                        .ok()
                }
                TypeSystemDocType::Extension => {
                    let with_loc = parse_type_system_extension(tokens, text_source)?;
                    with_loc
                        .map(GraphQLTypeSystemExtensionOrDefinition::Extension)
                        .ok()
                }
            },
            Err(unexpected_token) => {
                SchemaParseError::TopLevelSchemaDeclarationOrExtensionExpected {
                    found_text: unexpected_token.item.to_string(),
                }
                .with_span(unexpected_token.span)
                .err()
            }
        }?;
        definitions_or_extensions.push(definition_or_extension);
    }
    GraphQLTypeSystemExtensionDocument(definitions_or_extensions).ok()
}

fn parse_type_system_extension(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<WithLocation<GraphQLTypeSystemExtension>> {
    let extension = tokens.with_span_result(|tokens| {
        let identifier = tokens
            .parse_source_of_kind(TokenKind::Identifier)
            .expect("Expected identifier extend. This is indicative of a bug in Isograph.");
        assert!(
            identifier.item == "extend",
            "Expected identifier extend. This is indicative of a bug in Isograph."
        );

        let identifier = tokens
            .parse_source_of_kind(TokenKind::Identifier)
            .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?;
        match identifier.item {
            "type" => parse_object_type_extension(tokens, text_source)
                .map(GraphQLTypeSystemExtension::from),
            _ => SchemaParseError::TopLevelSchemaDeclarationExpected {
                found_text: identifier.to_string(),
            }
            .with_span(identifier.span)
            .err(),
        }
    })?;

    extension.to_with_location(text_source).ok()
}

fn parse_type_system_definition(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<WithLocation<GraphQLTypeSystemDefinition>> {
    let definition = tokens.with_span_result(|tokens| {
        let description = parse_optional_description(tokens);
        let identifier = tokens
            .parse_source_of_kind(TokenKind::Identifier)
            .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?;
        match identifier.item {
            "type" => parse_object_type_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "scalar" => parse_scalar_type_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "interface" => parse_interface_type_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "input" => parse_input_object_type_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "directive" => parse_directive_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "enum" => parse_enum_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "union" => parse_union_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            "schema" => parse_schema_definition(tokens, description, text_source)
                .map(GraphQLTypeSystemDefinition::from),
            _ => SchemaParseError::TopLevelSchemaDeclarationExpected {
                found_text: identifier.item.to_string(),
            }
            .with_span(identifier.span)
            .err(),
        }
    })?;

    definition.to_with_location(text_source).ok()
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLObjectTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?
        .to_with_embedded_location(text_source);

    let interfaces = parse_implements_interfaces_if_present(tokens, text_source)?;
    let directives = parse_constant_directives(tokens, text_source)?;
    let fields = parse_optional_fields(tokens, text_source)?;

    GraphQLObjectTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    }
    .ok()
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_extension(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<GraphQLObjectTypeExtension> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map(|with_span| with_span.to_with_location(text_source))
        .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?;

    let interfaces = parse_implements_interfaces_if_present(tokens, text_source)?;
    let directives = parse_constant_directives(tokens, text_source)?;
    let fields = parse_optional_fields(tokens, text_source)?;

    GraphQLObjectTypeExtension {
        name,
        interfaces,
        directives,
        fields,
    }
    .ok()
}

/// The state of the PeekableLexer is that it has processed the "interface" keyword
fn parse_interface_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLInterfaceTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?
        .to_with_embedded_location(text_source);

    let interfaces = parse_implements_interfaces_if_present(tokens, text_source)?;
    let directives = parse_constant_directives(tokens, text_source)?;
    let fields = parse_optional_fields(tokens, text_source)?;

    GraphQLInterfaceTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    }
    .ok()
}

fn parse_input_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLInputObjectTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?
        .to_with_embedded_location(text_source);

    let directives = parse_constant_directives(tokens, text_source)?;
    let fields = parse_optional_enclosed_items(
        tokens,
        text_source,
        TokenKind::OpenBrace,
        TokenKind::CloseBrace,
        parse_argument_definition,
    )?;

    GraphQLInputObjectTypeDefinition {
        description,
        name,
        directives,
        fields,
    }
    .ok()
}

/// The state of the PeekableLexer is that it has processed the "directive" keyword
fn parse_directive_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLDirectiveDefinition> {
    let _at = tokens.parse_token_of_kind(TokenKind::At);
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|diagnostic| SchemaParseError::ParseError(diagnostic).with_generated_span())?
        .to_with_location(text_source);

    let arguments = parse_optional_enclosed_items(
        tokens,
        text_source,
        TokenKind::OpenParen,
        TokenKind::CloseParen,
        parse_argument_definition,
    )?;

    let repeatable = tokens
        .parse_matching_identifier("repeatable")
        .ok()
        .map(|x| x.map(|_| ()));
    let _on = tokens
        .parse_matching_identifier("on")
        .map_err(|x| SchemaParseError::from(x).with_generated_span())?;

    let locations = parse_directive_locations(tokens)?;

    GraphQLDirectiveDefinition {
        name,
        arguments,
        repeatable,
        locations,
        description,
    }
    .ok()
}

fn parse_directive_locations(
    tokens: &mut PeekableLexer,
) -> ParseResult<Vec<WithSpan<DirectiveLocation>>> {
    // This is a no-op if the token kind doesn't match, so effectively
    // this is an optional pipe
    let _pipe = tokens.parse_token_of_kind(TokenKind::Pipe);
    let required_location = parse_directive_location(tokens)?;
    let mut locations = vec![required_location];

    while tokens.parse_token_of_kind(TokenKind::Pipe).is_ok() {
        locations.push(parse_directive_location(tokens)?);
    }

    Ok(locations)
}

fn parse_directive_location(
    tokens: &mut PeekableLexer,
) -> ParseResult<WithSpan<DirectiveLocation>> {
    let peek = tokens.peek();
    match tokens.parse_source_of_kind(TokenKind::Identifier) {
        Ok(text) => DirectiveLocation::from_str(text.item)
            .map_err(|_| {
                SchemaParseError::ParseError(Diagnostic::new(
                    format!("Expected directive location, found {}", text.item),
                    Location::new(tokens.text_source, text.span).some(),
                ))
                .with_span(text.span)
            })
            .map(|x| x.with_span(text.span)),
        Err(diagnostic) => {
            let text = tokens.source(peek.span);
            SchemaParseError::ParseError(Diagnostic::new(
                format!("Expected directive location, found {text}"),
                diagnostic.0.location,
            ))
            .with_generated_span()
            .err()
        }
    }
}

fn parse_enum_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLEnumDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|e| SchemaParseError::ParseError(e).with_generated_span())?
        .to_with_location(text_source);

    let directives = parse_constant_directives(tokens, text_source)?;

    let enum_value_definitions = parse_enum_value_definitions(tokens, text_source)?;

    GraphQLEnumDefinition {
        description,
        name,
        directives,
        enum_value_definitions,
    }
    .ok()
}

fn parse_enum_value_definitions(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLEnumValueDefinition>>> {
    parse_optional_enclosed_items(
        tokens,
        text_source,
        TokenKind::OpenBrace,
        TokenKind::CloseBrace,
        parse_enum_value_definition,
    )
}

fn parse_enum_value_definition(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<WithSpan<GraphQLEnumValueDefinition>> {
    tokens.with_span_result(|tokens| {
        let description = parse_optional_description(tokens);
        let enum_literal_value_str = tokens
            .parse_source_of_kind(TokenKind::Identifier)
            .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;
        let value = {
            if enum_literal_value_str.item == "true"
                || enum_literal_value_str.item == "false"
                || enum_literal_value_str.item == "null"
            {
                enum_literal_value_str
                    .map(|_| SchemaParseError::EnumValueTrueFalseNull)
                    .err()
            } else {
                enum_literal_value_str
                    .map(|enum_literal_value| EnumLiteralValue::from(enum_literal_value.intern()))
                    .to_with_location(text_source)
                    .ok()
            }
        }?;

        let directives = parse_constant_directives(tokens, text_source)?;

        GraphQLEnumValueDefinition {
            description,
            value,
            directives,
        }
        .ok()
    })
}

fn parse_union_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLUnionTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
        .to_with_embedded_location(text_source);

    let directives = parse_constant_directives(tokens, text_source)?;

    let _equal = tokens
        .parse_token_of_kind(TokenKind::Equals)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    let union_member_types = parse_union_member_types(tokens, text_source)?;

    GraphQLUnionTypeDefinition {
        description,
        name,
        directives,
        union_member_types,
    }
    .ok()
}

fn parse_union_member_types(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLObjectTypeName>>> {
    // This is a no-op if the token kind doesn't match, so effectively
    // this is an optional pipe
    let _pipe = tokens.parse_token_of_kind(TokenKind::Pipe);
    let required_first_value = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
        .to_with_location(text_source);

    let mut values = vec![required_first_value];

    while tokens.parse_token_of_kind(TokenKind::Pipe).is_ok() {
        values.push(
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                .to_with_location(text_source),
        );
    }

    Ok(values)
}

fn parse_schema_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLSchemaDefinition> {
    let directives = parse_constant_directives(tokens, text_source)?;

    let _open_curly = tokens
        .parse_token_of_kind(TokenKind::OpenBrace)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    let mut query_type = None;
    let mut mutation_type = None;
    let mut subscription_type = None;

    let first_root_operation_type = parse_root_operation_type(tokens, text_source)?;
    match first_root_operation_type.0.item {
        RootOperationKind::Query => query_type = first_root_operation_type.1.some(),
        RootOperationKind::Subscription => subscription_type = first_root_operation_type.1.some(),
        RootOperationKind::Mutation => mutation_type = first_root_operation_type.1.some(),
    };

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        let operation_type = parse_root_operation_type(tokens, text_source)?;

        match operation_type.0.item {
            RootOperationKind::Query => reassign_or_error(&mut query_type, &operation_type)?,
            RootOperationKind::Subscription => {
                reassign_or_error(&mut subscription_type, &operation_type)?
            }
            RootOperationKind::Mutation => reassign_or_error(&mut mutation_type, &operation_type)?,
        }
    }

    GraphQLSchemaDefinition {
        description,
        query: query_type,
        subscription: subscription_type,
        mutation: mutation_type,
        directives,
    }
    .ok()
}

fn reassign_or_error(
    root_type: &mut Option<WithLocation<GraphQLObjectTypeName>>,
    operation_type: &(
        WithSpan<RootOperationKind>,
        WithLocation<GraphQLObjectTypeName>,
    ),
) -> ParseResult<()> {
    if root_type.is_some() {
        return SchemaParseError::RootOperationTypeRedefined
            .with_span(operation_type.0.span)
            .err();
    }
    *root_type = operation_type.1.some();
    Ok(())
}

fn parse_root_operation_type(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<(
    WithSpan<RootOperationKind>,
    WithLocation<GraphQLObjectTypeName>,
)> {
    let name = tokens
        .parse_source_of_kind(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    let root_operation_type = match name.item {
        "query" => RootOperationKind::Query.with_span(name.span),
        "subscription" => RootOperationKind::Subscription.with_span(name.span),
        "mutation" => RootOperationKind::Mutation.with_span(name.span),
        _ => {
            return SchemaParseError::ExpectedRootOperationType
                .with_span(name.span)
                .err();
        }
    };

    let _colon = tokens
        .parse_token_of_kind(TokenKind::Colon)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    let object_name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    (
        root_operation_type,
        object_name.to_with_location(text_source),
    )
        .ok()
}

/// The state of the PeekableLexer is that it has processed the "scalar" keyword
fn parse_scalar_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithSpan<DescriptionValue>>,
    text_source: TextSource,
) -> ParseResult<GraphQLScalarTypeDefinition> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
        .to_with_location(text_source);

    let directives = parse_constant_directives(tokens, text_source)?;

    GraphQLScalarTypeDefinition {
        description,
        name,
        directives,
    }
    .ok()
}

/// The state of the PeekableLexer is that we have not parsed the "implements" keyword.
fn parse_implements_interfaces_if_present(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLInterfaceTypeName>>> {
    if tokens.parse_matching_identifier("implements").is_ok() {
        parse_interfaces(tokens, text_source)?.ok()
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
fn parse_interfaces(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLInterfaceTypeName>>> {
    let _optional_ampersand = tokens.parse_token_of_kind(TokenKind::Ampersand);

    let first_interface = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

    let mut interfaces = vec![first_interface.to_with_location(text_source)];

    while tokens.parse_token_of_kind(TokenKind::Ampersand).is_ok() {
        interfaces.push(
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                .to_with_location(text_source),
        );
    }

    Ok(interfaces)
}

fn parse_constant_directives(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<GraphQLDirective<GraphQLConstantValue>>> {
    let mut directives = vec![];
    while tokens.parse_token_of_kind(TokenKind::At).is_ok() {
        directives.push(GraphQLDirective {
            name: tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                .to_with_embedded_location(text_source),
            arguments: parse_optional_constant_arguments(tokens, text_source)?,
        })
    }
    Ok(directives)
}

// Parse constant arguments passed to a directive used in a schema definition.
fn parse_optional_constant_arguments<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<Vec<NameValuePair<T, GraphQLConstantValue>>> {
    if tokens.parse_token_of_kind(TokenKind::OpenParen).is_ok() {
        let first_name_value_pair = parse_constant_name_value_pair(
            tokens,
            |tokens| parse_constant_value(tokens, text_source),
            text_source,
        )?;

        let mut arguments = vec![first_name_value_pair];

        while tokens.parse_token_of_kind(TokenKind::CloseParen).is_err() {
            arguments.push(parse_constant_name_value_pair(
                tokens,
                |value| parse_constant_value(value, text_source),
                text_source,
            )?);
        }

        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

/// The state of the PeekableLexer is that it is about to parse the "foo" in "foo: bar"
fn parse_constant_name_value_pair<T: From<StringKey>, TValue>(
    tokens: &mut PeekableLexer,
    parse_value: impl Fn(&mut PeekableLexer) -> ParseResult<WithLocation<TValue>>,
    text_source: TextSource,
) -> ParseResult<NameValuePair<T, TValue>> {
    let name = tokens
        .parse_string_key_type(TokenKind::Identifier)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
        .to_with_location(text_source);
    tokens
        .parse_token_of_kind(TokenKind::Colon)
        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;
    let value = parse_value(tokens)?;

    NameValuePair { name, value }.ok()
}

fn parse_constant_value(
    tokens: &mut PeekableLexer,
    text_source: TextSource,
) -> ParseResult<WithLocation<GraphQLConstantValue>> {
    from_control_flow(|| {
        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::IntegerLiteral)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())
                .and_then(|int_literal_string| {
                    int_literal_string.and_then(|raw_int_value| {
                        match raw_int_value.parse::<i64>() {
                            Ok(value) => GraphQLConstantValue::Int(value).ok(),
                            Err(_) => SchemaParseError::InvalidIntValue {
                                text: raw_int_value.to_string(),
                            }
                            .with_span(int_literal_string.span)
                            .err(),
                        }
                    })
                })
                .map(|x| x.to_with_location(text_source))
        })?;

        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::FloatLiteral)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())
                .and_then(|float_literal_string| {
                    float_literal_string.and_then(|raw_float_value| {
                        match raw_float_value.parse::<f64>() {
                            Ok(value) => GraphQLConstantValue::Float(value.into()).ok(),
                            Err(_) => SchemaParseError::InvalidFloatValue {
                                text: raw_float_value.to_string(),
                            }
                            .with_span(float_literal_string.span)
                            .err(),
                        }
                    })
                })
                .map(|x| x.to_with_location(text_source))
        })?;

        to_control_flow(|| {
            tokens
                .parse_string_key_type(TokenKind::StringLiteral)
                .map(|with_quotes: WithSpan<StringLiteralValue>| {
                    // This seems very hacky
                    let without_quotes = with_quotes.map(|string_literal| {
                        let inner_str = &string_literal.lookup();
                        let len = inner_str.len();

                        (&inner_str[1..(len - 1)]).intern().into()
                    });
                    without_quotes.map(GraphQLConstantValue::String)
                })
                .map(|x| x.to_with_location(text_source))
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("true")
                .map(|x| x.map(|_| GraphQLConstantValue::Boolean(true)))
                .map(|x| x.to_with_location(text_source))
        })?;
        to_control_flow(|| {
            tokens
                .parse_matching_identifier("false")
                .map(|x| x.map(|_| GraphQLConstantValue::Boolean(false)))
                .map(|x| x.to_with_location(text_source))
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("null")
                .map(|x| x.map(|_| GraphQLConstantValue::Null))
                .map(|x| x.to_with_location(text_source))
        })?;

        // All remaining identifiers are treated as enums. It is recommended, but not enforced,
        // that enum values be all caps.
        to_control_flow(|| {
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map(|x| x.map(GraphQLConstantValue::Enum))
                .map(|x| x.to_with_location(text_source))
        })?;

        to_control_flow(|| {
            let x: ParseResult<_> = tokens
                .with_span_result(|tokens| {
                    tokens
                        .parse_token_of_kind(TokenKind::OpenBracket)
                        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;
                    let mut values = vec![];
                    while tokens.parse_token_of_kind(TokenKind::CloseBracket).is_err() {
                        values.push(parse_constant_value(tokens, text_source)?);
                    }
                    GraphQLConstantValue::List(values).ok()
                })
                .map(|x| x.to_with_location(text_source));
            x
        })?;

        to_control_flow(|| {
            let x: ParseResult<_> = tokens
                .with_span_result(|tokens| {
                    tokens
                        .parse_token_of_kind(TokenKind::OpenBrace)
                        .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

                    let mut values = vec![];
                    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
                        let name = tokens
                            .parse_string_key_type(TokenKind::Identifier)
                            .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                            .to_with_location(text_source);
                        tokens
                            .parse_token_of_kind(TokenKind::Colon)
                            .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                            .to_with_location(text_source);
                        let value = parse_constant_value(tokens, text_source)?;
                        values.push(NameValuePair { name, value });
                    }
                    GraphQLConstantValue::Object(values).ok()
                })
                .map(|x| x.to_with_location(text_source));
            x
        })?;

        ControlFlow::Continue(
            SchemaParseError::UnableToParseConstantValue.with_span(tokens.peek().span),
        )
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

fn parse_optional_fields(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResult<Vec<WithLocation<GraphQLFieldDefinition>>> {
    let brace = tokens.parse_token_of_kind(TokenKind::OpenBrace);
    if brace.is_err() {
        return Ok(vec![]);
    }

    let field = parse_field(tokens, text_source)?;
    let mut fields = vec![field];

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        fields.push(parse_field(tokens, text_source)?);
    }
    fields.ok()
}

fn parse_field(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResult<WithLocation<GraphQLFieldDefinition>> {
    tokens
        .with_span_result(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
                .to_with_location(text_source);

            let arguments = parse_optional_enclosed_items(
                tokens,
                text_source,
                TokenKind::OpenParen,
                TokenKind::CloseParen,
                parse_argument_definition,
            )?;

            tokens
                .parse_token_of_kind(TokenKind::Colon)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

            let type_ = parse_type_annotation(tokens)?;

            let directives = parse_constant_directives(tokens, text_source)?;

            GraphQLFieldDefinition {
                name,
                type_,
                description,
                arguments,
                directives,
                is_inline_fragment: false,
            }
            .ok()
        })?
        .to_with_location(text_source)
        .ok()
}

fn parse_type_annotation<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
) -> ParseResult<GraphQLTypeAnnotation<T>> {
    from_control_flow(|| {
        to_control_flow::<_, WithSpan<SchemaParseError>>(|| {
            let type_ = tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

            let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();
            if is_non_null {
                GraphQLTypeAnnotation::NonNull(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(type_)).boxed(),
                )
                .ok()
            } else {
                GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(type_)).ok()
            }
        })?;

        to_control_flow::<_, WithSpan<SchemaParseError>>(|| {
            // TODO: atomically parse everything here:
            tokens
                .parse_token_of_kind(TokenKind::OpenBracket)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;

            let inner_type_annotation = parse_type_annotation(tokens)?;
            tokens
                .parse_token_of_kind(TokenKind::CloseBracket)
                .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;
            let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();

            if is_non_null {
                GraphQLTypeAnnotation::NonNull(
                    GraphQLNonNullTypeAnnotation::List(GraphQLListTypeAnnotation(
                        inner_type_annotation,
                    ))
                    .boxed(),
                )
                .ok()
            } else {
                GraphQLTypeAnnotation::List(
                    GraphQLListTypeAnnotation(inner_type_annotation).boxed(),
                )
                .ok()
            }
        })?;

        // One **cannot** add additional cases here (though of course none exist in the spec.)
        // Because, if we successfully parse the OpenBracket for a list type, we must parse the
        // entirety of the list type. Otherwise, we will have eaten the OpenBracket and will
        // leave the parser in an inconsistent state.
        //
        // We don't get a great error message with this current approach.

        ControlFlow::Continue(
            SchemaParseError::ExpectedTypeAnnotation.with_span(tokens.peek().span),
        )
    })
}

fn parse_optional_enclosed_items<'a, T>(
    tokens: &mut PeekableLexer<'a>,
    text_source: TextSource,
    open_token: TokenKind,
    close_token: TokenKind,
    mut parse: impl FnMut(&mut PeekableLexer<'a>, TextSource) -> ParseResult<WithSpan<T>>,
) -> ParseResult<Vec<WithLocation<T>>> {
    let paren = tokens.parse_token_of_kind(open_token);

    if paren.is_ok() {
        let argument = parse(tokens, text_source)?.to_with_location(text_source);
        let mut arguments = vec![argument];

        while tokens.parse_token_of_kind(close_token).is_err() {
            arguments.push(parse(tokens, text_source)?.to_with_location(text_source));
        }
        arguments.ok()
    } else {
        vec![].ok()
    }
}

fn parse_argument_definition(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResult<WithSpan<GraphQLInputValueDefinition>> {
    tokens.with_span_result(|tokens| {
        let description = parse_optional_description(tokens);
        let name = tokens
            .parse_string_key_type(TokenKind::Identifier)
            .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?
            .to_with_location(text_source);
        tokens
            .parse_token_of_kind(TokenKind::Colon)
            .map_err(|err| SchemaParseError::ParseError(err).with_generated_span())?;
        let type_ = parse_type_annotation(tokens)?;
        let default_value = parse_optional_constant_default_value(tokens, text_source)?;
        let directives = parse_constant_directives(tokens, text_source)?;

        GraphQLInputValueDefinition {
            description,
            name,
            type_,
            default_value,
            directives,
        }
        .ok()
    })
}

fn parse_optional_constant_default_value(
    tokens: &mut PeekableLexer<'_>,
    text_source: TextSource,
) -> ParseResult<Option<WithLocation<GraphQLConstantValue>>> {
    let equal = tokens.parse_token_of_kind(TokenKind::Equals);
    if equal.is_err() {
        return Ok(None);
    }

    parse_constant_value(tokens, text_source)?.some().ok()
}

enum TypeSystemDocType {
    Definition,
    Extension,
}

fn peek_type_system_doc_type(
    tokens: &PeekableLexer,
) -> Result<TypeSystemDocType, WithSpan<TokenKind>> {
    let peeked = tokens.peek();
    match peeked.item {
        TokenKind::StringLiteral => TypeSystemDocType::Definition.ok(),
        TokenKind::BlockStringLiteral => TypeSystemDocType::Definition.ok(),
        TokenKind::Identifier => {
            let text = tokens.source(peeked.span);
            match text {
                "extend" => TypeSystemDocType::Extension,
                _ => TypeSystemDocType::Definition,
            }
            .ok()
        }
        _ => Err(peeked),
    }
}
