use prelude::Postfix;
use span::WithSpan;

use crate::token_cursor::TokenCursor;
use crate::value::parse_constant_value;
use crate::{
    Argument, ArgumentNameWrapper, Directive, DirectiveNameWrapper, Expectation, GraphQLTokenKind,
    ParseError,
};

#[cfg_attr(not(test), expect(dead_code))]
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
    use prelude::Postfix;
    use span::{Span, WithSpan};

    use super::parse_directives;
    use crate::span_of::span_of;
    use crate::test_cursor::with_cursor;
    use crate::{
        ConstantValue, Directive, Expectation, Found, GraphQLTokenKind, IntegerValue, ParseError,
    };

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
}
