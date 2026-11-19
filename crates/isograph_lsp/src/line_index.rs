use std::ops::ControlFlow;

use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{ClientId, HostLanguage, IsographDb, file_contents};
use pico_macros::memo;
use prelude::Postfix;
use span::Span;

/// Byte offsets of the line breaks and the non-ASCII chars of one text.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LineIndex {
    pub(crate) breaks: Vec<LineBreak>,
    wide_chars: Vec<WideChar>,
    len: u32,
}

/// One line terminator. `start` is the first byte of `\n`, `\r`, or `\r\n`.
/// `after` is the first byte of the next line.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub(crate) struct LineBreak {
    pub(crate) start: u32,
    pub(crate) after: u32,
}

/// A non-ASCII char. Its UTF-8 length exceeds its UTF-16 length.
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
struct WideChar {
    start: u32,
    utf8_len: u8,
    utf16_len: u8,
}

impl LineIndex {
    pub fn new(text: &str) -> Self {
        Self {
            breaks: line_breaks(text),
            wide_chars: text
                .char_indices()
                .filter(|(_, character)| !character.is_ascii())
                .map(|(start, character)| WideChar {
                    start: start as u32,
                    utf8_len: character.len_utf8() as u8,
                    utf16_len: character.len_utf16() as u8,
                })
                .collect(),
            len: text.len() as u32,
        }
    }

    /// 0-based line, and UTF-16 code units from the line start to `offset`.
    pub fn position(&self, offset: u32) -> lsp_types::Position {
        let line = self
            .breaks
            .partition_point(|line_break| line_break.after <= offset);
        let line_start = match line {
            0 => 0,
            n => self.breaks[n - 1].after,
        };
        let first = self
            .wide_chars
            .partition_point(|wide| wide.start < line_start);
        let last = self
            .wide_chars
            .partition_point(|wide| wide.start + u32::from(wide.utf8_len) <= offset);
        let narrowed: u32 = self.wide_chars[first..last]
            .iter()
            .map(|wide| u32::from(wide.utf8_len - wide.utf16_len))
            .sum();
        lsp_types::Position {
            line: line as u32,
            character: offset - line_start - narrowed,
        }
    }

    pub fn range(&self, span: Span) -> lsp_types::Range {
        lsp_types::Range {
            start: self.position(span.start),
            end: self.position(span.end),
        }
    }

    /// The byte offset of `position`. `None` when the line is past the last line, the
    /// column is past the line's end, or the column falls between the two code units
    /// of a surrogate pair.
    pub fn byte_offset(&self, position: lsp_types::Position) -> Option<u32> {
        let line = position.line as usize;
        let line_start = match line {
            0 => 0,
            n => self.breaks.get(n - 1)?.after,
        };
        let line_end = self
            .breaks
            .get(line)
            .map_or(self.len, |line_break| line_break.start);
        let first = self
            .wide_chars
            .partition_point(|wide| wide.start < line_start);
        let last = self
            .wide_chars
            .partition_point(|wide| wide.start < line_end);
        let walked = self.wide_chars[first..last].iter().try_fold(
            (line_start, position.character),
            |(offset, remaining), wide| {
                let narrow = wide.start - offset;
                if remaining <= narrow {
                    return ControlFlow::Break((offset + remaining).wrap_some());
                }
                match (remaining - narrow).checked_sub(u32::from(wide.utf16_len)) {
                    Some(rest) => {
                        ControlFlow::Continue((wide.start + u32::from(wide.utf8_len), rest))
                    }
                    None => ControlFlow::Break(None),
                }
            },
        );
        match walked {
            ControlFlow::Break(offset) => offset,
            ControlFlow::Continue((offset, remaining)) => {
                (remaining <= line_end - offset).then(|| offset + remaining)
            }
        }
    }
}

