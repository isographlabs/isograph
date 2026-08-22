use isograph_parser::IsographSemanticToken;
use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};
use span::{Span, WithSpan};

/// Parser tokens whose spans are byte offsets into `page_content`.
pub fn lsp_semantic_tokens(
    tokens: &[WithSpan<IsographSemanticToken>],
    page_content: &str,
) -> Vec<lsp_types::SemanticToken> {
    let index = LineIndex::new(page_content);
    let mut cursor = index.cursor();
    let mut previous_token_end = 0u32;
    let mut last_start = LastStart { line: 0, offset: 0 };
    let mut encoded = Vec::with_capacity(tokens.len());
    for token in tokens {
        cursor.check_span(token.location, previous_token_end);
        previous_token_end = token.location.end;
        emit_pieces(*token, &mut cursor, &mut last_start, |t| encoded.push(t));
    }
    encoded
}

fn emit_pieces(
    token: WithSpan<IsographSemanticToken>,
    cursor: &mut LineCursor,
    last_start: &mut LastStart,
    mut emit: impl FnMut(lsp_types::SemanticToken),
) {
    let span = token.location;
    let mut piece_start = span.start;
    while piece_start < span.end {
        cursor.advance_to(piece_start);
        match cursor.current_line_break() {
            Some(line_break) if line_break.start == piece_start => {
                piece_start = line_break.after;
            }
            Some(line_break) if line_break.start < span.end => {
                emit(lsp_semantic_token(
                    token.item,
                    piece_start,
                    line_break.start,
                    cursor,
                    last_start,
                ));
                piece_start = line_break.after;
            }
            _ => {
                emit(lsp_semantic_token(
                    token.item,
                    piece_start,
                    span.end,
                    cursor,
                    last_start,
                ));
                break;
            }
        }
    }
}

/// One LSP token for `piece_start..piece_end` on the current line.
///
/// `delta_line` is this line minus `last_start.line`. If that is 0, `delta_start`
/// is UTF-16 of `last_start.offset..piece_start`. Otherwise `delta_start` is
/// UTF-16 from column 0 of this line. `length` is UTF-16 of the piece.
fn lsp_semantic_token(
    token: IsographSemanticToken,
    piece_start: u32,
    piece_end: u32,
    cursor: &LineCursor,
    last_start: &mut LastStart,
) -> lsp_types::SemanticToken {
    let line = cursor.break_index as u32;
    let length =
        (cursor.index.utf16_len)(&cursor.index.text[(piece_start as usize)..(piece_end as usize)]);
    let (delta_line, delta_start) = match line - last_start.line {
        0 => (
            0,
            (cursor.index.utf16_len)(
                &cursor.index.text[(last_start.offset as usize)..(piece_start as usize)],
            ),
        ),
        delta_line => (delta_line, cursor.column(piece_start)),
    };
    *last_start = LastStart {
        line,
        offset: piece_start,
    };
    lsp_types::SemanticToken {
        delta_line,
        delta_start,
        length,
        token_type: lsp_type_index(token),
        token_modifiers_bitset: 0,
    }
}

/// Previous emitted piece's line and byte start.
#[derive(Copy, Clone)]
struct LastStart {
    line: u32,
    offset: u32,
}

struct LineIndex<'a> {
    text: &'a str,
    breaks: Vec<LineBreak>,
    utf16_len: fn(&str) -> u32,
}

/// `break_index` is the current line. Offsets must not go backward.
struct LineCursor<'a> {
    index: &'a LineIndex<'a>,
    break_index: usize,
}

/// One line terminator. `start` is the first byte of `\n`, `\r`, or `\r\n`.
/// `after` is the first byte of the next line.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct LineBreak {
    start: u32,
    after: u32,
}

impl<'a> LineIndex<'a> {
    fn new(text: &'a str) -> Self {
        Self {
            text,
            breaks: line_breaks(text),
            utf16_len: if text.is_ascii() {
                |text| text.len() as u32
            } else {
                |text| text.encode_utf16().count() as u32
            },
        }
    }

    fn cursor(&self) -> LineCursor<'_> {
        LineCursor {
            index: self,
            break_index: 0,
        }
    }
}

