use std::{num::NonZeroU32, ops::Range};

use colored::Colorize;

use prelude::Postfix;
use span::Span;

const LINE_COUNT_BUFFER: usize = 2;

/// The row number, 1-indexed. Because VSCode!
pub struct OneIndexedRowNumber(pub NonZeroU32);
/// The col number, 1-indexed. Because VSCode!
pub struct OneIndexedColNumber(pub NonZeroU32);

/// Whether the highlighted text and the carats under it are wrapped in
/// terminal color codes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum CaratColor {
    Colored,
    Plain,
}

/// The lines of `file_text` that `span` covers, the spanned text underlined
/// with carats, with [`LINE_COUNT_BUFFER`] context lines above and below; and
/// the row and column of the span's start.
pub fn text_with_carats(
    file_text: &str,
    span: Span,
    color: CaratColor,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
    text_with_carats_and_line_count_buffer(file_text, span, LINE_COUNT_BUFFER, color)
}

fn text_with_carats_and_line_count_buffer(
    file_text: &str,
    span: Span,
    line_count_buffer: usize,
    color: CaratColor,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
    if span.is_empty() {
        return (String::new(), None);
    }
    let span = span.as_usize_range();
    match locate(file_text, span.reference()) {
        LocatedSpan::OutsideText => (String::new(), None),
        LocatedSpan::OnLineBreaksOnly(row_col) => (String::new(), row_col.wrap_some()),
        LocatedSpan::Highlighting(highlighted) => {
            let rendered = render_window(
                file_text,
                span.reference(),
                highlighted.reference(),
                line_count_buffer,
                color,
            );
            (rendered, highlighted.row_col.wrap_some())
        }
    }
}

/// Where a non-empty span sits in a file's line structure.
enum LocatedSpan {
    /// The span starts past the end of the text.
    OutsideText,
    /// The span covers only line breaks: it has a position, but no line has
    /// anything to underline.
    OnLineBreaksOnly((OneIndexedRowNumber, OneIndexedColNumber)),
    Highlighting(HighlightedLines),
}

struct HighlightedLines {
    row_col: (OneIndexedRowNumber, OneIndexedColNumber),
    /// The 0-based index of the first line on which the span highlights at
    /// least one character.
    first_line: usize,
    /// The 0-based index of the last such line.
    last_line: usize,
}

fn locate(file_text: &str, span: &Range<usize>) -> LocatedSpan {
    let mut row_col = None;
    let mut highlighted_lines = None;
    for (index, (line, line_range)) in lines_with_ranges(file_text).enumerate() {
        if line_range.start > span.end {
            break;
        }
        if row_col.is_none() && line_range.end > span.start {
            let col = span.start - line_range.start;
            row_col = (
                OneIndexedRowNumber(one_indexed(index)),
                OneIndexedColNumber(one_indexed(col)),
            )
                .wrap_some();
        }
        if !highlight_on_line(span, line_range.reference(), line.len()).is_empty() {
            highlighted_lines = match highlighted_lines {
                None => (index, index).wrap_some(),
                Some((first_line, _)) => (first_line, index).wrap_some(),
            };
        }
    }
    match (row_col, highlighted_lines) {
        // A line with a non-empty highlight always sets row_col at or before
        // itself, so highlighted lines without a position cannot occur.
        (None, _) => LocatedSpan::OutsideText,
        (Some(row_col), None) => LocatedSpan::OnLineBreaksOnly(row_col),
        (Some(row_col), Some((first_line, last_line))) => {
            LocatedSpan::Highlighting(HighlightedLines {
                row_col,
                first_line,
                last_line,
            })
        }
    }
}

/// Each line of the text with the byte range it occupies. The trailing line
/// break is excluded from the text and included in the range, so the ranges
/// tile the file; the final line's range ends one past the end of the text,
/// where its break would sit.
fn lines_with_ranges(file_text: &str) -> impl Iterator<Item = (&str, Range<usize>)> {
    let mut start = 0;
    file_text.split('\n').map(move |line| {
        let range = start..start + line.len() + 1;
        start = range.end;
        (line, range)
    })
}

/// The columns of a line that the span highlights: empty for a line the span
/// misses, and for one whose only spanned byte is the line break.
fn highlight_on_line(
    span: &Range<usize>,
    line_range: &Range<usize>,
    line_len: usize,
) -> Range<usize> {
    let start = span.start.saturating_sub(line_range.start).min(line_len);
    let end = span.end.saturating_sub(line_range.start).min(line_len);
    start..end
}

