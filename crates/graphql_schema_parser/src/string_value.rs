use std::collections::VecDeque;

use intern::string_key::Intern;
use logos::Logos;
use nonempty::NonEmpty;
use prelude::Postfix;
use span::{Span, WithSpan, WithSpanPostfix};

use crate::token_cursor::{TokenCursor, TokenText};
use crate::token_kind::{StringToken, VariableWidthEscape};
use crate::{GraphQLTokenKind, ParseError};

pub(crate) fn consume_string_value_if<T: From<intern::string_key::StringKey>>(
    cursor: &mut TokenCursor<'_>,
) -> Result<Option<WithSpan<T>>, WithSpan<ParseError>> {
    if let Some(string) = cursor.consume_token_if(GraphQLTokenKind::StringLiteral) {
        let value = decode_string_value(string.exclude_ends(1))?;
        return value
            .intern()
            .to::<T>()
            .with_span(string.location)
            .wrap_some()
            .wrap_ok();
    }
    cursor
        .consume_token_if(GraphQLTokenKind::BlockStringLiteral)
        .map(|block| {
            intern_block_string_value::<T>(block.exclude_ends(3).text()).with_span(block.location)
        })
        .wrap_ok()
}

fn decode_string_value(interior: TokenText<'_>) -> Result<String, WithSpan<ParseError>> {
    let mut pieces = StringToken::lexer(interior.text()).spanned().peekable();
    let mut value = String::new();
    while let Some((piece, range)) = pieces.next() {
        let location = interior.sub_span(range.clone());
        match piece {
            StringToken::EscapedQuote => value.push('"'),
            StringToken::EscapedBackslash => value.push('\\'),
            StringToken::EscapedSlash => value.push('/'),
            StringToken::EscapedBackspace => value.push('\u{0008}'),
            StringToken::EscapedFormFeed => value.push('\u{000C}'),
            StringToken::EscapedLineFeed => value.push('\n'),
            StringToken::EscapedCarriageReturn => value.push('\r'),
            StringToken::EscapedTab => value.push('\t'),
            StringToken::EscapedUnicode(first) => {
                let mut units = NonEmpty::new(first.with_span(location));
                while let Some((StringToken::EscapedUnicode(unit), range)) =
                    pieces.next_if(|(piece, _)| matches!(piece, StringToken::EscapedUnicode(_)))
                {
                    units.push(unit.with_span(interior.sub_span(range)));
                }
                value.push_str(&decode_code_units(units.reference())?);
            }
            StringToken::EscapedVariableWidthUnicode(VariableWidthEscape::Scalar(scalar)) => {
                value.push(scalar)
            }
            StringToken::EscapedVariableWidthUnicode(VariableWidthEscape::NotAScalar) => {
                return ParseError::InvalidUnicodeEscape
                    .with_span(location)
                    .wrap_err();
            }
            StringToken::StringCharacters
            | StringToken::Quote
            | StringToken::LineTerminator
            | StringToken::Error => value.push_str(&interior.text()[range]),
        }
    }
    value.wrap_ok()
}

fn decode_code_units(units: &NonEmpty<WithSpan<u16>>) -> Result<String, WithSpan<ParseError>> {
    let location = Span::join(units.first().location, units.last().location);
    char::decode_utf16(units.iter().map(|unit| unit.item))
        .collect::<Result<String, _>>()
        .map_err(|_| ParseError::InvalidUnicodeEscape.with_span(location))
}

fn intern_block_string_value<T: From<intern::string_key::StringKey>>(interior: &str) -> T {
    clean_block_string(&interior.replace("\\\"\"\"", "\"\"\""))
        .intern()
        .to()
}

fn clean_block_string(source: &str) -> String {
    let common_indent = get_common_indent(source);

    let mut formatted_lines = block_string_lines(source)
        .enumerate()
        .map(|(i, line)| {
            if i == 0 {
                line.to_string()
            } else {
                line.chars().skip(common_indent).collect::<String>()
            }
        })
        .collect::<VecDeque<String>>();

    while formatted_lines
        .front()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_front();
    }
    while formatted_lines
        .back()
        .is_some_and(|line| line_is_whitespace(line))
    {
        formatted_lines.pop_back();
    }

    let lines_vec: Vec<String> = formatted_lines.into_iter().collect();
    lines_vec.join("\n")
}