impl LineCursor<'_> {
    fn check_span(&mut self, span: Span, previous_token_end: u32) {
        assert!(
            span.end >= span.start,
            "semantic token span {}..{} is inverted",
            span.start,
            span.end,
        );
        assert!(
            span.start >= previous_token_end,
            "semantic token spans must be mutually exclusive and ordered; got {}..{} after end {}",
            span.start,
            span.end,
            previous_token_end,
        );
        self.check_offset(span.start);
        let start_index = self.break_index;
        self.check_offset(span.end);
        self.break_index = start_index;
        assert!(
            span.start < span.end,
            "semantic token span {}..{} contains no text to highlight",
            span.start,
            span.end,
        );
        self.assert_has_text(span);
    }

    fn check_offset(&mut self, offset: u32) {
        let len = self.index.text.len() as u32;
        assert!(
            offset <= len,
            "byte {} is out of range for a document of length {}",
            offset,
            len,
        );
        assert!(
            self.index.text.is_char_boundary(offset as usize),
            "byte {} is not a char boundary",
            offset,
        );
        self.advance_to(offset);
        if let Some(line_break) = self.current_line_break() {
            assert!(
                !(line_break.start < offset && offset < line_break.after),
                "byte {} is strictly inside a line break {}..{}",
                offset,
                line_break.start,
                line_break.after,
            );
        }
    }

    fn assert_has_text(&mut self, span: Span) {
        let start_index = self.break_index;
        let mut offset = span.start;
        while offset < span.end {
            self.advance_to(offset);
            match self.current_line_break() {
                Some(line_break) if line_break.start == offset => {
                    offset = line_break.after;
                }
                _ => {
                    self.break_index = start_index;
                    return;
                }
            }
        }
        panic!(
            "semantic token span {}..{} contains no text to highlight",
            span.start, span.end,
        );
    }

    fn advance_to(&mut self, offset: u32) {
        while self.break_index < self.index.breaks.len()
            && self.index.breaks[self.break_index].after <= offset
        {
            self.break_index += 1;
        }
    }

    fn column(&self, offset: u32) -> u32 {
        let line_start = match self.break_index {
            0 => 0,
            n => self.index.breaks[n - 1].after,
        };
        (self.index.utf16_len)(&self.index.text[(line_start as usize)..(offset as usize)])
    }

    fn current_line_break(&self) -> Option<LineBreak> {
        self.index.breaks.get(self.break_index).copied()
    }
}

/// `\n`, `\r`, and `\r\n` (one break). Same set as `IsographLangTokenKind::LineBreak`.
fn line_breaks(text: &str) -> Vec<LineBreak> {
    let bytes = text.as_bytes();
    let mut index = 0usize;
    let mut breaks = Vec::new();
    while index < bytes.len() {
        match bytes[index] {
            b'\n' => {
                let start = index as u32;
                index += 1;
                breaks.push(LineBreak {
                    start,
                    after: index as u32,
                });
            }
            b'\r' => {
                let start = index as u32;
                index += 1;
                if matches!(bytes.get(index), Some(&b'\n')) {
                    index += 1;
                }
                breaks.push(LineBreak {
                    start,
                    after: index as u32,
                });
            }
            _ => index += 1,
        }
    }
    breaks
}

/// Index in this slice is `token_type`. Unused slots keep the origin numbering.
const LEGEND_TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::NAMESPACE,
    SemanticTokenType::TYPE,
    SemanticTokenType::CLASS,
    SemanticTokenType::ENUM,
    SemanticTokenType::INTERFACE,
    SemanticTokenType::STRUCT,
    SemanticTokenType::TYPE_PARAMETER,
    SemanticTokenType::PARAMETER,
    SemanticTokenType::VARIABLE,
    SemanticTokenType::PROPERTY,
    SemanticTokenType::ENUM_MEMBER,
    SemanticTokenType::EVENT,
    SemanticTokenType::FUNCTION,
    SemanticTokenType::METHOD,
    SemanticTokenType::MACRO,
    SemanticTokenType::KEYWORD,
    SemanticTokenType::MODIFIER,
    SemanticTokenType::COMMENT,
    SemanticTokenType::STRING,
    SemanticTokenType::NUMBER,
    SemanticTokenType::REGEXP,
    SemanticTokenType::OPERATOR,
    SemanticTokenType::DECORATOR,
];

