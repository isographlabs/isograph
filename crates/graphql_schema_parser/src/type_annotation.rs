use prelude::Postfix;
use span::WithSpan;

use crate::token_cursor::TokenCursor;
use crate::{EntityNameWrapper, Expectation, GraphQLTokenKind, ParseError};

/// Spec 2.12 `Type`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
    NonNull(NonNullTypeAnnotation),
}

/// Spec 2.12 `NonNullType`: a named or list type followed by `!`. The span of the inner
/// type excludes the `!`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonNullTypeAnnotation(pub WithSpan<NullableTypeAnnotation>);

/// The type a `NonNullType` wraps: spec 2.12 `NamedType` or `ListType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NullableTypeAnnotation {
    Named(NamedTypeAnnotation),
    List(Box<ListTypeAnnotation>),
}

/// Spec 2.12 `NamedType`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NamedTypeAnnotation(pub WithSpan<EntityNameWrapper>);

/// Spec 2.12 `ListType : [ Type ]`. The span includes the brackets.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListTypeAnnotation(pub WithSpan<TypeAnnotation>);

impl NullableTypeAnnotation {
    pub(crate) fn into_type_annotation(self) -> TypeAnnotation {
        match self {
            NullableTypeAnnotation::Named(named) => TypeAnnotation::Named(named),
            NullableTypeAnnotation::List(list) => TypeAnnotation::List(list),
        }
    }
}

pub(crate) fn parse_type_annotation(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        let nullable = parse_nullable_type_annotation(cursor)?;
        match cursor.consume_token_if(GraphQLTokenKind::Exclamation) {
            Some(_) => TypeAnnotation::NonNull(NonNullTypeAnnotation(nullable)).wrap_ok(),
            None => nullable.item.into_type_annotation().wrap_ok(),
        }
    })
}

fn parse_nullable_type_annotation(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<NullableTypeAnnotation>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(name) = cursor.consume_token_if(GraphQLTokenKind::Identifier) {
            return NullableTypeAnnotation::Named(NamedTypeAnnotation(
                name.interned().map(EntityNameWrapper),
            ))
            .wrap_ok();
        }
        if cursor
            .consume_token_if(GraphQLTokenKind::OpenBracket)
            .is_some()
        {
            let inner = parse_type_annotation(cursor)?;
            cursor
                .require_token(GraphQLTokenKind::CloseBracket)
                .map_err(|()| {
                    cursor.expected(Expectation::Token(GraphQLTokenKind::CloseBracket))
                })?;
            return NullableTypeAnnotation::List(ListTypeAnnotation(inner).boxed()).wrap_ok();
        }
        cursor.expected(Expectation::TypeAnnotation).wrap_err()
    })
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;
    use span::{Span, WithSpan};

    use super::{
        ListTypeAnnotation, NonNullTypeAnnotation, NullableTypeAnnotation, TypeAnnotation,
        parse_type_annotation,
    };
    use crate::test_cursor::with_cursor;
    use crate::{Expectation, Found, GraphQLTokenKind, ParseError};

    fn parsed(text: &str) -> Result<WithSpan<TypeAnnotation>, WithSpan<ParseError>> {
        with_cursor(text, parse_type_annotation)
    }

    fn annotation(text: &str) -> WithSpan<TypeAnnotation> {
        parsed(text).expect("the fixture is a type annotation")
    }

    fn error(text: &str) -> ParseError {
        parsed(text)
            .expect_err("the fixture is an invalid type annotation")
            .item
    }

    #[test]
    fn a_named_type_is_named() {
        let parsed = annotation("String");
        assert_eq!(parsed.location, Span::new(0, 6));
        let TypeAnnotation::Named(name) = parsed.item else {
            panic!("the fixture is a named type");
        };
        assert_eq!(name.0.item.0, "String");
        assert_eq!(name.0.location, Span::new(0, 6));
    }

    #[test]
    fn a_non_null_type_excludes_the_exclamation_from_its_inner_span() {
        let parsed = annotation("String!");
        assert_eq!(parsed.location, Span::new(0, 7));
        let TypeAnnotation::NonNull(NonNullTypeAnnotation(inner)) = parsed.item else {
            panic!("the fixture is a non-null type");
        };
        assert_eq!(inner.location, Span::new(0, 6));
        let NullableTypeAnnotation::Named(name) = inner.item else {
            panic!("the fixture wraps a named type");
        };
        assert_eq!(name.0.item.0, "String");
    }

    #[test]
    fn a_list_type_spans_its_brackets() {
        let parsed = annotation("[String]");
        assert_eq!(parsed.location, Span::new(0, 8));
        let TypeAnnotation::List(list) = parsed.item else {
            panic!("the fixture is a list type");
        };
        let ListTypeAnnotation(inner) = *list;
        let TypeAnnotation::Named(name) = inner.item else {
            panic!("the fixture lists a named type");
        };
        assert_eq!(name.0.item.0, "String");
    }

    #[test]
    fn a_non_null_list_of_non_null_nests() {
        let TypeAnnotation::NonNull(NonNullTypeAnnotation(outer)) = annotation("[String!]!").item
        else {
            panic!("the fixture is a non-null type");
        };
        let NullableTypeAnnotation::List(list) = outer.item else {
            panic!("the fixture wraps a list");
        };
        let TypeAnnotation::NonNull(NonNullTypeAnnotation(inner)) = list.0.item else {
            panic!("the list holds a non-null type");
        };
        let NullableTypeAnnotation::Named(name) = inner.item else {
            panic!("the non-null type wraps a named type");
        };
        assert_eq!(name.0.item.0, "String");
    }

    #[test]
    fn lists_nest() {
        let TypeAnnotation::List(outer) = annotation("[[Int]]").item else {
            panic!("the fixture is a list");
        };
        let TypeAnnotation::List(inner) = outer.0.item else {
            panic!("the fixture nests a list");
        };
        let TypeAnnotation::Named(name) = inner.0.item else {
            panic!("the inner list holds a named type");
        };
        assert_eq!(name.0.item.0, "Int");
    }

    #[test]
    fn a_second_exclamation_is_left_on_the_cursor() {
        with_cursor("String!!", |cursor| {
            let parsed = parse_type_annotation(cursor).expect("the fixture starts with String!");
            assert_eq!(parsed.location, Span::new(0, 7));
            assert_eq!(
                cursor.peek().map(|token| token.item),
                GraphQLTokenKind::Exclamation.wrap_some()
            );
        });
    }

    #[test]
    fn a_token_that_starts_no_type_is_an_error() {
        assert_eq!(
            error("!"),
            ParseError::expected(
                Expectation::TypeAnnotation,
                Found::Token(GraphQLTokenKind::Exclamation)
            )
        );
        assert_eq!(
            error("]"),
            ParseError::expected(
                Expectation::TypeAnnotation,
                Found::Token(GraphQLTokenKind::CloseBracket)
            )
        );
        assert_eq!(
            error(""),
            ParseError::expected(Expectation::TypeAnnotation, Found::EndOfFile)
        );
    }

    #[test]
    fn an_unclosed_or_empty_list_is_an_error() {
        assert_eq!(
            error("[String"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::CloseBracket),
                Found::EndOfFile
            )
        );
        assert_eq!(
            error("[]"),
            ParseError::expected(
                Expectation::TypeAnnotation,
                Found::Token(GraphQLTokenKind::CloseBracket)
            )
        );
    }
}
