use std::{ops::ControlFlow, str::FromStr};

use common_lang_types::{
    DescriptionValue, Diagnostic, DiagnosticResult, EntityName, EnumLiteralValue, Location,
    StringLiteralValue, TextSource, WithEmbeddedLocation, WithLocationPostfix,
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

use super::{description::parse_optional_description, peekable_lexer::PeekableLexer};

pub fn parse_schema(
    source: &str,
    text_source: TextSource,
) -> DiagnosticResult<GraphQLTypeSystemDocument> {
    let mut tokens = PeekableLexer::new(source, text_source);

    parse_type_system_document(&mut tokens)
}

fn parse_type_system_document(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<GraphQLTypeSystemDocument> {
    let mut type_system_definitions = vec![];
    while !tokens.reached_eof() {
        let type_system_definition = parse_type_system_definition(tokens)?;
        type_system_definitions.push(type_system_definition);
    }
    GraphQLTypeSystemDocument(type_system_definitions).wrap_ok()
}

pub fn parse_schema_extensions(
    source: &str,
    text_source: TextSource,
) -> DiagnosticResult<GraphQLTypeSystemExtensionDocument> {
    let mut tokens = PeekableLexer::new(source, text_source);

    parse_type_system_extension_document(&mut tokens)
}

fn parse_type_system_extension_document(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<GraphQLTypeSystemExtensionDocument> {
    let mut definitions_or_extensions = vec![];
    while !tokens.reached_eof() {
        let definition_or_extension = match peek_type_system_doc_type(tokens) {
            Ok(type_system_document_kind) => match type_system_document_kind {
                TypeSystemDocType::Definition => {
                    let with_loc = parse_type_system_definition(tokens)?;
                    with_loc
                        .map(GraphQLTypeSystemExtensionOrDefinition::Definition)
                        .wrap_ok()
                }
                TypeSystemDocType::Extension => {
                    let with_loc = parse_type_system_extension(tokens)?;
                    with_loc
                        .map(GraphQLTypeSystemExtensionOrDefinition::Extension)
                        .wrap_ok()
                }
            },
            Err(unexpected_token) => {
                let found_text = unexpected_token.item.to_string();
                Diagnostic::new(
                    format!(
                        "Expected extend, scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\""
                    ),
                    unexpected_token.location.to::<Location>().wrap_some(),
                ).wrap_err()
            }
        }?;
        definitions_or_extensions.push(definition_or_extension);
    }
    GraphQLTypeSystemExtensionDocument(definitions_or_extensions).wrap_ok()
}

fn parse_type_system_extension(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLTypeSystemExtension>> {
    let extension = tokens.with_embedded_location_result(|tokens| {
        let identifier = tokens
            .parse_source_of_kind(TokenKind::Identifier)
            .expect("Expected identifier extend. This is indicative of a bug in Isograph.");
        assert!(
            identifier.item == "extend",
            "Expected identifier extend. This is indicative of a bug in Isograph."
        );

        let identifier = tokens.parse_source_of_kind(TokenKind::Identifier)?;
        match identifier.item {
            "type" => parse_object_type_extension(tokens)
                .map(GraphQLTypeSystemExtension::from),
            _ => {
                let found_text = identifier.item;
                Diagnostic::new(
                    format!("Expected scalar, type, interface, union, enum, input object, schema or directive, found \"{found_text}\""),
                    identifier.location.to::<Location>().wrap_some()
                )
                .wrap_err()
            },
        }
    })?;

    extension.wrap_ok()
}

fn parse_type_system_definition(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLTypeSystemDefinition>> {
    let definition =
        tokens.with_embedded_location_result(|tokens| {
            let description = parse_optional_description(tokens);
            let identifier = tokens.parse_source_of_kind(TokenKind::Identifier)?;
            match identifier.item {
                "type" => parse_object_type_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "scalar" => parse_scalar_type_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "interface" => parse_interface_type_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "input" => parse_input_object_type_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "directive" => parse_directive_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "enum" => parse_enum_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "union" => parse_union_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                "schema" => parse_schema_definition(tokens, description)
                    .map(GraphQLTypeSystemDefinition::from),
                _ => {
                    let found_text = identifier.item;
                    Diagnostic::new(
                        format!(
                            "Expected extend, scalar, type, interface, union, \
                            enum, input object, schema or directive, found \"{found_text}\""
                        ),
                        identifier.location.to::<Location>().wrap_some(),
                    )
                    .wrap_err()
                }
            }
        })?;

    definition.wrap_ok()
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLObjectTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens)?;

    GraphQLObjectTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    }
    .wrap_ok()
}

/// The state of the PeekableLexer is that it has processed the "type" keyword
fn parse_object_type_extension(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<GraphQLObjectTypeExtension> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens)?;

    GraphQLObjectTypeExtension {
        name,
        interfaces,
        directives,
        fields,
    }
    .wrap_ok()
}

/// The state of the PeekableLexer is that it has processed the "interface" keyword
fn parse_interface_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLInterfaceTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let interfaces = parse_implements_interfaces_if_present(tokens)?;
    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_fields(tokens)?;

    GraphQLInterfaceTypeDefinition {
        description,
        name,
        interfaces,
        directives,
        fields,
    }
    .wrap_ok()
}

fn parse_input_object_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLInputObjectTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let directives = parse_constant_directives(tokens)?;
    let fields = parse_optional_enclosed_items(
        tokens,
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
    .wrap_ok()
}

/// The state of the PeekableLexer is that it has processed the "directive" keyword
fn parse_directive_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLDirectiveDefinition> {
    let _at = tokens.parse_token_of_kind(TokenKind::At);
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let arguments = parse_optional_enclosed_items(
        tokens,
        TokenKind::OpenParen,
        TokenKind::CloseParen,
        parse_argument_definition,
    )?;

    let repeatable = tokens
        .parse_matching_identifier("repeatable")
        .ok()
        .map(|x| x.map(|_| ()));
    let _on = tokens.parse_matching_identifier("on")?;

    let locations = parse_directive_locations(tokens)?;

    GraphQLDirectiveDefinition {
        name,
        arguments,
        repeatable,
        locations,
        description,
    }
    .wrap_ok()
}

fn parse_directive_locations(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<DirectiveLocation>>> {
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
) -> DiagnosticResult<WithEmbeddedLocation<DirectiveLocation>> {
    let peek = tokens.peek();
    match tokens.parse_source_of_kind(TokenKind::Identifier) {
        Ok(text) => DirectiveLocation::from_str(text.item)
            .map_err(|_| {
                Diagnostic::new(
                    format!("Expected directive location, found {}", text.item),
                    text.location.to::<Location>().wrap_some(),
                )
            })
            .map(|x| x.with_location(text.location)),
        Err(diagnostic) => {
            let text = tokens.source(peek.location.span);
            Diagnostic::new(
                format!("Expected directive location, found {text}"),
                diagnostic.0.location,
            )
            .wrap_err()
        }
    }
}

fn parse_enum_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLEnumDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let directives = parse_constant_directives(tokens)?;

    let enum_value_definitions = parse_enum_value_definitions(tokens)?;

    GraphQLEnumDefinition {
        description,
        name,
        directives,
        enum_value_definitions,
    }
    .wrap_ok()
}

fn parse_enum_value_definitions(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<GraphQLEnumValueDefinition>>> {
    parse_optional_enclosed_items(
        tokens,
        TokenKind::OpenBrace,
        TokenKind::CloseBrace,
        parse_enum_value_definition,
    )
}

fn parse_enum_value_definition(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLEnumValueDefinition>> {
    tokens.with_embedded_location_result(|tokens| {
        let description = parse_optional_description(tokens);
        let enum_literal_value_str = tokens.parse_source_of_kind(TokenKind::Identifier)?;
        let value = {
            if enum_literal_value_str.item == "true"
                || enum_literal_value_str.item == "false"
                || enum_literal_value_str.item == "null"
            {
                Diagnostic::new(
                    "Enum values cannot be true, false or  null.".to_string(),
                    enum_literal_value_str.location.to::<Location>().wrap_some(),
                )
                .wrap_err()
            } else {
                enum_literal_value_str
                    .map(|enum_literal_value| EnumLiteralValue::from(enum_literal_value.intern()))
                    .wrap_ok()
            }
        }?;

        let directives = parse_constant_directives(tokens)?;

        GraphQLEnumValueDefinition {
            description,
            value,
            directives,
        }
        .wrap_ok()
    })
}

fn parse_union_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLUnionTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let directives = parse_constant_directives(tokens)?;

    let _equal = tokens.parse_token_of_kind(TokenKind::Equals)?;

    let union_member_types = parse_union_member_types(tokens)?;

    GraphQLUnionTypeDefinition {
        description,
        name,
        directives,
        union_member_types,
    }
    .wrap_ok()
}

fn parse_union_member_types(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<EntityName>>> {
    // This is a no-op if the token kind doesn't match, so effectively
    // this is an optional pipe
    let _pipe = tokens.parse_token_of_kind(TokenKind::Pipe);
    let required_first_value = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let mut values = vec![required_first_value];

    while tokens.parse_token_of_kind(TokenKind::Pipe).is_ok() {
        values.push(tokens.parse_string_key_type(TokenKind::Identifier)?);
    }

    Ok(values)
}

fn parse_schema_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLSchemaDefinition> {
    let directives = parse_constant_directives(tokens)?;

    let _open_curly = tokens.parse_token_of_kind(TokenKind::OpenBrace)?;

    let mut query_type = None;
    let mut mutation_type = None;
    let mut subscription_type = None;

    let first_root_operation_type = parse_root_operation_type(tokens)?;
    match first_root_operation_type.0.item {
        RootOperationKind::Query => query_type = first_root_operation_type.1.wrap_some(),
        RootOperationKind::Subscription => {
            subscription_type = first_root_operation_type.1.wrap_some()
        }
        RootOperationKind::Mutation => mutation_type = first_root_operation_type.1.wrap_some(),
    };

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        let operation_type = parse_root_operation_type(tokens)?;

        match operation_type.0.item {
            RootOperationKind::Query => {
                reassign_or_error(&mut query_type, &operation_type, tokens.text_source)?
            }
            RootOperationKind::Subscription => {
                reassign_or_error(&mut subscription_type, &operation_type, tokens.text_source)?
            }
            RootOperationKind::Mutation => {
                reassign_or_error(&mut mutation_type, &operation_type, tokens.text_source)?
            }
        }
    }

    GraphQLSchemaDefinition {
        description,
        query: query_type,
        subscription: subscription_type,
        mutation: mutation_type,
        directives,
    }
    .wrap_ok()
}

fn reassign_or_error(
    root_type: &mut Option<WithEmbeddedLocation<EntityName>>,
    operation_type: &(
        WithEmbeddedLocation<RootOperationKind>,
        WithEmbeddedLocation<EntityName>,
    ),
    _text_source: TextSource,
) -> DiagnosticResult<()> {
    if root_type.is_some() {
        return Diagnostic::new(
            "Root operation types (query, subscription or mutation) \
            cannot be defined twice in a schema definition."
                .to_string(),
            operation_type.0.location.to::<Location>().wrap_some(),
        )
        .wrap_err();
    }
    *root_type = operation_type.1.wrap_some();
    Ok(())
}

fn parse_root_operation_type(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<(
    WithEmbeddedLocation<RootOperationKind>,
    WithEmbeddedLocation<EntityName>,
)> {
    let name = tokens.parse_source_of_kind(TokenKind::Identifier)?;

    let root_operation_type = match name.item {
        "query" => RootOperationKind::Query.with_location(name.location),
        "subscription" => RootOperationKind::Subscription.with_location(name.location),
        "mutation" => RootOperationKind::Mutation.with_location(name.location),
        _ => {
            return Diagnostic::new(
                "Expected schema, mutation or subscription".to_string(),
                name.location.to::<Location>().wrap_some(),
            )
            .wrap_err();
        }
    };

    let _colon = tokens.parse_token_of_kind(TokenKind::Colon)?;

    let object_name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    (root_operation_type, object_name).wrap_ok()
}

/// The state of the PeekableLexer is that it has processed the "scalar" keyword
fn parse_scalar_type_definition(
    tokens: &mut PeekableLexer,
    description: Option<WithEmbeddedLocation<DescriptionValue>>,
) -> DiagnosticResult<GraphQLScalarTypeDefinition> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

    let directives = parse_constant_directives(tokens)?;

    GraphQLScalarTypeDefinition {
        description,
        name,
        directives,
    }
    .wrap_ok()
}

/// The state of the PeekableLexer is that we have not parsed the "implements" keyword.
fn parse_implements_interfaces_if_present(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<EntityName>>> {
    if tokens.parse_matching_identifier("implements").is_ok() {
        parse_interfaces(tokens)?.wrap_ok()
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
) -> DiagnosticResult<Vec<WithEmbeddedLocation<EntityName>>> {
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
) -> DiagnosticResult<Vec<GraphQLDirective<GraphQLConstantValue>>> {
    let mut directives = vec![];
    while tokens.parse_token_of_kind(TokenKind::At).is_ok() {
        directives.push(GraphQLDirective {
            name: tokens.parse_string_key_type(TokenKind::Identifier)?,
            arguments: parse_optional_constant_arguments(tokens)?,
        })
    }
    Ok(directives)
}

// Parse constant arguments passed to a directive used in a schema definition.
fn parse_optional_constant_arguments<T: From<StringKey>>(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<Vec<NameValuePair<T, GraphQLConstantValue>>> {
    if tokens.parse_token_of_kind(TokenKind::OpenParen).is_ok() {
        let first_name_value_pair = parse_constant_name_value_pair(tokens, parse_constant_value)?;

        let mut arguments = vec![first_name_value_pair];

        while tokens.parse_token_of_kind(TokenKind::CloseParen).is_err() {
            arguments.push(parse_constant_name_value_pair(tokens, |value| {
                parse_constant_value(value)
            })?);
        }

        Ok(arguments)
    } else {
        Ok(vec![])
    }
}

/// The state of the PeekableLexer is that it is about to parse the "foo" in "foo: bar"
fn parse_constant_name_value_pair<T: From<StringKey>, TValue>(
    tokens: &mut PeekableLexer,
    parse_value: impl Fn(&mut PeekableLexer) -> DiagnosticResult<WithEmbeddedLocation<TValue>>,
) -> DiagnosticResult<NameValuePair<T, TValue>> {
    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
    tokens.parse_token_of_kind(TokenKind::Colon)?;
    let value = parse_value(tokens)?;

    NameValuePair { name, value }.wrap_ok()
}

fn parse_constant_value(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLConstantValue>> {
    from_control_flow(|| {
        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::IntegerLiteral)
                .and_then(|int_literal_string| {
                    int_literal_string.and_then(|raw_int_value| {
                        match raw_int_value.parse::<i64>() {
                            Ok(value) => GraphQLConstantValue::Int(value).wrap_ok(),
                            Err(_) => Diagnostic::new(
                                format!("Invalid integer value. Received {raw_int_value}"),
                                int_literal_string.location.to::<Location>().wrap_some(),
                            )
                            .wrap_err(),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens
                .parse_source_of_kind(TokenKind::FloatLiteral)
                .and_then(|float_literal_string| {
                    float_literal_string.and_then(|raw_float_value| {
                        match raw_float_value.parse::<f64>() {
                            Ok(value) => GraphQLConstantValue::Float(value.into()).wrap_ok(),
                            Err(_) => Diagnostic::new(
                                format!("Invalid float value. Received {raw_float_value}."),
                                float_literal_string.location.to::<Location>().wrap_some(),
                            )
                            .wrap_err(),
                        }
                    })
                })
        })?;

        to_control_flow(|| {
            tokens.parse_string_key_type(TokenKind::StringLiteral).map(
                |with_quotes: WithEmbeddedLocation<StringLiteralValue>| {
                    // This seems very hacky
                    let without_quotes = with_quotes.map(|string_literal| {
                        let inner_str = &string_literal.lookup();
                        let len = inner_str.len();

                        (&inner_str[1..(len - 1)]).intern().into()
                    });
                    without_quotes.map(GraphQLConstantValue::String)
                },
            )
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("true")
                .map(|x| x.map(|_| GraphQLConstantValue::Boolean(true)))
        })?;
        to_control_flow(|| {
            tokens
                .parse_matching_identifier("false")
                .map(|x| x.map(|_| GraphQLConstantValue::Boolean(false)))
        })?;

        to_control_flow(|| {
            tokens
                .parse_matching_identifier("null")
                .map(|x| x.map(|_| GraphQLConstantValue::Null))
        })?;

        // All remaining identifiers are treated as enums. It is recommended, but not enforced,
        // that enum values be all caps.
        to_control_flow(|| {
            tokens
                .parse_string_key_type(TokenKind::Identifier)
                .map(|x| x.map(GraphQLConstantValue::Enum))
        })?;

        to_control_flow(|| {
            tokens.with_embedded_location_result::<_, Diagnostic>(|tokens| {
                tokens.parse_token_of_kind(TokenKind::OpenBracket)?;
                let mut values = vec![];
                while tokens.parse_token_of_kind(TokenKind::CloseBracket).is_err() {
                    values.push(parse_constant_value(tokens)?);
                }
                GraphQLConstantValue::List(values).wrap_ok()
            })
        })?;

        to_control_flow(|| {
            tokens.with_embedded_location_result::<_, Diagnostic>(|tokens| {
                tokens.parse_token_of_kind(TokenKind::OpenBrace)?;

                let mut values = vec![];
                while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
                    let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
                    tokens.parse_token_of_kind(TokenKind::Colon)?;
                    let value = parse_constant_value(tokens)?;
                    values.push(NameValuePair { name, value });
                }
                GraphQLConstantValue::Object(values).wrap_ok()
            })
        })?;

        ControlFlow::Continue(Diagnostic::new(
            "Unable to parse constant value".to_string(),
            tokens.peek().location.to::<Location>().wrap_some(),
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

fn parse_optional_fields(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<GraphQLFieldDefinition>>> {
    let brace = tokens.parse_token_of_kind(TokenKind::OpenBrace);
    if brace.is_err() {
        return Ok(vec![]);
    }

    let field = parse_field(tokens)?;
    let mut fields = vec![field];

    while tokens.parse_token_of_kind(TokenKind::CloseBrace).is_err() {
        fields.push(parse_field(tokens)?);
    }
    fields.wrap_ok()
}

fn parse_field(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLFieldDefinition>> {
    tokens
        .with_embedded_location_result(|tokens| {
            let description = parse_optional_description(tokens);
            let name = tokens.parse_string_key_type(TokenKind::Identifier)?;

            let arguments = parse_optional_enclosed_items(
                tokens,
                TokenKind::OpenParen,
                TokenKind::CloseParen,
                parse_argument_definition,
            )?;

            tokens.parse_token_of_kind(TokenKind::Colon)?;

            let type_ = parse_type_annotation(tokens)?;

            let directives = parse_constant_directives(tokens)?;

            GraphQLFieldDefinition {
                name,
                type_,
                description,
                arguments,
                directives,
            }
            .wrap_ok::<Diagnostic>()
        })?
        .wrap_ok()
}

fn parse_type_annotation(
    tokens: &mut PeekableLexer,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLTypeAnnotation>> {
    tokens.with_embedded_location_result(|tokens| {
        from_control_flow(|| {
            to_control_flow::<_, Diagnostic>(|| {
                let entity_name = tokens.parse_string_key_type(TokenKind::Identifier)?.item;

                let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();
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
                tokens.parse_token_of_kind(TokenKind::OpenBracket)?;

                let inner_type_annotation = parse_type_annotation(tokens)?;
                tokens.parse_token_of_kind(TokenKind::CloseBracket)?;
                let is_non_null = tokens.parse_token_of_kind(TokenKind::Exclamation).is_ok();

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
                "Expected a type (e.g. String, [String] or String!)".to_string(),
                tokens.peek().location.to::<Location>().wrap_some(),
            ))
        })
    })
}

fn parse_optional_enclosed_items<'a, T>(
    tokens: &mut PeekableLexer<'a>,
    open_token: TokenKind,
    close_token: TokenKind,
    mut parse: impl FnMut(&mut PeekableLexer<'a>) -> DiagnosticResult<WithEmbeddedLocation<T>>,
) -> DiagnosticResult<Vec<WithEmbeddedLocation<T>>> {
    let paren = tokens.parse_token_of_kind(open_token);

    if paren.is_ok() {
        let argument = parse(tokens)?;
        let mut arguments = vec![argument];

        while tokens.parse_token_of_kind(close_token).is_err() {
            arguments.push(parse(tokens)?);
        }
        arguments.wrap_ok()
    } else {
        vec![].wrap_ok()
    }
}

fn parse_argument_definition(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<WithEmbeddedLocation<GraphQLInputValueDefinition>> {
    tokens.with_embedded_location_result(|tokens| {
        let description = parse_optional_description(tokens);
        let name = tokens.parse_string_key_type(TokenKind::Identifier)?;
        tokens.parse_token_of_kind(TokenKind::Colon)?;
        let type_ = parse_type_annotation(tokens)?;
        let default_value = parse_optional_constant_default_value(tokens)?;
        let directives = parse_constant_directives(tokens)?;

        GraphQLInputValueDefinition {
            description,
            name,
            type_,
            default_value,
            directives,
        }
        .wrap_ok()
    })
}

fn parse_optional_constant_default_value(
    tokens: &mut PeekableLexer<'_>,
) -> DiagnosticResult<Option<WithEmbeddedLocation<GraphQLConstantValue>>> {
    let equal = tokens.parse_token_of_kind(TokenKind::Equals);
    if equal.is_err() {
        return Ok(None);
    }

    parse_constant_value(tokens)?.wrap_some().wrap_ok()
}

enum TypeSystemDocType {
    Definition,
    Extension,
}

fn peek_type_system_doc_type(
    tokens: &PeekableLexer,
) -> Result<TypeSystemDocType, WithEmbeddedLocation<TokenKind>> {
    let peeked = tokens.peek();
    match peeked.item {
        TokenKind::StringLiteral => TypeSystemDocType::Definition.wrap_ok(),
        TokenKind::BlockStringLiteral => TypeSystemDocType::Definition.wrap_ok(),
        TokenKind::Identifier => {
            let text = tokens.source(peeked.location.span);
            match text {
                "extend" => TypeSystemDocType::Extension,
                _ => TypeSystemDocType::Definition,
            }
            .wrap_ok()
        }
        _ => Err(peeked),
    }
}
