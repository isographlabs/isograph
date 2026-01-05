use std::num::NonZeroU32;

use colored::Colorize;

use crate::Span;

enum SpanState {
    /// We have not yet reached the start of the span
    Before,
    /// We have reached the start of the span, but not the end
    Inside,
    /// We have passed the end of the span
    After,
}

static LINE_COUNT_BUFFER: usize = 2;

pub fn text_with_carats(
    file_text: &str,
    outer_span: Option<Span>,
    inner_span: Span,
    color: bool,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
    text_with_carats_and_line_count_buffer_and_line_numbers(
        file_text,
        outer_span,
        inner_span,
        LINE_COUNT_BUFFER,
        color,
    )
}

/// The row number, 1-indexed. Because VSCode!
pub struct OneIndexedRowNumber(pub NonZeroU32);
/// The col number, 1-indexed. Because VSCode!
pub struct OneIndexedColNumber(pub NonZeroU32);

/// For a given string and span, return a string with
/// the span underlined with carats and LINE_COUNT_BUFFER previous and following
/// lines.
fn text_with_carats_and_line_count_buffer_and_line_numbers(
    file_text: &str,
    outer_span: Option<Span>,
    inner_span: Span,
    line_count_buffer: usize,
    colorize_carats: bool,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
    // Major hack alert
    if inner_span.is_empty() {
        return ("".to_string(), None);
    }

    // Another major hack
    let outer_span_start = outer_span.map(|x| x.start).unwrap_or(0);
    let actual_span = Span::new(
        outer_span_start + inner_span.start,
        outer_span_start + inner_span.end,
    );

    let mut output_lines = vec![];
    let mut cur_index = 0;

    // index of the line (in output_lines) of **source text** in which the span starts
    let mut first_line_with_span = usize::MAX;
    // index of the line (in output_lines) of **carat text** in which the span ends
    let mut last_line_with_span = 0;

    let mut line_row = None;

    let mut span_state = SpanState::Before;
    for (line_index, line_content) in file_text.split('\n').enumerate() {
        let start_of_line = cur_index;

        // +1 is accounting for \n, though presumably we should handle other line endings
        cur_index += line_content.len() + 1;

        let end_of_line = cur_index;

        let should_print_carats = match span_state {
            SpanState::Before => {
                if end_of_line > actual_span.end as usize {
                    line_row = Some((
                        OneIndexedRowNumber((line_index as u32 + 1).try_into().unwrap()),
                        OneIndexedColNumber(
                            (actual_span.start - (start_of_line as u32) + 1)
                                .try_into()
                                .expect("Expected col index to be positive"),
                        ),
                    ));
                    span_state = SpanState::After;
                    true
                } else if end_of_line > actual_span.start as usize {
                    line_row = Some((
                        OneIndexedRowNumber((line_index as u32 + 1).try_into().unwrap()),
                        OneIndexedColNumber(
                            (actual_span.start - (start_of_line as u32) + 1)
                                .try_into()
                                .expect("Expected col index to be positive"),
                        ),
                    ));
                    span_state = SpanState::Inside;
                    true
                } else {
                    false
                }
            }
            SpanState::Inside => {
                if end_of_line > actual_span.end as usize {
                    span_state = SpanState::After;
                }
                true
            }
            SpanState::After => false,
        };

        if should_print_carats {
            let line_len = line_content.len();
            let start_of_carats = (actual_span.start as usize).saturating_sub(start_of_line);

            let end_of_carats = std::cmp::min(
                (actual_span.end as usize).saturating_sub(start_of_line),
                line_len,
            );

            let prefix = &line_content[0..start_of_carats];
            let highlighted = &line_content[start_of_carats..end_of_carats];
            let suffix = &line_content[end_of_carats..];
            let colored_source = format!(
                "{}{}{}",
                prefix,
                if colorize_carats {
                    highlighted.bright_red()
                } else {
                    highlighted.normal()
                },
                suffix
            );
            output_lines.push(colored_source);
            // a line may be entirely empty, due to containing only a \n. We probably want to avoid
            // printing an empty line underneath. This is weird and probably buggy!

            if start_of_carats != line_len && end_of_carats != 0 {
                first_line_with_span = std::cmp::min(first_line_with_span, output_lines.len());
                last_line_with_span = output_lines.len() + 1;

                let mut carats = String::new();
                for _ in 0..start_of_carats {
                    carats.push(' ');
                }
                for _ in start_of_carats..end_of_carats {
                    carats.push_str(&format!(
                        "{}",
                        if colorize_carats {
                            "^".bright_red()
                        } else {
                            "^".normal()
                        }
                    ));
                }
                for _ in end_of_carats..line_len {
                    carats.push(' ');
                }

                output_lines.push(carats);
            }
        } else {
            output_lines.push(line_content.to_string());
        }
    }

    // This is indicative of a bug. If we are passed a span that encompasses
    // only a line break, we never set first_line_with_span, so the range would
    // have a start > end, causing a panic. See the test bug_span_on_line_break
    //
    // This case also happens if the span.start > text.len()
    if first_line_with_span == usize::MAX {
        return ("".to_string(), line_row);
    }

    // Which output lines do we care about? We would like:
    // - the source line containing the start of the span and LINE_COUNT_BUFFER earlier lines
    // - the carat line containing the end of the span and LINE_COUNT_BUFFER later lines
    // - everything in between

    (
        output_lines[(first_line_with_span.saturating_sub(line_count_buffer + 1))
            ..(std::cmp::min(last_line_with_span + line_count_buffer, output_lines.len()))]
            .join("\n"),
        line_row,
    )
}

