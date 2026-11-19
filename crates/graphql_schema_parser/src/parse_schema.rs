use common_lang_types::{DescriptionValue, DirectiveName, EntityName};
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::keyword::BooleanOrNullKeyword;
use crate::string_value::consume_string_value_if;
use crate::token_cursor::TokenCursor;
use crate::type_annotation::parse_type_annotation;
use crate::value::parse_constant_value;
use crate::{
    Argument, ArgumentNameWrapper, Description, Directive, DirectiveDefinition, DirectiveLocation,
    DirectiveLocationWrapper, DirectiveNameWrapper, DirectiveRepeatability, EntityNameWrapper,
    EnumLiteralValueWrapper, EnumTypeDefinition, EnumValueDefinition, Expectation, FieldDefinition,
    GraphQLTokenKind, InputObjectTypeDefinition, InputValueDefinition, InputValueNameWrapper,
    InterfaceTypeDefinition, ObjectTypeDefinition, OperationType, OperationTypeWrapper, ParseError,
    RootOperationTypeDefinition, ScalarTypeDefinition, SchemaDefinition, SelectableNameWrapper,
    UnionTypeDefinition,
};

/// A top-level definition or extension whose parse failed. The wrapping span is the text the
/// definition occupied, from its first token to the last token recovery skipped.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct DefinitionError {
    /// The name the failed definition declared; `None` when the parse failed before the
    /// name, and for a schema definition or extension, which has no name.
    pub name: Option<DefinitionName>,
    pub error: WithSpan<ParseError>,
}

/// The name a top-level definition or extension declares.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DefinitionName {
    Entity(EntityName),
    Directive(DirectiveName),
}

impl DefinitionError {
    fn unnamed(error: WithSpan<ParseError>) -> Self {
        DefinitionError { name: None, error }
    }
}

fn parse_after_entity_name<T>(
    cursor: &mut TokenCursor<'_>,
    parse_rest: impl FnOnce(
        &mut TokenCursor<'_>,
        WithSpan<EntityNameWrapper>,
    ) -> Result<T, WithSpan<ParseError>>,
) -> Result<T, DefinitionError> {
    let name = parse_entity_name(cursor).map_err(DefinitionError::unnamed)?;
    parse_rest(cursor, name).map_err(|error| DefinitionError {
        name: DefinitionName::Entity(name.item.0).wrap_some(),
        error,
    })
}

#[cfg_attr(not(test), expect(dead_code))]
fn consume_description(
    cursor: &mut TokenCursor<'_>,
) -> Result<Option<WithSpan<Description>>, WithSpan<ParseError>> {
    consume_string_value_if::<DescriptionValue>(cursor)
        .map(|description| description.map(|description| description.map(Description)))
}

fn parse_entity_name(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<EntityNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map(|name| name.interned().map(EntityNameWrapper))
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_scalar_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ScalarTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        ScalarTypeDefinition {
            description,
            name,
            directives: parse_directives(cursor)?,
        }
        .wrap_ok()
    })
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_object_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<ObjectTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        ObjectTypeDefinition {
            description,
            name,
            interfaces: parse_implements_interfaces(cursor)?,
            directives: parse_directives(cursor)?,
            fields: parse_fields_definition(cursor)?,
        }
        .wrap_ok()
    })
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_interface_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InterfaceTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        InterfaceTypeDefinition {
            description,
            name,
            interfaces: parse_implements_interfaces(cursor)?,
            directives: parse_directives(cursor)?,
            fields: parse_fields_definition(cursor)?,
        }
        .wrap_ok()
    })
}