fn render_window(
    file_text: &str,
    span: &Range<usize>,
    highlighted: &HighlightedLines,
    line_count_buffer: usize,
    color: CaratColor,
) -> String {
    let first_printed = highlighted.first_line.saturating_sub(line_count_buffer);
    let last_printed = highlighted.last_line + line_count_buffer;

    let mut output_lines = Vec::new();
    for (line, line_range) in lines_with_ranges(file_text)
        .skip(first_printed)
        .take(last_printed - first_printed + 1)
    {
        let highlight = highlight_on_line(span, line_range.reference(), line.len());
        if highlight.is_empty() {
            output_lines.push(line.to_string());
            continue;
        }
        output_lines.push(format!(
            "{}{}{}",
            &line[..highlight.start],
            colorize(&line[highlight.clone()], color),
            &line[highlight.end..],
        ));
        output_lines.push(format!(
            "{}{}{}",
            " ".repeat(highlight.start),
            colorize("^".repeat(highlight.len()).reference(), color),
            " ".repeat(line.len() - highlight.end),
        ));
    }
    output_lines.join("\n")
}

fn colorize(text: &str, color: CaratColor) -> String {
    match color {
        CaratColor::Colored => text.bright_red().to_string(),
        CaratColor::Plain => text.to_string(),
    }
}

/// Converts to 1-indexed, saturating past `u32::MAX`. Callers pass values
/// bounded by a position in a `Span`, which is a `u32`, so the saturation is
/// unreachable.
fn one_indexed(zero_indexed: usize) -> NonZeroU32 {
    NonZeroU32::MIN.saturating_add(u32::try_from(zero_indexed).unwrap_or(u32::MAX))
}

#[cfg(test)]
mod test {
    // Note: we use raw strings in this module, and the extra
    // spaces on lines with carats matter!

    use std::sync::{LazyLock, Mutex};

    use prelude::Postfix;
    use span::Span;

    use crate::{
        CaratColor, OneIndexedColNumber, OneIndexedRowNumber,
        text_with_carats::text_with_carats_and_line_count_buffer,
    };

    static RUN_SERIALLY: LazyLock<Mutex<()>> = LazyLock::new(Mutex::default);

    fn text_with_carats_for_test(
        file_text: &str,
        span: Span,
        line_count_buffer: usize,
        color: CaratColor,
    ) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
        // https://github.com/colored-rs/colored/issues/201
        let _serial_lock = RUN_SERIALLY.lock();
        colored::control::set_override(true);
        let text_with_carats =
            text_with_carats_and_line_count_buffer(file_text, span, line_count_buffer, color);
        colored::control::unset_override();
        text_with_carats
    }

    fn input_with_lines(line_count: usize) -> String {
        // 9 is not present — this is so that every line has 10
        // characters (including the \n) for easy math.
        "012345678\n".repeat(line_count).to_string()
    }

    #[test]
    fn input_with_lines_tests() {
        let input = input_with_lines(10);
        // Just some sanity checks here
        assert_eq!(input.len(), 100);
        assert_eq!(input.as_bytes()[9], "\n".as_bytes()[0]);
        assert_eq!(input.as_bytes()[19], "\n".as_bytes()[0]);
    }

    fn with_leading_line_break(text: String) -> String {
        // This function makes the output of text_with_carats comparable
        // to the raw strings we are using
        format!("\n{text}")
    }

    fn u32_row_col(
        row_col: Option<(OneIndexedRowNumber, OneIndexedColNumber)>,
    ) -> Option<(u32, u32)> {
        row_col.map(|(row, col)| (row.0.get(), col.0.get()))
    }

    /// The text with every ANSI escape sequence removed: everything from an escape
    /// character through the terminating `m`.
    fn stripped(text: &str) -> String {
        let mut result = String::new();
        let mut rest = text;
        while let Some(escape_start) = rest.find('\u{1b}') {
            result.push_str(&rest[..escape_start]);
            let after_escape = &rest[escape_start..];
            match after_escape.find('m') {
                Some(m_index) => rest = &after_escape[m_index + 1..],
                None => return result,
            }
        }
        result.push_str(rest);
        result
    }

    #[test]
    fn empty_span() {
        let output = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(0, 0),
            3,
            CaratColor::Plain,
        )
        .0;
        assert_eq!(output, "");
    }

    #[test]
    fn empty_span_but_not_zero() {
        // This is weird behavior, and maybe we should print no output here.
        let output = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(1, 1),
            3,
            CaratColor::Plain,
        )
        .0;
        assert_eq!(output, "");
    }

    #[test]
    fn bug_span_on_line_break() {
        let output = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(9, 10),
            3,
            CaratColor::Plain,
        )
        .0;
        assert_eq!(output, "");
    }

    #[test]
    fn one_leading_char_first_line_span() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(0, 1),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_leading_char_first_line_span() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(0, 3),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