/// The line index of `path` as `client` reads it.
#[memo]
pub fn line_index<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<LineIndex> {
    let contents = file_contents(db, client, path).as_ref()?;
    LineIndex::new(contents).wrap_some()
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

#[cfg(test)]
mod tests {
    use lsp_types::Position;
    use prelude::Postfix;
    use span::Span;

    use super::{LineBreak, LineIndex, line_breaks};

    fn at(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn byte_offset(text: &str, line: u32, character: u32) -> Option<u32> {
        LineIndex::new(text).byte_offset(at(line, character))
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
    fn byte_offset_of_ascii_is_the_column() {
        assert_eq!(byte_offset("abc", 0, 0), 0.wrap_some());
        assert_eq!(byte_offset("abc", 0, 2), 2.wrap_some());
    }

    #[test]
    fn byte_offset_after_e_acute_counts_one_unit_for_two_bytes() {
        let text = "é.x";
        assert_eq!(byte_offset(text, 0, 1), 2.wrap_some());
        assert_eq!(byte_offset(text, 0, 2), 3.wrap_some());
    }

    #[test]
    fn byte_offset_after_an_emoji_counts_two_units_for_four_bytes() {
        let text = "😀.x";
        assert_eq!(byte_offset(text, 0, 2), 4.wrap_some());
        assert_eq!(byte_offset(text, 0, 3), 5.wrap_some());
    }

    #[test]
    fn byte_offset_inside_a_surrogate_pair_is_none() {
        assert_eq!(byte_offset("😀x", 0, 1), None);
    }

    #[test]
    fn byte_offset_at_the_line_length_is_the_line_end() {
        assert_eq!(byte_offset("ab\ncd", 0, 2), 2.wrap_some());
        assert_eq!(byte_offset("ab\ncd", 1, 2), 5.wrap_some());
        assert_eq!(byte_offset("é", 0, 1), 2.wrap_some());
    }

    #[test]
    fn byte_offset_past_the_line_end_is_none() {
        assert_eq!(byte_offset("ab\ncd", 0, 3), None);
        assert_eq!(byte_offset("ab\ncd", 1, 3), None);
        assert_eq!(byte_offset("é", 0, 2), None);
    }

    #[test]
    fn byte_offset_on_a_line_past_the_end_is_none() {
        assert_eq!(byte_offset("ab\ncd", 2, 0), None);
        assert_eq!(byte_offset("ab", 1, 0), None);
    }

    #[test]
    fn byte_offset_after_a_trailing_line_break_is_the_text_end() {
        assert_eq!(byte_offset("ab\n", 1, 0), 3.wrap_some());
    }

    #[test]
    fn byte_offset_after_crlf_and_cr_starts_the_next_line() {
        assert_eq!(byte_offset("ab\r\ncd", 1, 1), 5.wrap_some());
        assert_eq!(byte_offset("ab\r\ncd", 0, 3), None);
        assert_eq!(byte_offset("ab\rcd", 1, 1), 4.wrap_some());
        assert_eq!(byte_offset("ab\rcd", 0, 3), None);
    }

    #[test]
    fn byte_offset_ignores_wide_chars_on_other_lines() {
        assert_eq!(byte_offset("😀\nab\né", 1, 1), 6.wrap_some());
    }

    #[test]
    fn byte_offset_of_empty_text() {
        assert_eq!(byte_offset("", 0, 0), 0.wrap_some());
        assert_eq!(byte_offset("", 0, 1), None);
        assert_eq!(byte_offset("", 1, 0), None);
    }

    #[test]
    fn position_of_empty_text_is_zero_zero() {
        assert_eq!(LineIndex::new("").position(0), at(0, 0));
    }

    #[test]
    fn position_after_each_line_break_is_the_next_line() {
        assert_eq!(LineIndex::new("a\nb").position(2), at(1, 0));
        assert_eq!(LineIndex::new("a\r\nb").position(3), at(1, 0));
        assert_eq!(LineIndex::new("a\rb").position(2), at(1, 0));
    }

    #[test]
    fn position_between_cr_and_lf_is_on_the_first_line() {
        assert_eq!(LineIndex::new("a\r\nb").position(2), at(0, 2));
    }

    #[test]
    fn position_counts_utf16_units_of_wide_chars_before_it() {
        assert_eq!(LineIndex::new("é.").position(2), at(0, 1));
        assert_eq!(LineIndex::new("😀.").position(4), at(0, 2));
    }

    #[test]
    fn position_ignores_wide_chars_on_earlier_lines() {
        assert_eq!(LineIndex::new("😀\nab").position(6), at(1, 1));
    }

    #[test]
    fn range_across_a_line_break() {
        assert_eq!(
            LineIndex::new("ab\ncd").range(Span::new(1, 4)),
            lsp_types::Range {
                start: at(0, 1),
                end: at(1, 1),
            }
        );
    }

    #[test]
    fn byte_offset_inverts_position_at_every_char_boundary_outside_crlf() {
        let text = "a😀b\r\nécd\re";
        let between_cr_and_lf = 7;
        let index = LineIndex::new(text);
        text.char_indices()
            .map(|(offset, _)| offset as u32)
            .chain(std::iter::once(text.len() as u32))
            .filter(|offset| *offset != between_cr_and_lf)
            .for_each(|offset| {
                assert_eq!(
                    index.byte_offset(index.position(offset)),
                    offset.wrap_some(),
                    "offset {offset}"
                );
            });
    }
}