const LEGEND_TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DECLARATION,
    SemanticTokenModifier::DEFINITION,
    SemanticTokenModifier::READONLY,
    SemanticTokenModifier::STATIC,
    SemanticTokenModifier::DEPRECATED,
    SemanticTokenModifier::ABSTRACT,
    SemanticTokenModifier::ASYNC,
];

pub fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: LEGEND_TOKEN_TYPES.to_vec(),
        token_modifiers: LEGEND_TOKEN_MODIFIERS.to_vec(),
    }
}

const TYPE: u32 = 1;
const CLASS: u32 = 2;
const PARAMETER: u32 = 7;
const VARIABLE: u32 = 8;
const PROPERTY: u32 = 9;
const KEYWORD: u32 = 15;
const COMMENT: u32 = 17;
const STRING: u32 = 18;
const NUMBER: u32 = 19;
const OPERATOR: u32 = 21;
const DECORATOR: u32 = 22;

fn lsp_type_index(token: IsographSemanticToken) -> u32 {
    match token {
        IsographSemanticToken::Keyword => KEYWORD,
        IsographSemanticToken::Type => CLASS,
        IsographSemanticToken::FieldName | IsographSemanticToken::ObjectKey => PROPERTY,
        IsographSemanticToken::GraphQLTypeName => TYPE,
        IsographSemanticToken::DirectiveName => DECORATOR,
        IsographSemanticToken::Variable | IsographSemanticToken::BooleanOrNull => VARIABLE,
        IsographSemanticToken::Argument => PARAMETER,
        IsographSemanticToken::Integer => NUMBER,
        IsographSemanticToken::String => STRING,
        IsographSemanticToken::Period
        | IsographSemanticToken::Colon
        | IsographSemanticToken::Equals
        | IsographSemanticToken::Parenthesis
        | IsographSemanticToken::Brace
        | IsographSemanticToken::Content
        | IsographSemanticToken::Bracket => OPERATOR,
        IsographSemanticToken::Error => COMMENT,
    }
}

#[cfg(test)]
mod tests {
    use isograph_parser::{IsographSemanticToken, parse_iso_literal};
    use lsp_types::{SemanticToken, SemanticTokenType};
    use prelude::Postfix;
    use span::{Span, WithSpan, WithSpanPostfix};

    use super::{
        CLASS, COMMENT, DECORATOR, KEYWORD, LineBreak, NUMBER, OPERATOR, PARAMETER, PROPERTY,
        STRING, TYPE, VARIABLE, line_breaks, lsp_semantic_tokens, lsp_type_index,
        semantic_token_legend,
    };

    fn encoded(source: &str) -> Vec<SemanticToken> {
        let parsed = parse_iso_literal(source);
        encode(&parsed.tokens, source)
    }

    fn rebased(
        tokens: &[WithSpan<IsographSemanticToken>],
        offset: u32,
    ) -> Vec<WithSpan<IsographSemanticToken>> {
        tokens
            .iter()
            .map(|token| token.item.with_span(token.location.with_offset(offset)))
            .collect()
    }

    fn encode_rebased(
        tokens: &[WithSpan<IsographSemanticToken>],
        offset: u32,
        page_content: &str,
    ) -> Vec<SemanticToken> {
        encode(&rebased(tokens, offset), page_content)
    }

    fn of_type(lsp: &[SemanticToken], token_type: u32) -> Vec<SemanticToken> {
        lsp.iter()
            .filter(|token| token.token_type == token_type)
            .copied()
            .collect()
    }

    fn encode(
        tokens: &[WithSpan<IsographSemanticToken>],
        page_content: &str,
    ) -> Vec<SemanticToken> {
        lsp_semantic_tokens(tokens, page_content)
    }