fn get_common_indent(source: &str) -> usize {
    let lines = block_string_lines(source).skip(1);
    let mut common_indent: Option<usize> = None;
    for line in lines {
        if let Some((first_index, _)) = line.match_indices(is_not_whitespace).next()
            && common_indent.is_none_or(|indent| first_index < indent)
        {
            common_indent = first_index.wrap_some()
        }
    }
    common_indent.unwrap_or(0)
}

fn block_string_lines(source: &str) -> impl Iterator<Item = &str> {
    source
        .split("\r\n")
        .flat_map(|line| line.split(['\n', '\r']))
}

fn line_is_whitespace(line: &str) -> bool {
    !line.contains(is_not_whitespace)
}

fn is_not_whitespace(c: char) -> bool {
    c != ' ' && c != '\t'
}

#[cfg(test)]
mod tests {
    use common_lang_types::StringLiteralValue;
    use prelude::Postfix;
    use span::{Span, WithSpan};

    use super::{clean_block_string, decode_string_value, intern_block_string_value};
    use crate::span_of::span_of;
    use crate::test_cursor::with_cursor;
    use crate::{GraphQLTokenKind, ParseError};

    fn decoded(text: &str) -> Result<String, WithSpan<ParseError>> {
        with_cursor(text, |cursor| {
            let string = cursor
                .require_token(GraphQLTokenKind::StringLiteral)
                .expect("the fixture is a string literal");
            assert!(cursor.at_end(), "for text {text:?}");
            decode_string_value(string.exclude_ends(1))
        })
    }

    fn invalid_unicode_escape_at(text: &str) -> Span {
        let error = decoded(text).expect_err("the fixture holds an invalid escape");
        assert_eq!(
            error.item,
            ParseError::InvalidUnicodeEscape,
            "for text {text:?}"
        );
        error.location
    }

    #[test]
    fn a_plain_string_is_its_interior() {
        assert_eq!(decoded("\"hi\""), "hi".to_string().wrap_ok());
        assert_eq!(decoded("\"\""), String::new().wrap_ok());
    }

    #[test]
    fn every_escaped_character_decodes() {
        assert_eq!(
            decoded(r#""a\"b\\c\/d\be\ff\ng\rh\ti""#),
            "a\"b\\c/d\u{0008}e\u{000C}f\ng\rh\ti".to_string().wrap_ok()
        );
    }

    #[test]
    fn unicode_escapes_decode() {
        assert_eq!(decoded(r#""A""#), "A".to_string().wrap_ok());
        assert_eq!(decoded(r#""\u{41}""#), "A".to_string().wrap_ok());
        assert_eq!(decoded(r#""\u{1F4A9}""#), "\u{1F4A9}".to_string().wrap_ok());
        assert_eq!(decoded(r#""💩""#), "\u{1F4A9}".to_string().wrap_ok());
    }

    #[test]
    fn a_fixed_width_run_decodes_a_scalar_and_a_pair() {
        assert_eq!(decoded(r#""A💩""#), "A\u{1F4A9}".to_string().wrap_ok());
    }

    #[test]
    fn an_unescaped_supplementary_character_is_itself() {
        assert_eq!(decoded("\"\u{1F4A9}\""), "\u{1F4A9}".to_string().wrap_ok());
    }

    #[test]
    fn escapes_that_are_not_scalar_values_are_errors() {
        for text in [
            r#""\uDEAD""#,
            r#""\uD83D""#,
            r#""\uD83Dx""#,
            r#""\u{D800}""#,
            r#""\u{110000}""#,
            r#""\u{FFFFFFFFF}""#,
        ] {
            invalid_unicode_escape_at(text);
        }
    }

    #[test]
    fn an_unpaired_surrogate_error_spans_its_escape() {
        let text = r#""x\uDEADy""#;
        assert_eq!(invalid_unicode_escape_at(text), span_of(text, r"\uDEAD"));
    }

    #[test]
    fn a_block_string_dedents() {
        assert_eq!(clean_block_string("hi"), "hi");
        assert_eq!(clean_block_string("\n  hello\n  world\n"), "hello\nworld");
        assert_eq!(clean_block_string("   hi"), "   hi");
        assert_eq!(clean_block_string("\r  a\r  b"), "a\nb");
        assert_eq!(clean_block_string("\r\n  a\r\n  b\r\n"), "a\nb");
        assert_eq!(clean_block_string(""), "");
        assert_eq!(clean_block_string("  \n  \n"), "");
    }

    #[test]
    fn an_escaped_triple_quote_is_three_quotes() {
        assert_eq!(
            intern_block_string_value::<StringLiteralValue>(r#"a\"""b"#),
            "a\"\"\"b"
        );
    }
}