fn parse_implements_interfaces(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>> {
    match cursor.consume_keyword_if("implements") {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_separated_nonempty(cursor, GraphQLTokenKind::Ampersand, parse_entity_name),
    }
}

fn parse_fields_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<FieldDefinition>>, WithSpan<ParseError>> {
    parse_optional_nonempty_group(
        cursor,
        GraphQLTokenKind::OpenBrace,
        GraphQLTokenKind::CloseBrace,
        |cursor| cursor.spanning(parse_field_definition),
    )
}

fn parse_field_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<FieldDefinition, WithSpan<ParseError>> {
    let description = consume_description(cursor)?;
    let name = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?
        .interned()
        .map(SelectableNameWrapper);
    let arguments = parse_arguments_definition(cursor)?;
    cursor
        .require_token(GraphQLTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?;
    FieldDefinition {
        description,
        name,
        arguments,
        type_: parse_type_annotation(cursor)?,
        directives: parse_directives(cursor)?,
    }
    .wrap_ok()
}

fn parse_arguments_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<InputValueDefinition>>, WithSpan<ParseError>> {
    parse_optional_nonempty_group(
        cursor,
        GraphQLTokenKind::OpenParenthesis,
        GraphQLTokenKind::CloseParenthesis,
        |cursor| cursor.spanning(parse_input_value_definition),
    )
}

fn parse_input_value_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<InputValueDefinition, WithSpan<ParseError>> {
    let description = consume_description(cursor)?;
    let name = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?
        .interned()
        .map(InputValueNameWrapper);
    cursor
        .require_token(GraphQLTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?;
    let type_ = parse_type_annotation(cursor)?;
    let default_value = match cursor.consume_token_if(GraphQLTokenKind::Equals) {
        Some(_) => parse_constant_value(cursor)?.wrap_some(),
        None => None,
    };
    InputValueDefinition {
        description,
        name,
        type_,
        default_value,
        directives: parse_directives(cursor)?,
    }
    .wrap_ok()
}

fn parse_separated_nonempty<T>(
    cursor: &mut TokenCursor<'_>,
    separator: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    cursor.consume_token_if(separator);
    let mut items = parse_item(cursor)?.wrap_vec();
    items.extend(
        std::iter::from_fn(|| {
            cursor
                .consume_token_if(separator)
                .map(|_| parse_item(cursor))
        })
        .collect::<Result<Vec<_>, _>>()?,
    );
    items.wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_union_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<UnionTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        UnionTypeDefinition {
            description,
            name,
            directives: parse_directives(cursor)?,
            member_types: parse_union_member_types(cursor)?,
        }
        .wrap_ok()
    })
}

fn parse_union_member_types(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EntityNameWrapper>>, WithSpan<ParseError>> {
    match cursor.consume_token_if(GraphQLTokenKind::Equals) {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_separated_nonempty(cursor, GraphQLTokenKind::Pipe, parse_entity_name),
    }
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_enum_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<EnumTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        EnumTypeDefinition {
            description,
            name,
            directives: parse_directives(cursor)?,
            values: parse_enum_values_definition(cursor)?,
        }
        .wrap_ok()
    })
}

fn parse_enum_values_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<EnumValueDefinition>>, WithSpan<ParseError>> {
    parse_optional_nonempty_group(
        cursor,
        GraphQLTokenKind::OpenBrace,
        GraphQLTokenKind::CloseBrace,
        |cursor| cursor.spanning(parse_enum_value_definition),
    )
}

fn parse_enum_value_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<EnumValueDefinition, WithSpan<ParseError>> {
    let description = consume_description(cursor)?;
    if let Some(word) = cursor.peek_identifier_of::<BooleanOrNullKeyword>() {
        return ParseError::EnumValueCannotBeTrueFalseOrNull
            .with_span(word.location)
            .wrap_err();
    }
    let value = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?;
    EnumValueDefinition {
        description,
        value: value.interned().map(EnumLiteralValueWrapper),
        directives: parse_directives(cursor)?,
    }
    .wrap_ok()
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_input_object_type_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<InputObjectTypeDefinition, DefinitionError> {
    parse_after_entity_name(cursor, |cursor, name| {
        InputObjectTypeDefinition {
            description,
            name,
            directives: parse_directives(cursor)?,
            fields: parse_input_fields_definition(cursor)?,
        }
        .wrap_ok()
    })
}

fn parse_input_fields_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<InputValueDefinition>>, WithSpan<ParseError>> {
    parse_optional_nonempty_group(
        cursor,
        GraphQLTokenKind::OpenBrace,
        GraphQLTokenKind::CloseBrace,
        |cursor| cursor.spanning(parse_input_value_definition),
    )
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_schema_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<SchemaDefinition, DefinitionError> {
    let directives = parse_directives(cursor).map_err(DefinitionError::unnamed)?;
    cursor
        .require_token(GraphQLTokenKind::OpenBrace)
        .map_err(|()| {
            DefinitionError::unnamed(
                cursor.expected(Expectation::Token(GraphQLTokenKind::OpenBrace)),
            )
        })?;
    SchemaDefinition {
        description,
        directives,
        root_operation_types: parse_root_operation_types_after_open(cursor)
            .map_err(DefinitionError::unnamed)?,
    }
    .wrap_ok()
}

fn parse_root_operation_types_after_open(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<RootOperationTypeDefinition>>, WithSpan<ParseError>> {
    parse_nonempty_group_after_open(cursor, GraphQLTokenKind::CloseBrace, |cursor| {
        cursor.spanning(parse_root_operation_type_definition)
    })
}

fn parse_root_operation_type_definition(
    cursor: &mut TokenCursor<'_>,
) -> Result<RootOperationTypeDefinition, WithSpan<ParseError>> {
    let operation_type = cursor
        .consume_identifier_of::<OperationType>()
        .ok_or_else(|| cursor.expected(Expectation::OperationType))?
        .map(OperationTypeWrapper);
    cursor
        .require_token(GraphQLTokenKind::Colon)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?;
    RootOperationTypeDefinition {
        operation_type,
        type_name: parse_entity_name(cursor)?,
    }
    .wrap_ok()
}

fn parse_after_directive_name<T>(
    cursor: &mut TokenCursor<'_>,
    parse_rest: impl FnOnce(
        &mut TokenCursor<'_>,
        WithSpan<DirectiveNameWrapper>,
    ) -> Result<T, WithSpan<ParseError>>,
) -> Result<T, DefinitionError> {
    let name = parse_directive_definition_name(cursor).map_err(DefinitionError::unnamed)?;
    parse_rest(cursor, name).map_err(|error| DefinitionError {
        name: DefinitionName::Directive(name.item.0).wrap_some(),
        error,
    })
}

#[cfg_attr(not(test), expect(dead_code))]
fn parse_directive_definition(
    cursor: &mut TokenCursor<'_>,
    description: Option<WithSpan<Description>>,
) -> Result<DirectiveDefinition, DefinitionError> {
    parse_after_directive_name(cursor, |cursor, name| {
        let arguments = parse_arguments_definition(cursor)?;
        let repeatable = match cursor.consume_keyword_if("repeatable") {
            Some(_) => DirectiveRepeatability::Repeatable,
            None => DirectiveRepeatability::NotRepeatable,
        };
        cursor
            .require_keyword("on")
            .map_err(|()| cursor.expected(Expectation::Keyword("on")))?;
        DirectiveDefinition {
            description,
            name,
            arguments,
            repeatable,
            locations: parse_separated_nonempty(
                cursor,
                GraphQLTokenKind::Pipe,
                parse_directive_location,
            )?,
        }
        .wrap_ok()
    })
}

fn parse_directive_definition_name(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<DirectiveNameWrapper>, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::At)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::At)))?;
    cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map(|name| name.interned().map(DirectiveNameWrapper))
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))
}

