use std::collections::VecDeque;

use common_lang_types::{DescriptionValue, WithSpan};
use graphql_syntax::TokenKind;
use intern::string_key::Intern;

use super::peekable_lexer::PeekableLexer;

pub(crate) fn parse_optional_description(
    tokens: &mut PeekableLexer,
) -> Option<WithSpan<DescriptionValue>> {
    parse_single_line_description(tokens).or_else(|| parse_multiline_description(tokens))
}

fn parse_multiline_description(tokens: &mut PeekableLexer) -> Option<WithSpan<DescriptionValue>> {
    tokens
        .parse_source_of_kind(TokenKind::BlockStringLiteral)
        .map(|parsed_str| {
            parsed_str
                .map(|unparsed_text| clean_block_string_literal(unparsed_text).intern().into())
        })
        .ok()
}

fn parse_single_line_description(tokens: &mut PeekableLexer) -> Option<WithSpan<DescriptionValue>> {
    tokens
        .parse_source_of_kind(TokenKind::StringLiteral)
        .map(|parsed_str| {
            parsed_str.map(|source_with_quotes| {
                source_with_quotes[1..source_with_quotes.len() - 1]
                    .intern()
                    .into()
            })
        })
        .ok()
}
// https://spec.graphql.org/June2018/#sec-String-Value
fn clean_block_string_literal(source: &str) -> String {
    let inner = &source[3..source.len() - 3];
    let common_indent = get_common_indent(inner);

    let mut formatted_lines = inner
        .lines()
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
        .map_or(false, |line| line_is_whitespace(line))
    {
        formatted_lines.pop_front();
    }
    while formatted_lines
        .back()
        .map_or(false, |line| line_is_whitespace(line))
    {
        formatted_lines.pop_back();
    }

    let lines_vec: Vec<String> = formatted_lines.into_iter().collect();
    lines_vec.join("\n")
}

fn get_common_indent(source: &str) -> usize {
    let lines = source.lines().skip(1);
    let mut common_indent: Option<usize> = None;
    for line in lines {
        if let Some((first_index, _)) = line.match_indices(is_not_whitespace).next() {
            if common_indent.map_or(true, |indent| first_index < indent) {
                common_indent = Some(first_index)
            }
        }
    }
    common_indent.unwrap_or(0)
}

fn line_is_whitespace(line: &str) -> bool {
    !line.contains(is_not_whitespace)
}

// https://spec.graphql.org/June2018/#sec-White-Space
fn is_not_whitespace(c: char) -> bool {
    c != ' ' && c != '\t'
}
