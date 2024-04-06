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

/// For a given string and span, return a string with
/// the span underlined with carats and LINE_COUNT_BUFFER previous and following
/// lines.
pub(crate) fn text_with_carats(text: &str, span: Span) -> String {
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

    // Which output lines do we care about? We would like:
    // - the source line containing the start of the span and LINE_COUNT_BUFFER earlier lines
    // - the carat line containing the end of the span and LINE_COUNT_BUFFER later lines
    // - everything in between

    output_lines[(first_line_with_span.saturating_sub(LINE_COUNT_BUFFER + 1))
        ..(std::cmp::min(last_line_with_span + LINE_COUNT_BUFFER, output_lines.len()))]
        .join("\n")
}
