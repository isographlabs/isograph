use common_lang_types::{EnumLiteralValue, StringLiteralValue};
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::ast::ValueKeyNameWrapper;
use crate::keyword::BooleanOrNullKeyword;
use crate::parse_schema::parse_group_after_open;
use crate::string_value::consume_string_value_if;
use crate::token_cursor::TokenCursor;
use crate::{Expectation, GraphQLTokenKind, ParseError};

/// Spec 2.10 `Value[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ConstantValue {
    Int(IntegerValue),
    Float(FloatValue),
    String(StringLiteralValueWrapper),
    Boolean(BooleanValue),
    Null(NullValue),
    Enum(EnumLiteralValueWrapper),
    List(ListValue),
    Object(ObjectValue),
}

/// Spec 2.10.7 `ListValue[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ListValue(pub Vec<WithSpan<ConstantValue>>);

/// Spec 2.10.8 `ObjectValue[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectValue(pub Vec<WithSpan<ObjectField>>);

/// Spec 2.10.8 `ObjectField[Const] : Name : Value[Const]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectField {
    pub name: WithSpan<ValueKeyNameWrapper>,
    pub value: WithSpan<ConstantValue>,
}

/// Spec 2.10.1 `IntValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct IntegerValue(pub i64);

/// Spec 2.10.2 `FloatValue`, stored as its bits so the tree is `Eq`.
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub struct FloatValue(u64);

impl FloatValue {
    pub fn new(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub fn as_float(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl std::fmt::Debug for FloatValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{}", self.as_float()))
    }
}

/// Spec 2.10.3 `BooleanValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct BooleanValue(pub Boolean);

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Boolean {
    True,
    False,
}

/// Spec 2.10.5 `NullValue`.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct NullValue;

/// Spec 2.10.4 `StringValue`, decoded.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct StringLiteralValueWrapper(pub StringLiteralValue);

/// Spec 2.10.6 `EnumValue`: a constant enum value, or the value an enum value definition
/// defines.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct EnumLiteralValueWrapper(pub EnumLiteralValue);

pub(crate) fn parse_constant_value(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        if let Some(value) = consume_string_value_if::<StringLiteralValue>(cursor)? {
            return ConstantValue::String(StringLiteralValueWrapper(value.item)).wrap_ok();
        }
        match cursor.peek().map(|token| token.item) {
            Some(GraphQLTokenKind::IntegerLiteral) => {
                ConstantValue::Int(parse_integer_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::FloatLiteral) => {
                ConstantValue::Float(parse_float_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::Identifier) => parse_name_value(cursor),
            Some(GraphQLTokenKind::OpenBracket) => {
                ConstantValue::List(parse_list_value(cursor)?).wrap_ok()
            }
            Some(GraphQLTokenKind::OpenBrace) => {
                ConstantValue::Object(parse_object_value(cursor)?).wrap_ok()
            }
            _ => cursor.expected(Expectation::ConstantValue).wrap_err(),
        }
    })
}

fn parse_integer_value(cursor: &mut TokenCursor<'_>) -> Result<IntegerValue, WithSpan<ParseError>> {
    let token = cursor
        .require_token(GraphQLTokenKind::IntegerLiteral)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::IntegerLiteral)))?;
    token
        .text()
        .parse::<i64>()
        .map(IntegerValue)
        .map_err(|_| ParseError::IntegerDoesNotFitI64.with_span(token.location))
}

fn parse_float_value(cursor: &mut TokenCursor<'_>) -> Result<FloatValue, WithSpan<ParseError>> {
    let token = cursor
        .require_token(GraphQLTokenKind::FloatLiteral)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::FloatLiteral)))?;
    token
        .text()
        .parse::<f64>()
        .map(FloatValue::new)
        .map_err(|_| ParseError::InvalidFloat.with_span(token.location))
}