#[cfg(test)]
mod test {
    // Note: we use raw strings in this module, and the extra
    // spaces on lines with carats matter!

    use crate::{Span, text_with_carats::text_with_carats_and_line_count_buffer_and_line_numbers};

    fn text_with_carats_for_test(
        file_text: &str,
        outer_span: Option<Span>,
        inner_span: Span,
        line_count_buffer: usize,
        colorize_carats: bool,
    ) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>) {
        colored::control::set_override(true);
        let text_with_carats = text_with_carats_and_line_count_buffer_and_line_numbers(
            file_text,
            outer_span,
            inner_span,
            line_count_buffer,
            colorize_carats,
        );
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

    #[test]
    fn empty_span() {
        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 0), 3, false).0;
        assert_eq!(output, "");
    }

    #[test]
    fn empty_span_but_not_zero() {
        // This is weird behavior, and maybe we should print no output here.
        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(1, 1), 3, false).0;
        assert_eq!(output, "");
    }

    #[test]
    fn bug_span_on_line_break() {
        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(9, 10), 3, false).0;
        assert_eq!(output, "");
    }

    #[test]
    fn one_leading_char_first_line_span() {
        let output = with_leading_line_break(
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 1), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 3), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 9), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 10), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 43), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 53), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(30, 42), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(29, 42), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(90, 100), 3, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(90, 105), 3, false).0,
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

        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(105, 110), 3, false).0;
        assert_eq!(output, "");
    }

    #[test]
    fn line_count_buffer_0() {
        let output = with_leading_line_break(
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 0, false).0,
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
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 1, false).0,
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
    fn text_with_carats() {
        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, true).0;

        let expected = "012345678\n012345678\n012345678\n0\u{1b}[91m12\u{1b}[0m345678\n \u{1b}[91m^\u{1b}[0m\u{1b}[91m^\u{1b}[0m      \n012345678\n012345678\n012345678";
        assert_eq!(output, expected);
    }
    #[test]
    fn text_with_carats_multiline() {
        let output =
            text_with_carats_for_test(&input_with_lines(10), None, Span::new(8, 12), 3, true).0;

        let expected = "01234567\u{1b}[91m8\u{1b}[0m\n        \u{1b}[91m^\u{1b}[0m\n\u{1b}[91m01\u{1b}[0m2345678\n\u{1b}[91m^\u{1b}[0m\u{1b}[91m^\u{1b}[0m       \n012345678\n012345678\n012345678";

        assert_eq!(output, expected);
    }
}