^^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_leading_char_full_first_line_span() {
        // In this test, the span ends on 9. In the next test, on 10.
        // Char 9 is the line break, and is basically ignored, so these
        // tests compare against the same output (i.e. the same raw string).
        //
        // Note that spans do not include the final character (i.e. it is a range
        // of the form [start, end).)
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(0, 9),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
^^^^^^^^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_leading_char_full_first_line_span_2() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(0, 10),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
^^^^^^^^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_char_mid_line_span() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(31, 33),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
 ^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_char_multi_line_span() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(31, 43),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
 ^^^^^^^^
012345678
^^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_char_multi_line_span_2() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(31, 53),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
 ^^^^^^^^
012345678
^^^^^^^^^
012345678
^^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_line_start_on_beginning_of_line() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(30, 42),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
^^^^^^^^^
012345678
^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn multi_line_start_on_line_break() {
        // char 29 is the line break character...
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(29, 42),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
^^^^^^^^^
012345678
^^
012345678
012345678
012345678"
        );
    }

    #[test]
    fn span_ends_on_final_line() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(90, 100),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
^^^^^^^^^
"
        );
    }

    #[test]
    fn span_longer_than_text() {
        // Maybe this should panic! But it doesn't.

        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(90, 105),
                3,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
012345678
012345678
^^^^^^^^^
"
        );
    }

    #[test]
    fn span_outside_text() {
        // Maybe this should panic! But it doesn't.

        let output = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(105, 110),
            3,
            CaratColor::Plain,
        )
        .0;
        assert_eq!(output, "");
    }

    #[test]
    fn line_count_buffer_0() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(31, 33),
                0,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
 ^^      "
        );
    }

    #[test]
    fn line_count_buffer_1() {
        let output = with_leading_line_break(
            text_with_carats_for_test(
                input_with_lines(10).reference(),
                Span::new(31, 33),
                1,
                CaratColor::Plain,
            )
            .0,
        );
        assert_eq!(
            output,
            r"
012345678
012345678
 ^^
012345678"
        );
    }

    #[test]
    fn the_row_and_col_locate_the_spans_start() {
        let (_, row_col) = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(0, 1),
            3,
            CaratColor::Plain,
        );
        assert_eq!(u32_row_col(row_col), (1, 1).wrap_some());

        let (_, row_col) = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(31, 33),
            3,
            CaratColor::Plain,
        );
        assert_eq!(u32_row_col(row_col), (4, 2).wrap_some());
    }

    #[test]
    fn a_span_on_a_line_break_has_a_position_but_no_output() {
        let (output, row_col) = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(9, 10),
            3,
            CaratColor::Plain,
        );
        assert_eq!(output, "");
        assert_eq!(u32_row_col(row_col), (1, 10).wrap_some());
    }

    #[test]
    fn a_span_past_the_text_has_no_position() {
        let (output, row_col) = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(105, 110),
            3,
            CaratColor::Plain,
        );
        assert_eq!(output, "");
        assert_eq!(u32_row_col(row_col), None);
    }

    #[test]
    fn an_empty_line_inside_the_span_gets_no_carat_line() {
        let text = "ab\n\ncd";
        let output = text_with_carats_for_test(text, Span::new(0, 6), 0, CaratColor::Plain).0;
        assert_eq!(output, "ab\n^^\n\ncd\n^^");
    }

    #[test]
    fn colored_output_reads_the_same_as_plain_output() {
        for span in [Span::new(31, 33), Span::new(8, 12)] {
            let colored = text_with_carats_for_test(
                input_with_lines(10).reference(),
                span,
                3,
                CaratColor::Colored,
            )
            .0;
            let plain = text_with_carats_for_test(
                input_with_lines(10).reference(),
                span,
                3,
                CaratColor::Plain,
            )
            .0;
            assert_eq!(stripped(colored.reference()), plain);
        }
    }

    #[test]
    fn colored_output_highlights_in_bright_red_and_plain_output_has_no_escapes() {
        let colored = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(31, 33),
            3,
            CaratColor::Colored,
        )
        .0;
        assert!(colored.contains("\u{1b}[91m"));
        let plain = text_with_carats_for_test(
            input_with_lines(10).reference(),
            Span::new(31, 33),
            3,
            CaratColor::Plain,
        )
        .0;
        assert!(!plain.contains('\u{1b}'));
    }
}
