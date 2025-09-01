use std::collections::VecDeque;

use intern::string_key::Intern;

use common_lang_types::{DescriptionValue, WithSpan};
use isograph_lang_types::{Description, semantic_token_legend};

use crate::{IsographLangTokenKind, PeekableLexer};

pub(crate) fn parse_optional_description(
    tokens: &mut PeekableLexer,
) -> Option<WithSpan<Description>> {
    parse_single_line_description(tokens).or_else(|| parse_multiline_description(tokens))
}

fn parse_multiline_description(tokens: &mut PeekableLexer) -> Option<WithSpan<Description>> {
    tokens
        .parse_source_of_kind(
            IsographLangTokenKind::BlockStringLiteral,
            semantic_token_legend::ST_COMMENT,
        )
        .map(|parsed_str| {
            parsed_str
                .map(|unparsed_text| clean_block_string_literal(unparsed_text).intern().into())
        })
        .ok()
        .map(|with_span| {
            with_span.map(|description_value: DescriptionValue| description_value.into())
        })
}

fn parse_single_line_description(tokens: &mut PeekableLexer) -> Option<WithSpan<Description>> {
    tokens
        .parse_source_of_kind(
            IsographLangTokenKind::StringLiteral,
            semantic_token_legend::ST_COMMENT,
        )
        .map(|parsed_str| {
            parsed_str.map(|source_with_quotes| {
                source_with_quotes[1..source_with_quotes.len() - 1]
                    .intern()
                    .into()
            })
        })
        .ok()
        .map(|with_span| {
            with_span.map(|description_value: DescriptionValue| description_value.into())
        })
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
    let lines = source.lines().skip(1);
    let mut common_indent: Option<usize> = None;
    for line in lines {
        if let Some((first_index, _)) = line.match_indices(is_not_whitespace).next()
            && common_indent.is_none_or(|indent| first_index < indent)
        {
            common_indent = Some(first_index)
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