fn parse_directive_location(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<DirectiveLocationWrapper>, WithSpan<ParseError>> {
    cursor
        .consume_identifier_of::<DirectiveLocation>()
        .map(|location| location.map(DirectiveLocationWrapper))
        .ok_or_else(|| cursor.expected(Expectation::DirectiveLocation))
}

fn parse_directives(
    cursor: &mut TokenCursor<'_>,
) -> Result<Vec<WithSpan<Directive>>, WithSpan<ParseError>> {
    std::iter::from_fn(|| {
        cursor
            .peek()
            .is_some_and(|token| token.item == GraphQLTokenKind::At)
            .then(|| cursor.spanning(parse_directive))
    })
    .collect()
}

fn parse_directive(cursor: &mut TokenCursor<'_>) -> Result<Directive, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::At)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::At)))?;
    let name = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?
        .interned()
        .map(DirectiveNameWrapper);
    Directive {
        name,
        arguments: parse_optional_nonempty_group(
            cursor,
            GraphQLTokenKind::OpenParenthesis,
            GraphQLTokenKind::CloseParenthesis,
            parse_argument,
        )?,
    }
    .wrap_ok()
}

fn parse_argument(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<Argument>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        let name = cursor
            .require_token(GraphQLTokenKind::Identifier)
            .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?
            .interned()
            .map(ArgumentNameWrapper);
        cursor
            .require_token(GraphQLTokenKind::Colon)
            .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?;
        Argument {
            name,
            value: parse_constant_value(cursor)?,
        }
        .wrap_ok()
    })
}

/// `open item+ close`, or no items when `open` is absent.
fn parse_optional_nonempty_group<T>(
    cursor: &mut TokenCursor<'_>,
    open: GraphQLTokenKind,
    close: GraphQLTokenKind,
    parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    match cursor.consume_token_if(open) {
        None => Vec::new().wrap_ok(),
        Some(_) => parse_nonempty_group_after_open(cursor, close, parse_item),
    }
}

/// `item+ close`, the open already consumed.
fn parse_nonempty_group_after_open<T>(
    cursor: &mut TokenCursor<'_>,
    close: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    let mut items = parse_item(cursor)?.wrap_vec();
    items.extend(parse_group_after_open(cursor, close, parse_item)?);
    items.wrap_ok()
}