fn parse_name_value(cursor: &mut TokenCursor<'_>) -> Result<ConstantValue, WithSpan<ParseError>> {
    if let Some(keyword) = cursor.consume_identifier_of::<BooleanOrNullKeyword>() {
        return match keyword.item {
            BooleanOrNullKeyword::True => ConstantValue::Boolean(BooleanValue(Boolean::True)),
            BooleanOrNullKeyword::False => ConstantValue::Boolean(BooleanValue(Boolean::False)),
            BooleanOrNullKeyword::Null => ConstantValue::Null(NullValue),
        }
        .wrap_ok();
    }
    let name = cursor
        .require_token(GraphQLTokenKind::Identifier)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?;
    ConstantValue::Enum(EnumLiteralValueWrapper(name.interned().item)).wrap_ok()
}

fn parse_list_value(cursor: &mut TokenCursor<'_>) -> Result<ListValue, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::OpenBracket)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::OpenBracket)))?;
    parse_group_after_open(cursor, GraphQLTokenKind::CloseBracket, parse_constant_value)
        .map(ListValue)
}

fn parse_object_value(cursor: &mut TokenCursor<'_>) -> Result<ObjectValue, WithSpan<ParseError>> {
    cursor
        .require_token(GraphQLTokenKind::OpenBrace)
        .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::OpenBrace)))?;
    parse_group_after_open(cursor, GraphQLTokenKind::CloseBrace, parse_object_field)
        .map(ObjectValue)
}

fn parse_object_field(
    cursor: &mut TokenCursor<'_>,
) -> Result<WithSpan<ObjectField>, WithSpan<ParseError>> {
    cursor.spanning(|cursor| {
        let name = cursor
            .require_token(GraphQLTokenKind::Identifier)
            .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Identifier)))?
            .interned()
            .map(ValueKeyNameWrapper);
        cursor
            .require_token(GraphQLTokenKind::Colon)
            .map_err(|()| cursor.expected(Expectation::Token(GraphQLTokenKind::Colon)))?;
        ObjectField {
            name,
            value: parse_constant_value(cursor)?,
        }
        .wrap_ok()
    })
}

#[cfg(test)]
mod tests {
    use prelude::Postfix;
    use span::WithSpan;

    use super::{
        Boolean, BooleanValue, ConstantValue, IntegerValue, NullValue, parse_constant_value,
    };
    use crate::span_of::span_of;
    use crate::test_cursor::with_cursor;
    use crate::{Expectation, Found, GraphQLTokenKind, ParseError};

    fn parsed(text: &str) -> Result<WithSpan<ConstantValue>, WithSpan<ParseError>> {
        with_cursor(text, parse_constant_value)
    }

    fn value(text: &str) -> ConstantValue {
        parsed(text).expect("the fixture is a constant value").item
    }

    fn error(text: &str) -> ParseError {
        parsed(text)
            .expect_err("the fixture is an invalid constant value")
            .item
    }

    #[test]
    fn integers_parse() {
        assert_eq!(value("1"), ConstantValue::Int(IntegerValue(1)));
        assert_eq!(value("-7"), ConstantValue::Int(IntegerValue(-7)));
        let text = "9223372036854775808";
        let error = parsed(text).expect_err("the fixture overflows i64");
        assert_eq!(error.item, ParseError::IntegerDoesNotFitI64);
        assert_eq!(error.location, span_of(text, text));
    }

    #[test]
    fn floats_parse() {
        let ConstantValue::Float(float) = value("1.5") else {
            panic!("the fixture is a float");
        };
        assert_eq!(float.as_float(), 1.5);
        let ConstantValue::Float(float) = value("1e400") else {
            panic!("the fixture is a float");
        };
        assert!(float.as_float().is_infinite());
    }

