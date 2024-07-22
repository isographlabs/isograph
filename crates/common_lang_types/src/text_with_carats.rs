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

pub(crate) fn text_with_carats(text: &str, span: Span) -> String {
    text_with_carats_and_line_count_buffer(text, span, LINE_COUNT_BUFFER)
}

/// For a given string and span, return a string with
/// the span underlined with carats and LINE_COUNT_BUFFER previous and following
/// lines.
pub(crate) fn text_with_carats_and_line_count_buffer(
    text: &str,
    span: Span,
    line_count_buffer: usize,
) -> String {
    // Major hack alert
    if span.is_empty() {
        return "".to_string();
    }

    let mut output_lines = vec![];
    let mut cur_index = 0;

    // index of the line (in output_lines) of **source text** in which the span starts
    let mut first_line_with_span = usize::MAX;
    // index of the line (in output_lines) of **carat text** in which the span ends
    let mut last_line_with_span = 0;

    let mut span_state = SpanState::Before;
    for line_content in text.split('\n') {
        output_lines.push(line_content.to_string());

        let start_of_line = cur_index;

        // +1 is accounting for \n, though presumably we should handle other line endings
        cur_index += line_content.len() + 1;

        let end_of_line = cur_index;

        let should_print_carats = match span_state {
            SpanState::Before => {
                if end_of_line > span.end as usize {
                    span_state = SpanState::After;
                    true
                } else if end_of_line > span.start as usize {
                    span_state = SpanState::Inside;
                    true
                } else {
                    false
                }
            }
            SpanState::Inside => {
                if end_of_line > span.end as usize {
                    span_state = SpanState::After;
                }
                true
            }
            SpanState::After => false,
        };

        if should_print_carats {
            let start_of_carats = span.start.saturating_sub(start_of_line as u32);
            let line_len = line_content.len() as u32;
            let end_of_carats =
                std::cmp::min(span.end.saturating_sub(start_of_line as u32), line_len);

            // a line may be entirely empty, due to containing only a \n. We probably want to avoid
            // printing an empty line underneath. This is weird and probably buggy!
            if start_of_carats != line_len && end_of_carats != 0 {
                first_line_with_span = std::cmp::min(first_line_with_span, output_lines.len());
                // add +1 because we want the index of the line containing the carats, not the
                // source text, and because ranges are exclusive on the end
                last_line_with_span = output_lines.len() + 1;

                let mut carats = String::new();
                for _ in 0..start_of_carats {
                    carats.push(' ');
                }
                for _ in start_of_carats..end_of_carats {
                    carats.push('^');
                }
                for _ in end_of_carats..line_len {
                    carats.push(' ');
                }

                output_lines.push(carats);
            }
        }
    }

    // This is indicative of a bug. If we are passed a span that encompasses
    // only a line break, we never set first_line_with_span, so the range would
    // have a start > end, causing a panic. See the test bug_span_on_line_break
    //
    // This case also happens if the span.start > text.len()
    if first_line_with_span == usize::MAX {
        return "".to_string();
    }

    // Which output lines do we care about? We would like:
    // - the source line containing the start of the span and LINE_COUNT_BUFFER earlier lines
    // - the carat line containing the end of the span and LINE_COUNT_BUFFER later lines
    // - everything in between

    output_lines[(first_line_with_span.saturating_sub(line_count_buffer + 1))
        ..(std::cmp::min(last_line_with_span + line_count_buffer, output_lines.len()))]
        .join("\n")
}

#[cfg(test)]
mod test {
    // Note: we use raw strings in this module, and the extra
    // spaces on lines with carats matter!

    use crate::{text_with_carats::text_with_carats_and_line_count_buffer, Span};

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
            text_with_carats_and_line_count_buffer(&input_with_lines(10), Span::new(0, 0), 3);
        assert_eq!(output, "");
    }

    #[test]
    fn empty_span_but_not_zero() {
        // This is weird behavior, and maybe we should print no output here.
        let output =
            text_with_carats_and_line_count_buffer(&input_with_lines(10), Span::new(1, 1), 3);
        assert_eq!(output, "");
    }

    #[test]
    fn bug_span_on_line_break() {
        let output =
            text_with_carats_and_line_count_buffer(&input_with_lines(10), Span::new(9, 10), 3);
        assert_eq!(output, "");
    }

    #[test]
    fn one_leading_char_first_line_span() {
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(0, 1),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(0, 3),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(0, 9),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(0, 10),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(31, 33),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(31, 43),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(31, 53),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(30, 42),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(29, 42),
            3,
        ));
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
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(90, 100),
            3,
        ));
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

        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(90, 105),
            3,
        ));
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
            text_with_carats_and_line_count_buffer(&input_with_lines(10), Span::new(105, 110), 3);
        assert_eq!(output, "");
    }

    #[test]
    fn line_count_buffer_0() {
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(31, 33),
            0,
        ));
        assert_eq!(
            output,
            r"
012345678
 ^^      "
        );
    }

    #[test]
    fn line_count_buffer_1() {
        let output = with_leading_line_break(text_with_carats_and_line_count_buffer(
            &input_with_lines(10),
            Span::new(31, 33),
            1,
        ));
        assert_eq!(
            output,
            r"
012345678
012345678
 ^^      
012345678"
        );
    }
}