/// `item* close`, the open already consumed.
pub(crate) fn parse_group_after_open<T>(
    cursor: &mut TokenCursor<'_>,
    close: GraphQLTokenKind,
    mut parse_item: impl FnMut(&mut TokenCursor<'_>) -> Result<WithSpan<T>, WithSpan<ParseError>>,
) -> Result<Vec<WithSpan<T>>, WithSpan<ParseError>> {
    std::iter::from_fn(|| match cursor.consume_token_if(close) {
        Some(_) => None,
        None if cursor.at_end() => cursor
            .expected(Expectation::Token(close))
            .wrap_err()
            .wrap_some(),
        None => parse_item(cursor).wrap_some(),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use common_lang_types::{DescriptionValue, DirectiveName, EntityName};
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        DefinitionError, DefinitionName, consume_description, parse_directive_definition,
        parse_directives, parse_enum_type_definition, parse_input_object_type_definition,
        parse_interface_type_definition, parse_object_type_definition,
        parse_scalar_type_definition, parse_schema_definition, parse_union_type_definition,
    };
    use crate::span_of::span_of;
    use crate::test_cursor::with_cursor;
    use crate::token_cursor::TokenCursor;
    use crate::{
        ConstantValue, Description, Directive, DirectiveDefinition, DirectiveLocation,
        DirectiveRepeatability, EntityNameWrapper, EnumValueDefinition, Expectation,
        FieldDefinition, Found, GraphQLTokenKind, InputObjectTypeDefinition, IntegerValue,
        NullableTypeAnnotation, ObjectTypeDefinition, OperationType, ParseError, SchemaDefinition,
        TypeAnnotation, UnionTypeDefinition,
    };

    /// Runs `parse` on `text` after its description, when it has one, and its keyword.
    fn after_keyword<T>(
        text: &str,
        parse: impl FnOnce(
            &mut TokenCursor<'_>,
            Option<WithSpan<Description>>,
        ) -> Result<T, DefinitionError>,
    ) -> Result<WithSpan<T>, DefinitionError> {
        with_cursor(text, |cursor| {
            let description =
                consume_description(cursor).expect("the fixture's description decodes");
            cursor.advance();
            cursor.spanning(|cursor| parse(cursor, description))
        })
    }

    fn entity(name: &str) -> Option<DefinitionName> {
        DefinitionName::Entity(EntityName::from(name.intern())).wrap_some()
    }

    #[test]
    fn a_scalar_is_its_name_and_directives() {
        let scalar = after_keyword("scalar Date", parse_scalar_type_definition)
            .expect("the fixture is a scalar");
        assert_eq!(scalar.location, Span::new(7, 11));
        assert_eq!(scalar.item.name.item.0, "Date");
        assert!(scalar.item.directives.is_empty());
        assert_eq!(scalar.item.description, None);

        let scalar = after_keyword(
            "scalar Date @specifiedBy(url: \"https://example.com\")",
            parse_scalar_type_definition,
        )
        .expect("the fixture is a scalar");
        assert_eq!(scalar.item.directives.len(), 1);
        let directive = scalar.item.directives[0].item.reference();
        assert_eq!(directive.name.item.0, "specifiedBy");
        assert_eq!(directive.arguments.len(), 1);
        assert_eq!(directive.arguments[0].item.name.item.0, "url");
    }

    #[test]
    fn a_scalar_that_fails_before_its_name_is_unnamed() {
        assert_eq!(
            after_keyword("scalar", parse_scalar_type_definition),
            DefinitionError {
                name: None,
                error: ParseError::expected(
                    Expectation::Token(GraphQLTokenKind::Identifier),
                    Found::EndOfFile
                )
                .with_span(Span::new(6, 6)),
            }
            .wrap_err()
        );
        let error = after_keyword("scalar 1", parse_scalar_type_definition)
            .expect_err("the fixture has no name");
        assert_eq!(error.name, None);
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::IntegerLiteral)
            )
        );
    }

    #[test]
    fn a_scalar_that_fails_after_its_name_keeps_the_name() {
        let error = after_keyword("scalar Date @a(", parse_scalar_type_definition)
            .expect_err("the fixture has an unclosed directive");
        assert_eq!(error.name, entity("Date"));
    }

    #[test]
    fn descriptions_decode() {
        with_cursor("\"a date\"", |cursor| {
            let description = consume_description(cursor)
                .expect("the fixture decodes")
                .expect("the fixture is a description");
            assert_eq!(description.item.0, "a date");
            assert_eq!(description.location, Span::new(0, 8));
        });
        with_cursor("\"\"\"\n  a\n  date\n\"\"\"", |cursor| {
            let description = consume_description(cursor)
                .expect("the fixture decodes")
                .expect("the fixture is a description");
            assert_eq!(description.item.0, "a\ndate");
        });
        with_cursor("scalar", |cursor| {
            assert_eq!(consume_description(cursor), None.wrap_ok());
            assert!(!cursor.at_end());
        });
        with_cursor(r#""\uDEAD""#, |cursor| {
            assert_eq!(
                consume_description(cursor).map_err(|error| error.item),
                ParseError::InvalidUnicodeEscape.wrap_err()
            );
        });
    }

    fn directives(text: &str) -> Result<Vec<WithSpan<Directive>>, WithSpan<ParseError>> {
        with_cursor(text, parse_directives)
    }

    fn error(text: &str) -> ParseError {
        directives(text)
            .expect_err("the fixture is an invalid directive list")
            .item
    }

    #[test]
    fn directives_parse_until_the_next_non_directive() {
        let text = "@a @b(x: 1) rest";
        with_cursor(text, |cursor| {
            let directives = parse_directives(cursor).expect("the fixture is two directives");
            assert_eq!(directives.len(), 2);
            assert_eq!(directives[0].item.name.item.0, "a");
            assert!(directives[0].item.arguments.is_empty());
            assert_eq!(directives[1].item.name.item.0, "b");
            assert_eq!(directives[1].item.arguments.len(), 1);
            let argument = &directives[1].item.arguments[0].item;
            assert_eq!(argument.name.item.0, "x");
            assert_eq!(argument.value.item, ConstantValue::Int(IntegerValue(1)));
            assert_eq!(directives[1].location, span_of(text, "@b(x: 1)"));
            assert_eq!(
                cursor.peek().map(|token| token.location),
                span_of(text, "rest").wrap_some()
            );
        });
    }

    #[test]
    fn no_directive_consumes_nothing() {
        with_cursor("rest", |cursor| {
            assert_eq!(parse_directives(cursor), Vec::new().wrap_ok());
            assert_eq!(
                cursor.peek().map(|token| token.location),
                Span::new(0, 4).wrap_some()
            );
        });
    }

    #[test]
    fn invalid_directives_are_errors() {
        assert_eq!(
            error("@a()"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::CloseParenthesis)
            )
        );
        assert_eq!(
            error("@"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::EndOfFile
            )
        );
        assert_eq!(
            error("@a(x: 1"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::CloseParenthesis),
                Found::EndOfFile
            )
        );
    }

    fn object(text: &str) -> ObjectTypeDefinition {
        after_keyword(text, parse_object_type_definition)
            .expect("the fixture is an object type")
            .item
    }

    fn object_error(text: &str) -> DefinitionError {
        after_keyword(text, parse_object_type_definition)
            .expect_err("the fixture is an invalid object type")
    }

    fn names(names: &[WithSpan<EntityNameWrapper>]) -> Vec<&'static str> {
        names
            .iter()
            .map(|name| intern::Lookup::lookup(name.item.0))
            .collect()
    }

    fn field_names(fields: &[WithSpan<FieldDefinition>]) -> Vec<&'static str> {
        fields
            .iter()
            .map(|field| intern::Lookup::lookup(field.item.name.item.0))
            .collect()
    }

    /// The annotation written back as SDL.
    fn shape(annotation: &TypeAnnotation) -> String {
        match annotation {
            TypeAnnotation::Named(named) => named.0.item.0.to_string(),
            TypeAnnotation::List(list) => format!("[{}]", shape(list.0.item.reference())),
            TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
                NullableTypeAnnotation::Named(named) => format!("{}!", named.0.item.0),
                NullableTypeAnnotation::List(list) => {
                    format!("[{}]!", shape(list.0.item.reference()))
                }
            },
        }
    }

    #[test]
    fn an_object_has_its_fields() {
        for text in [
            "type Pet { id: ID! name: String }",
            "type Pet { id: ID!, name: String, }",
        ] {
            let pet = object(text);
            assert_eq!(pet.name.item.0, "Pet");
            assert_eq!(field_names(pet.fields.as_slice()), vec!["id", "name"]);
            assert_eq!(shape(pet.fields[0].item.type_.item.reference()), "ID!");
            assert_eq!(shape(pet.fields[1].item.type_.item.reference()), "String");
        }
    }

    #[test]
    fn an_object_without_braces_has_no_fields() {
        assert!(object("type Pet").fields.is_empty());
    }

    #[test]
    fn an_empty_or_unclosed_field_group_is_an_error() {
        assert_eq!(
            object_error("type Pet {}"),
            DefinitionError {
                name: entity("Pet"),
                error: ParseError::expected(
                    Expectation::Token(GraphQLTokenKind::Identifier),
                    Found::Token(GraphQLTokenKind::CloseBrace)
                )
                .with_span(Span::new(10, 11)),
            }
        );
        let error = object_error("type Pet {");
        assert_eq!(error.name, entity("Pet"));
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::EndOfFile
            )
        );
        let error = object_error("type Pet { id: ID!");
        assert_eq!(error.name, entity("Pet"));
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::CloseBrace),
                Found::EndOfFile
            )
        );
    }

    #[test]
    fn an_object_implements_interfaces() {
        assert_eq!(
            names(
                object("type Pet implements Node { id: ID! }")
                    .interfaces
                    .as_slice()
            ),
            "Node".wrap_vec()
        );
        assert_eq!(
            names(
                object("type Pet implements Node & Named { id: ID! }")
                    .interfaces
                    .as_slice()
            ),
            vec!["Node", "Named"]
        );
        assert_eq!(
            names(
                object("type Pet implements & Node { id: ID! }")
                    .interfaces
                    .as_slice()
            ),
            "Node".wrap_vec()
        );
        let error = object_error("type Pet implements { id: ID! }");
        assert_eq!(error.name, entity("Pet"));
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::OpenBrace)
            )
        );
    }

    #[test]
    fn an_object_and_its_fields_have_directives_and_descriptions() {
        assert_eq!(
            object("type Pet @key(fields: \"id\") { id: ID! }")
                .directives
                .len(),
            1
        );
        let pet = object("type Pet { \"the id\" id: ID! @deprecated(reason: \"no\") }");
        let id = pet.fields[0].item.reference();
        assert_eq!(
            id.description.map(|description| description.item.0),
            DescriptionValue::from("the id".intern()).wrap_some()
        );
        assert_eq!(id.directives.len(), 1);
        assert_eq!(id.directives[0].item.name.item.0, "deprecated");
    }

    #[test]
    fn a_field_has_arguments_with_defaults() {
        let pet = object("type Pet { name(lang: String = \"en\", upper: Boolean): String }");
        let arguments = pet.fields[0].item.arguments.as_slice();
        assert_eq!(arguments.len(), 2);
        assert_eq!(arguments[0].item.name.item.0, "lang");
        let Some(ConstantValue::String(default)) = arguments[0]
            .item
            .default_value
            .as_ref()
            .map(|value| value.item.clone())
        else {
            panic!("lang has a string default");
        };
        assert_eq!(default.0, "en");
        assert_eq!(arguments[1].item.name.item.0, "upper");
        assert_eq!(arguments[1].item.default_value, None);
    }

    #[test]
    fn an_empty_argument_group_or_missing_default_is_an_error() {
        let error = object_error("type Pet { name(): String }");
        assert_eq!(error.name, entity("Pet"));
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::CloseParenthesis)
            )
        );
        assert_eq!(
            object_error("type Pet { name(lang: String = ): String }")
                .error
                .item,
            ParseError::expected(
                Expectation::ConstantValue,
                Found::Token(GraphQLTokenKind::CloseParenthesis)
            )
        );
    }

    #[test]
    fn keywords_are_names() {
        assert_eq!(
            field_names(
                object("type Pet { type: String input: Int }")
                    .fields
                    .as_slice()
            ),
            vec!["type", "input"]
        );
        assert_eq!(object("type type { id: ID! }").name.item.0, "type");
    }

    #[test]
    fn an_interface_has_fields_and_interfaces() {
        let node = after_keyword(
            "interface Node { id: ID! }",
            parse_interface_type_definition,
        )
        .expect("the fixture is an interface")
        .item;
        assert_eq!(node.name.item.0, "Node");
        assert_eq!(field_names(node.fields.as_slice()), "id".wrap_vec());
        let named = after_keyword(
            "interface Named implements Node { id: ID! }",
            parse_interface_type_definition,
        )
        .expect("the fixture is an interface")
        .item;
        assert_eq!(names(named.interfaces.as_slice()), "Node".wrap_vec());
    }

    fn union(text: &str) -> UnionTypeDefinition {
        after_keyword(text, parse_union_type_definition)
            .expect("the fixture is a union")
            .item
    }

    #[test]
    fn a_union_has_its_members() {
        for text in ["union Search = Pet | User", "union Search = | Pet | User"] {
            assert_eq!(
                names(union(text).member_types.as_slice()),
                vec!["Pet", "User"],
                "for text {text:?}"
            );
        }
        assert_eq!(
            names(union("union Search = Pet").member_types.as_slice()),
            "Pet".wrap_vec()
        );
        assert!(union("union Search").member_types.is_empty());
        let directed = union("union Search @a");
        assert_eq!(directed.directives.len(), 1);
        assert!(directed.member_types.is_empty());
    }

    #[test]
    fn a_union_missing_a_member_is_an_error() {
        for text in ["union Search =", "union Search = |", "union Search = Pet |"] {
            let error = after_keyword(text, parse_union_type_definition)
                .expect_err("the fixture is missing a member");
            assert_eq!(error.name, entity("Search"), "for text {text:?}");
            assert_eq!(
                error.error.item,
                ParseError::expected(
                    Expectation::Token(GraphQLTokenKind::Identifier),
                    Found::EndOfFile
                ),
                "for text {text:?}"
            );
        }
    }

    fn enum_values(text: &str) -> Vec<WithSpan<EnumValueDefinition>> {
        after_keyword(text, parse_enum_type_definition)
            .expect("the fixture is an enum")
            .item
            .values
    }

    #[test]
    fn an_enum_has_its_values() {
        let values = enum_values("enum Status { AVAILABLE SOLD }");
        let names: Vec<_> = values
            .iter()
            .map(|value| intern::Lookup::lookup(value.item.value.item.0))
            .collect();
        assert_eq!(names, vec!["AVAILABLE", "SOLD"]);
        let described = enum_values("enum Status { \"on sale\" AVAILABLE @deprecated }");
        assert_eq!(described.len(), 1);
        assert_eq!(
            described[0]
                .item
                .description
                .map(|description| description.item.0),
            DescriptionValue::from("on sale".intern()).wrap_some()
        );
        assert_eq!(described[0].item.directives.len(), 1);
        assert!(enum_values("enum Status").is_empty());
        assert_eq!(
            after_keyword("enum Status {}", parse_enum_type_definition)
                .expect_err("the fixture has an empty value group")
                .error
                .item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::CloseBrace)
            )
        );
    }

    #[test]
    fn an_enum_value_cannot_be_true_false_or_null() {
        for word in ["true", "false", "null"] {
            let text = format!("enum Status {{ {word} }}");
            let error = after_keyword(text.as_str(), parse_enum_type_definition)
                .expect_err("the fixture's value is a reserved word");
            assert_eq!(error.name, entity("Status"), "for word {word:?}");
            assert_eq!(
                error.error,
                ParseError::EnumValueCannotBeTrueFalseOrNull
                    .with_span(span_of(text.as_str(), word)),
                "for word {word:?}"
            );
        }
        assert_eq!(enum_values("enum Status { nullable }").len(), 1);
    }

    fn input_object(text: &str) -> InputObjectTypeDefinition {
        after_keyword(text, parse_input_object_type_definition)
            .expect("the fixture is an input object")
            .item
    }

    #[test]
    fn an_input_object_has_its_fields() {
        let pet = input_object("input PetInput { name: String = \"x\" tags: [String!] @a }");
        assert_eq!(pet.fields.len(), 2);
        let name = pet.fields[0].item.reference();
        assert_eq!(name.name.item.0, "name");
        let Some(ConstantValue::String(default)) =
            name.default_value.as_ref().map(|value| value.item.clone())
        else {
            panic!("name has a string default");
        };
        assert_eq!(default.0, "x");
        let tags = pet.fields[1].item.reference();
        assert_eq!(tags.name.item.0, "tags");
        assert_eq!(tags.directives.len(), 1);
        assert!(input_object("input PetInput").fields.is_empty());
        assert_eq!(
            after_keyword("input PetInput {}", parse_input_object_type_definition)
                .expect_err("the fixture has an empty field group")
                .name,
            entity("PetInput")
        );
    }

    #[test]
    fn an_input_field_default_is_any_constant_value() {
        let input = input_object("input I { a: I = { b: [1, 2], c: ENUM } }");
        let Some(ConstantValue::Object(object)) = input.fields[0]
            .item
            .default_value
            .as_ref()
            .map(|value| value.item.clone())
        else {
            panic!("a has an object default");
        };
        assert_eq!(object.0.len(), 2);
        assert_eq!(object.0[0].item.name.item.0, "b");
        assert!(matches!(
            object.0[0].item.value.item,
            ConstantValue::List(_)
        ));
        assert_eq!(object.0[1].item.name.item.0, "c");
        assert!(matches!(
            object.0[1].item.value.item,
            ConstantValue::Enum(_)
        ));
    }

    fn schema(text: &str) -> SchemaDefinition {
        after_keyword(text, parse_schema_definition)
            .expect("the fixture is a schema definition")
            .item
    }

    fn schema_error(text: &str) -> DefinitionError {
        after_keyword(text, parse_schema_definition)
            .expect_err("the fixture is an invalid schema definition")
    }

    fn root_operation_types(schema: &SchemaDefinition) -> Vec<(OperationType, &'static str)> {
        schema
            .root_operation_types
            .iter()
            .map(|root| {
                (
                    root.item.operation_type.item.0,
                    intern::Lookup::lookup(root.item.type_name.item.0),
                )
            })
            .collect()
    }

    #[test]
    fn a_schema_lists_its_root_operation_types() {
        assert_eq!(
            root_operation_types(
                schema("schema { query: Query mutation: Mutation subscription: Subscription }")
                    .reference()
            ),
            vec![
                (OperationType::Query, "Query"),
                (OperationType::Mutation, "Mutation"),
                (OperationType::Subscription, "Subscription"),
            ]
        );
        let described = schema("\"the schema\" schema @a { query: Query }");
        assert_eq!(
            described.description.map(|description| description.item.0),
            DescriptionValue::from("the schema".intern()).wrap_some()
        );
        assert_eq!(described.directives.len(), 1);
        assert_eq!(
            root_operation_types(schema("schema { query: Query query: Other }").reference()),
            vec![
                (OperationType::Query, "Query"),
                (OperationType::Query, "Other")
            ]
        );
    }

    #[test]
    fn an_invalid_schema_is_an_unnamed_error() {
        assert_eq!(
            schema_error("schema {}"),
            DefinitionError {
                name: None,
                error: ParseError::expected(
                    Expectation::OperationType,
                    Found::Token(GraphQLTokenKind::CloseBrace)
                )
                .with_span(Span::new(8, 9)),
            }
        );
        let error = schema_error("schema");
        assert_eq!(error.name, None);
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::OpenBrace),
                Found::EndOfFile
            )
        );
        let text = "schema { foo: Query }";
        assert_eq!(
            schema_error(text).error,
            ParseError::expected(
                Expectation::OperationType,
                Found::Token(GraphQLTokenKind::Identifier)
            )
            .with_span(span_of(text, "foo"))
        );
    }

    fn directive_definition(text: &str) -> DirectiveDefinition {
        after_keyword(text, parse_directive_definition)
            .expect("the fixture is a directive definition")
            .item
    }

    fn directive_error(text: &str) -> DefinitionError {
        after_keyword(text, parse_directive_definition)
            .expect_err("the fixture is an invalid directive definition")
    }

    fn locations(definition: &DirectiveDefinition) -> Vec<DirectiveLocation> {
        definition
            .locations
            .iter()
            .map(|location| location.item.0)
            .collect()
    }

    fn directive_named(name: &str) -> Option<DefinitionName> {
        DefinitionName::Directive(DirectiveName::from(name.intern())).wrap_some()
    }

    #[test]
    fn a_directive_definition_has_its_parts() {
        let one_of = directive_definition("directive @oneOf on INPUT_OBJECT");
        assert_eq!(one_of.name.item.0, "oneOf");
        assert!(one_of.arguments.is_empty());
        assert_eq!(one_of.repeatable, DirectiveRepeatability::NotRepeatable);
        assert_eq!(
            locations(one_of.reference()),
            DirectiveLocation::InputObject.wrap_vec()
        );
        for text in [
            "directive @a(x: Int = 1) repeatable on FIELD | OBJECT",
            "directive @a on | FIELD | OBJECT",
        ] {
            let definition = directive_definition(text);
            assert_eq!(
                locations(definition.reference()),
                vec![DirectiveLocation::Field, DirectiveLocation::Object],
                "for text {text:?}"
            );
        }
        let repeatable = directive_definition("directive @a(x: Int = 1) repeatable on FIELD");
        assert_eq!(repeatable.arguments.len(), 1);
        assert_eq!(repeatable.repeatable, DirectiveRepeatability::Repeatable);
    }

    #[test]
    fn every_directive_location_parses() {
        let definition = directive_definition(
            "directive @a on QUERY | MUTATION | SUBSCRIPTION | FIELD | FRAGMENT_DEFINITION | FRAGMENT_SPREAD | INLINE_FRAGMENT | VARIABLE_DEFINITION | SCHEMA | SCALAR | OBJECT | FIELD_DEFINITION | ARGUMENT_DEFINITION | INTERFACE | UNION | ENUM | ENUM_VALUE | INPUT_OBJECT | INPUT_FIELD_DEFINITION",
        );
        assert_eq!(
            locations(definition.reference()),
            vec![
                DirectiveLocation::Query,
                DirectiveLocation::Mutation,
                DirectiveLocation::Subscription,
                DirectiveLocation::Field,
                DirectiveLocation::FragmentDefinition,
                DirectiveLocation::FragmentSpread,
                DirectiveLocation::InlineFragment,
                DirectiveLocation::VariableDefinition,
                DirectiveLocation::Schema,
                DirectiveLocation::Scalar,
                DirectiveLocation::Object,
                DirectiveLocation::FieldDefinition,
                DirectiveLocation::ArgumentDefinition,
                DirectiveLocation::Interface,
                DirectiveLocation::Union,
                DirectiveLocation::Enum,
                DirectiveLocation::EnumValue,
                DirectiveLocation::InputObject,
                DirectiveLocation::InputFieldDefinition,
            ]
        );
    }

    #[test]
    fn an_unknown_directive_location_is_an_error() {
        for (text, word) in [
            ("directive @a on FOO", "FOO"),
            ("directive @a on field", "field"),
        ] {
            assert_eq!(
                directive_error(text),
                DefinitionError {
                    name: directive_named("a"),
                    error: ParseError::expected(
                        Expectation::DirectiveLocation,
                        Found::Token(GraphQLTokenKind::Identifier)
                    )
                    .with_span(span_of(text, word)),
                }
            );
        }
    }

    #[test]
    fn an_invalid_directive_definition_is_an_error() {
        assert_eq!(
            directive_error("directive @a FIELD").error.item,
            ParseError::expected(
                Expectation::Keyword("on"),
                Found::Token(GraphQLTokenKind::Identifier)
            )
        );
        assert_eq!(
            directive_error("directive @a on").error.item,
            ParseError::expected(Expectation::DirectiveLocation, Found::EndOfFile)
        );
        assert_eq!(
            directive_error("directive @a() on FIELD").error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Identifier),
                Found::Token(GraphQLTokenKind::CloseParenthesis)
            )
        );
        let error = directive_error("directive a on FIELD");
        assert_eq!(error.name, None);
        assert_eq!(
            error.error.item,
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::At),
                Found::Token(GraphQLTokenKind::Identifier)
            )
        );
    }

    #[test]
    fn a_directive_named_like_a_keyword_parses() {
        let text = "directive @type on FIELD";
        let definition = directive_definition(text);
        assert_eq!(definition.name.item.0, "type");
        assert_eq!(definition.name.location, span_of(text, "type"));
    }
}