    #[test]
    fn strings_and_block_strings_are_string_values() {
        let ConstantValue::String(string) = value("\"hi\"") else {
            panic!("the fixture is a string");
        };
        assert_eq!(string.0, "hi");
        let ConstantValue::String(block) = value("\"\"\"hi\"\"\"") else {
            panic!("the fixture is a block string");
        };
        assert_eq!(block.0, "hi");
        let ConstantValue::String(escaped) = value(r#""a\nb""#) else {
            panic!("the fixture is a string");
        };
        assert_eq!(escaped.0, "a\nb");
    }

    #[test]
    fn names_are_booleans_null_or_enum_values() {
        assert_eq!(
            value("true"),
            ConstantValue::Boolean(BooleanValue(Boolean::True))
        );
        assert_eq!(
            value("false"),
            ConstantValue::Boolean(BooleanValue(Boolean::False))
        );
        assert_eq!(value("null"), ConstantValue::Null(NullValue));
        let ConstantValue::Enum(available) = value("AVAILABLE") else {
            panic!("the fixture is an enum value");
        };
        assert_eq!(available.0, "AVAILABLE");
        let ConstantValue::Enum(nullable) = value("nullable") else {
            panic!("the fixture is an enum value");
        };
        assert_eq!(nullable.0, "nullable");
    }

    #[test]
    fn lists_parse() {
        let ConstantValue::List(empty) = value("[]") else {
            panic!("the fixture is a list");
        };
        assert!(empty.0.is_empty());
        let ConstantValue::List(list) = value("[1, \"a\", [true]]") else {
            panic!("the fixture is a list");
        };
        assert_eq!(list.0.len(), 3);
        let ConstantValue::List(inner) = list.0[2].item.reference() else {
            panic!("the third item is a list");
        };
        assert_eq!(inner.0.len(), 1);
        assert_eq!(
            error("[1"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::CloseBracket),
                Found::EndOfFile
            )
        );
    }

    #[test]
    fn objects_parse() {
        let ConstantValue::Object(empty) = value("{}") else {
            panic!("the fixture is an object");
        };
        assert!(empty.0.is_empty());
        let ConstantValue::Object(object) = value("{ a: 1, b: { c: null } }") else {
            panic!("the fixture is an object");
        };
        assert_eq!(object.0.len(), 2);
        assert_eq!(object.0[0].item.name.item.0, "a");
        assert_eq!(object.0[1].item.name.item.0, "b");
        let ConstantValue::Object(inner) = object.0[1].item.value.item.reference() else {
            panic!("b holds an object");
        };
        assert_eq!(inner.0.len(), 1);
        assert_eq!(inner.0[0].item.name.item.0, "c");
        assert_eq!(
            error("{ a 1 }"),
            ParseError::expected(
                Expectation::Token(GraphQLTokenKind::Colon),
                Found::Token(GraphQLTokenKind::IntegerLiteral)
            )
        );
    }

    #[test]
    fn a_token_that_starts_no_value_is_an_error() {
        assert_eq!(
            error("}"),
            ParseError::expected(
                Expectation::ConstantValue,
                Found::Token(GraphQLTokenKind::CloseBrace)
            )
        );
        assert_eq!(
            error(""),
            ParseError::expected(Expectation::ConstantValue, Found::EndOfFile)
        );
        assert_eq!(
            error("$x"),
            ParseError::expected(
                Expectation::ConstantValue,
                Found::Token(GraphQLTokenKind::Error)
            )
        );
    }

    #[test]
    fn values_span_their_text() {
        let text = "[1, 2]";
        let list = parsed(text).expect("the fixture is a list");
        assert_eq!(list.location, span_of(text, text));
        let ConstantValue::List(items) = list.item else {
            panic!("the fixture is a list");
        };
        let locations: Vec<_> = items.0.iter().map(|item| item.location).collect();
        assert_eq!(locations, vec![span_of(text, "1"), span_of(text, "2")]);
        let string = "\"hi\"";
        assert_eq!(
            parsed(string).map(|value| value.location),
            span_of(string, string).wrap_ok()
        );
    }
}