    #[test]
    fn line_breaks_records_crlf_as_one_break() {
        assert_eq!(
            line_breaks("a\r\nb\nc\rd"),
            vec![
                LineBreak { start: 1, after: 3 },
                LineBreak { start: 4, after: 5 },
                LineBreak { start: 6, after: 7 },
            ],
        );
    }

    #[test]
    fn entrypoint_encodes_as_keyword_class_operator_property() {
        let lsp = encoded("entrypoint Query.foo");
        assert_eq!(lsp.len(), 4);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 10);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].token_modifiers_bitset, 0);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 11);
        assert_eq!(lsp[1].length, 5);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(lsp[2].delta_line, 0);
        assert_eq!(lsp[2].delta_start, 5);
        assert_eq!(lsp[2].length, 1);
        assert_eq!(lsp[2].token_type, OPERATOR);
        assert_eq!(lsp[3].delta_line, 0);
        assert_eq!(lsp[3].delta_start, 1);
        assert_eq!(lsp[3].length, 3);
        assert_eq!(lsp[3].token_type, PROPERTY);
    }

    #[test]
    fn extraction_offset_shifts_the_first_delta_start() {
        let prefix = "const x = iso(`";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, prefix.len() as u32);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[1].token_type, CLASS);
        assert_eq!(&source[prefix.len()..prefix.len() + 5], "field");
    }

    #[test]
    fn extraction_offset_after_a_newline_sets_delta_line() {
        let prefix = "const x = iso(`\n  ";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(lsp[0].delta_line, 1);
        assert_eq!(lsp[0].delta_start, 2);
        assert_eq!(lsp[0].token_type, KEYWORD);
        assert_eq!(lsp[0].length, 5);
    }

    #[test]
    fn extraction_offset_counts_utf16_in_the_prefix() {
        let prefix = "const é = iso(`";
        let literal = "field Pet.fullName { id }";
        let source = format!("{prefix}{literal}`)");
        let parsed = parse_iso_literal(literal);
        let lsp = encode_rebased(&parsed.tokens, prefix.len() as u32, source.reference());
        assert_eq!(prefix.len(), 16);
        assert_eq!(lsp[0].delta_line, 0);
        assert_eq!(lsp[0].delta_start, 15);
        assert_eq!(lsp[0].token_type, KEYWORD);
    }

    #[test]
    fn two_literals_in_one_page_are_in_order() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source
            .find(first)
            .expect("the first literal is in the file");
        let second_start = source
            .find(second)
            .expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_a.tokens, first_start as u32);
        tokens.extend(rebased(&parsed_b.tokens, second_start as u32));
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 1);
        assert_eq!(lsp[4].delta_start, 5);
        assert_eq!(lsp[4].length, 10);
        assert_eq!(lsp[4].token_type, KEYWORD);
        assert_eq!(lsp[7].delta_line, 0);
        assert_eq!(lsp[7].delta_start, 1);
        assert_eq!(lsp[7].token_type, PROPERTY);
    }

    #[test]
    fn two_literals_on_the_same_line() {
        let source = "iso(`entrypoint Query.A`) iso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source
            .find(first)
            .expect("the first literal is in the file");
        let second_start = source
            .find(second)
            .expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_a.tokens, first_start as u32);
        tokens.extend(rebased(&parsed_b.tokens, second_start as u32));
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 8);
        assert_eq!(lsp[4].delta_line, 0);
        assert_eq!(lsp[4].delta_start, 9);
        assert_eq!(lsp[4].token_type, KEYWORD);
    }

    #[test]
    fn tokens_on_successive_lines_set_delta_line() {
        let lsp = encoded("field Query.Foo {\n  bar\n}");
        assert_eq!(lsp.len(), 7);
        assert_eq!(lsp[4].token_type, OPERATOR);
        assert_eq!(lsp[4].length, 1);
        assert_eq!(lsp[5].delta_line, 1);
        assert_eq!(lsp[5].delta_start, 2);
        assert_eq!(lsp[5].length, 3);
        assert_eq!(lsp[5].token_type, PROPERTY);
        assert_eq!(lsp[6].delta_line, 1);
        assert_eq!(lsp[6].delta_start, 0);
        assert_eq!(lsp[6].length, 1);
        assert_eq!(lsp[6].token_type, OPERATOR);
    }

    #[test]
    fn a_blank_line_between_parser_tokens_increments_delta_line() {
        let lsp = encoded("field Query.Foo {\n\n  bar\n}");
        assert_eq!(lsp.len(), 7);
        assert_eq!(lsp[5].delta_line, 2);
        assert_eq!(lsp[5].delta_start, 2);
        assert_eq!(lsp[5].token_type, PROPERTY);
    }

    #[test]
    fn a_quoted_string_is_one_lsp_token() {
        let source = "field Query.Foo \"the home route\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].delta_line, 0);
        assert_eq!(strings[0].delta_start, 4);
        assert_eq!(strings[0].length, 16);
        assert_eq!(lsp[5].token_type, OPERATOR);
        assert_eq!(lsp[5].delta_line, 0);
        assert_eq!(lsp[5].delta_start, 17);
    }

    #[test]
    fn a_quoted_string_with_an_escaped_newline_is_one_lsp_token() {
        let source = "field Query.Foo \"hi\\n\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 6);
        assert_eq!(strings[0].delta_line, 0);
    }

    #[test]
    fn an_empty_quoted_string_is_one_lsp_token() {
        let strings = of_type(&encoded("field Query.Foo \"\" { bar }"), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 2);
    }

    #[test]
    fn a_one_line_block_string_is_one_lsp_token() {
        let source = "field Query.Foo \"\"\"hi\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 8);
    }

    #[test]
    fn an_empty_block_string_is_one_lsp_token() {
        let strings = of_type(&encoded("field Query.Foo \"\"\"\"\"\" { bar }"), STRING);
        assert_eq!(strings.len(), 1);
        assert_eq!(strings[0].length, 6);
    }

    #[test]
    fn a_multiline_string_becomes_one_lsp_token_per_line_without_the_newline() {
        let source = "field Query.Foo \"\"\"\n  the home\n  route\n\"\"\" { bar }";
        let parsed = parse_iso_literal(source);
        assert_eq!(
            parsed
                .tokens
                .iter()
                .filter(|token| token.item == IsographSemanticToken::String)
                .count(),
            1,
        );
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 4);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].delta_start, 0);
        assert_eq!(strings[2].length, 7);
        assert_eq!(strings[3].delta_line, 1);
        assert_eq!(strings[3].delta_start, 0);
        assert_eq!(strings[3].length, 3);
        assert_eq!(lsp[8].delta_line, 0);
        assert_eq!(lsp[8].delta_start, 4);
        assert_eq!(lsp[8].length, 1);
        assert_eq!(lsp[8].token_type, OPERATOR);
    }

    #[test]
    fn a_block_string_with_content_on_the_opening_line() {
        let source = "field Query.Foo \"\"\"the home\n  route\n\"\"\" { bar }";
        let strings = of_type(&encoded(source), STRING);
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 11);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 7);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].delta_start, 0);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_block_string_with_closing_quotes_on_the_content_line() {
        let source = "field Query.Foo \"\"\"\n  route\"\"\" { bar }";
        let lsp = encoded(source);
        let strings = of_type(&lsp, STRING);
        assert_eq!(strings.len(), 2);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(lsp[6].delta_line, 0);
        assert_eq!(lsp[6].delta_start, 11);
        assert_eq!(lsp[6].token_type, OPERATOR);
    }

    #[test]
    fn a_blank_line_in_a_block_string_does_not_emit_a_token() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n\n  x\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 2);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 3);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn two_blank_lines_in_a_block_string_do_not_emit_tokens() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n\n\n  x\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 3);
        assert_eq!(strings[1].length, 3);
    }

    #[test]
    fn a_crlf_blank_line_in_a_block_string_does_not_emit_a_token() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r\n\r\n  x\r\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 2);
        assert_eq!(strings[1].length, 3);
    }

    #[test]
    fn a_cr_blank_line_in_a_block_string_does_not_emit_a_token() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r\r  x\r\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 2);
        assert_eq!(strings[1].length, 3);
    }

    #[test]
    fn a_block_string_ending_with_a_blank_line() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n  x\n\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].length, 3);
        assert_eq!(strings[2].delta_line, 2);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_crlf_block_string_splits_without_including_the_break() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r\n  the home\r\n  route\r\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 4);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 10);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 7);
        assert_eq!(strings[3].delta_line, 1);
        assert_eq!(strings[3].length, 3);
    }

    #[test]
    fn a_cr_block_string_splits_without_including_the_break() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\r  x\r\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[0].length, 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 3);
        assert_eq!(strings[2].delta_line, 1);
        assert_eq!(strings[2].length, 3);
    }

    #[test]
    fn a_non_ascii_continuation_line_of_a_block_string_is_utf16_length() {
        let strings = of_type(
            &encoded("field Query.Foo \"\"\"\n  café\n\"\"\" { bar }"),
            STRING,
        );
        assert_eq!(strings.len(), 3);
        assert_eq!(strings[1].delta_line, 1);
        assert_eq!(strings[1].delta_start, 0);
        assert_eq!(strings[1].length, 6);
    }

    #[test]
    fn a_multiline_leftover_token_splits_the_same_way() {
        let source = "\"\"\"\n  x";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].token_type, OPERATOR);
        assert_eq!(lsp[0].length, 3);
        assert_eq!(lsp[1].delta_line, 1);
        assert_eq!(lsp[1].delta_start, 0);
        assert_eq!(lsp[1].length, 3);
        assert_eq!(lsp[1].token_type, OPERATOR);
    }

    #[test]
    fn a_multiline_leftover_with_a_blank_line() {
        let source = "\"\"\"\n\n  x";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].length, 3);
        assert_eq!(lsp[1].delta_line, 2);
        assert_eq!(lsp[1].length, 3);
    }

    #[test]
    fn a_span_starting_at_a_line_break_emits_the_text_after_it() {
        let source = "\n  x";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_line, 1);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 3);
    }

    #[test]
    fn no_tokens_encodes_empty() {
        let lsp = encode(&[], "");
        assert_eq!(lsp.len(), 0);
    }

    #[test]
    #[should_panic(expected = "contains no text to highlight")]
    fn a_span_of_only_line_breaks_panics() {
        let source = "\n\n";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "contains no text to highlight")]
    fn a_span_of_one_line_break_panics() {
        let source = "\n";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "contains no text to highlight")]
    fn a_span_of_only_crlf_panics() {
        let source = "\r\n";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "contains no text to highlight")]
    fn a_span_of_only_cr_panics() {
        let source = "\r";
        let tokens = IsographSemanticToken::Content
            .with_span(Span::from_usize(0, source.len()))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    fn utf16_length_of_a_non_ascii_token() {
        let source = "aéb";
        let tokens = IsographSemanticToken::String
            .with_span(Span::from_usize(1, 3))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 1);
    }

    #[test]
    fn utf16_length_of_a_surrogate_pair() {
        let source = "a😀b";
        let tokens = IsographSemanticToken::String
            .with_span(Span::from_usize(1, 5))
            .wrap_vec();
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 1);
        assert_eq!(lsp[0].delta_start, 1);
        assert_eq!(lsp[0].length, 2);
    }

    #[test]
    fn adjacent_spans_encode() {
        let source = "ab";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            IsographSemanticToken::Type.with_span(Span::from_usize(1, 2)),
        ];
        let lsp = encode(&tokens, source);
        assert_eq!(lsp.len(), 2);
        assert_eq!(lsp[0].delta_start, 0);
        assert_eq!(lsp[0].length, 1);
        assert_eq!(lsp[1].delta_line, 0);
        assert_eq!(lsp[1].delta_start, 1);
        assert_eq!(lsp[1].length, 1);
    }

    #[test]
    #[should_panic(expected = "contains no text to highlight")]
    fn an_empty_span_panics() {
        let source = "a";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span::from_usize(0, 0))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn overlapping_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 2)),
            IsographSemanticToken::Type.with_span(Span::from_usize(1, 3)),
        ];
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn out_of_order_spans_panic() {
        let source = "abcd";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(2, 4)),
            IsographSemanticToken::Type.with_span(Span::from_usize(0, 1)),
        ];
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "is inverted")]
    fn inverted_span_panics() {
        let source = "abcd";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span { start: 3, end: 1 })
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "out of range")]
    fn out_of_range_span_panics() {
        let source = "ab";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span::from_usize(0, 5))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "not a char boundary")]
    fn a_span_inside_a_multibyte_char_panics() {
        let source = "aéb";
        let tokens = IsographSemanticToken::Keyword
            .with_span(Span::from_usize(2, 3))
            .wrap_vec();
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "strictly inside a line break")]
    fn a_span_strictly_inside_crlf_panics() {
        let source = "a\r\nb";
        let tokens = vec![
            IsographSemanticToken::Keyword.with_span(Span::from_usize(0, 1)),
            IsographSemanticToken::Type.with_span(Span::from_usize(2, 3)),
        ];
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    #[should_panic(expected = "mutually exclusive and ordered")]
    fn reversed_literals_panic() {
        let source = "iso(`entrypoint Query.A`)\niso(`entrypoint Query.B`)";
        let first = "entrypoint Query.A";
        let second = "entrypoint Query.B";
        let first_start = source
            .find(first)
            .expect("the first literal is in the file");
        let second_start = source
            .find(second)
            .expect("the second literal is in the file");
        let parsed_a = parse_iso_literal(first);
        let parsed_b = parse_iso_literal(second);
        let mut tokens = rebased(&parsed_b.tokens, second_start as u32);
        tokens.extend(rebased(&parsed_a.tokens, first_start as u32));
        lsp_semantic_tokens(&tokens, source);
    }

    #[test]
    fn lsp_type_index_matches_the_legend() {
        let types = semantic_token_legend().token_types;
        assert_eq!(types.len(), 23);
        assert_eq!(types[TYPE as usize], SemanticTokenType::TYPE);
        assert_eq!(types[CLASS as usize], SemanticTokenType::CLASS);
        assert_eq!(types[PARAMETER as usize], SemanticTokenType::PARAMETER);
        assert_eq!(types[VARIABLE as usize], SemanticTokenType::VARIABLE);
        assert_eq!(types[PROPERTY as usize], SemanticTokenType::PROPERTY);
        assert_eq!(types[KEYWORD as usize], SemanticTokenType::KEYWORD);
        assert_eq!(types[COMMENT as usize], SemanticTokenType::COMMENT);
        assert_eq!(types[STRING as usize], SemanticTokenType::STRING);
        assert_eq!(types[NUMBER as usize], SemanticTokenType::NUMBER);
        assert_eq!(types[OPERATOR as usize], SemanticTokenType::OPERATOR);
        assert_eq!(types[DECORATOR as usize], SemanticTokenType::DECORATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Keyword), KEYWORD);
        assert_eq!(lsp_type_index(IsographSemanticToken::Type), CLASS);
        assert_eq!(lsp_type_index(IsographSemanticToken::FieldName), PROPERTY);
        assert_eq!(lsp_type_index(IsographSemanticToken::ObjectKey), PROPERTY);
        assert_eq!(lsp_type_index(IsographSemanticToken::GraphQLTypeName), TYPE);
        assert_eq!(
            lsp_type_index(IsographSemanticToken::DirectiveName),
            DECORATOR
        );
        assert_eq!(lsp_type_index(IsographSemanticToken::Variable), VARIABLE);
        assert_eq!(lsp_type_index(IsographSemanticToken::Argument), PARAMETER);
        assert_eq!(lsp_type_index(IsographSemanticToken::Integer), NUMBER);
        assert_eq!(lsp_type_index(IsographSemanticToken::String), STRING);
        assert_eq!(
            lsp_type_index(IsographSemanticToken::BooleanOrNull),
            VARIABLE
        );
        assert_eq!(lsp_type_index(IsographSemanticToken::Period), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Colon), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Equals), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Parenthesis), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Brace), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Content), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Bracket), OPERATOR);
        assert_eq!(lsp_type_index(IsographSemanticToken::Error), COMMENT);
    }
}
