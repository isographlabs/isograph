use crate::Span;

enum SpanState {
    /// We have not yet reached the start of the span
    Before,
    /// We have reached the start of the span, but not the end
    Inside,
    /// We have passed the end of the span
    After,
}

/// For a given string (e.g. "A\nB\nC\nD") and span (e.g. 2:3), return a string with
/// the span underlined with carats and prev/next lines, e.g. "A\nB\n^\nC".
///
/// Intended implementation:
/// - split the string into lines
/// - find the line containing the start of the span
/// - for each line overlapping the span,
/// - create a new string with spaces or carats for each character in the line,
///   depending on whether the carat is in the span.
/// - interleave the new strings with the original lines
pub(crate) fn text_with_carats(text: &str, span: Span) -> String {
    let mut output_lines = vec![];
    let mut cur_index = 0;

    let mut span_state = SpanState::Before;
    for line_content in text.split("\n").into_iter() {
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

    output_lines.join("\n")
}
