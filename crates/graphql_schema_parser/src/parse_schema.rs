use common_lang_types::{DescriptionValue, DirectiveName, EntityName};
use prelude::Postfix;
use span::WithSpan;

use crate::string_value::consume_string_value_if;
use crate::token_cursor::TokenCursor;
use crate::type_annotation::parse_type_annotation;
use crate::value::parse_constant_value;
use crate::{
    Argument, ArgumentNameWrapper, Description, Directive, DirectiveNameWrapper, EntityNameWrapper,
    Expectation, FieldDefinition, GraphQLTokenKind, InputValueDefinition, InputValueNameWrapper,
    InterfaceTypeDefinition, ObjectTypeDefinition, ParseError, ScalarTypeDefinition,
    SelectableNameWrapper,
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
    use common_lang_types::{DescriptionValue, EntityName};
    use intern::string_key::Intern;
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        DefinitionError, DefinitionName, consume_description, parse_directives,
        parse_interface_type_definition, parse_object_type_definition,
        parse_scalar_type_definition,
    };
    use crate::span_of::span_of;
    use crate::test_cursor::with_cursor;
    use crate::token_cursor::TokenCursor;
    use crate::{
        ConstantValue, Description, Directive, EntityNameWrapper, Expectation, FieldDefinition,
        Found, GraphQLTokenKind, IntegerValue, NullableTypeAnnotation, ObjectTypeDefinition,
        ParseError, TypeAnnotation,
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
}
