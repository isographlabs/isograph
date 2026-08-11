# text_with_carats

A rewrite of `crates/common_lang_types/src/text_with_carats.rs`. The current implementation walks every line of the file through a `SpanState` state machine, allocates a `String` for every line, colors carats one character at a time, and slices the window it actually wants out at the end. The rewrite locates the span's lines first and renders only the window, in two changes:

1. Tests that pin the observable behavior, written against the current implementation.
2. The rewrite itself.

The public signature changes with the rewrite:

```rust
// from crates/common_lang_types/src/text_with_carats.rs (before)
pub fn text_with_carats(
    file_text: &str,
    outer_span: Option<Span>,
    inner_span: Span,
    color: bool,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>)
```

```rust
// from crates/common_lang_types/src/text_with_carats.rs (after)
pub fn text_with_carats(
    file_text: &str,
    span: Span,
    color: CaratColor,
) -> (String, Option<(OneIndexedRowNumber, OneIndexedColNumber)>)
```

- `span` is file-absolute. The current `outer_span: Option<Span>` exists only to add `outer_span.start` to `inner_span` inside the callee; a caller with a literal-relative span writes `inner_span.with_offset(text_source_span.start)` instead. Nothing in the workspace calls the function today (the module's tests are the only callers), so no call sites migrate.
- `color: bool` becomes the enum `CaratColor`, and the color decision is applied once per highlighted segment instead of once per character.

## The behavior, pinned

These facts hold before and after the rewrite; Change 1 asserts each of them.

- The second element of the return is the 1-indexed row and column of the span's start, `None` when the span is empty or starts past the end of the text.
- An empty span returns `("", None)`.
- A span that starts past the end of the text returns `("", None)`.
- A span that covers only line breaks (or only line breaks and text past the end) returns `("", Some(row_col))`: a position, but nothing to underline.
- The rendered window contains, in order: `line_count_buffer` context lines above, every line from the first through the last line on which the span highlights at least one character (each followed by its carat line when its highlight is non-empty), and `line_count_buffer` context lines below, clamped to the file.
- A line the span touches only through its line break (including an empty line inside a multi-line span) prints without a carat line.
- Carat lines pad with spaces to the source line's length, on both sides of the carat run.
- Colored output differs from plain output only by ANSI escape sequences, and highlights in bright red.

One byte-level output change ships with the rewrite: colored output wraps each highlighted segment in one escape pair (`\u{1b}[91m^^^\u{1b}[0m`) where the current code wraps every carat character separately (`\u{1b}[91m^\u{1b}[0m\u{1b}[91m^\u{1b}[0m...`). Rendered terminal output is identical. The two existing tests that assert the per-character layout (`text_with_carats` and `text_with_carats_multiline`, at the bottom of the module) pin that layout, so Change 1 replaces them with the escape-stripped assertions below.

## Change 1: behavior tests

Test-only edits to the `test` module in `text_with_carats.rs`. The existing helper `text_with_carats_for_test` and the `RUN_SERIALLY` lock stay as they are. The tests `text_with_carats` and `text_with_carats_multiline` are deleted; everything below is added. All of it passes against the current implementation.

```rust
// from crates/common_lang_types/src/text_with_carats.rs
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
fn the_row_and_col_locate_the_spans_start() {
    let (_, row_col) =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(0, 1), 3, false);
    assert_eq!(u32_row_col(row_col), Some((1, 1)));

    let (_, row_col) =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, false);
    assert_eq!(u32_row_col(row_col), Some((4, 2)));
}

#[test]
fn a_span_on_a_line_break_has_a_position_but_no_output() {
    let (output, row_col) =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(9, 10), 3, false);
    assert_eq!(output, "");
    assert_eq!(u32_row_col(row_col), Some((1, 10)));
}

#[test]
fn a_span_past_the_text_has_no_position() {
    let (output, row_col) =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(105, 110), 3, false);
    assert_eq!(output, "");
    assert_eq!(u32_row_col(row_col), None);
}

#[test]
fn an_outer_span_shifts_the_inner_span_by_its_start() {
    let with_outer = text_with_carats_for_test(
        &input_with_lines(10),
        Some(Span::new(20, 80)),
        Span::new(11, 13),
        3,
        false,
    );
    let absolute =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, false);
    assert_eq!(with_outer.0, absolute.0);
    assert_eq!(u32_row_col(with_outer.1), u32_row_col(absolute.1));
}

#[test]
fn an_empty_line_inside_the_span_gets_no_carat_line() {
    let text = "ab\n\ncd";
    let output = text_with_carats_for_test(text, None, Span::new(0, 6), 0, false).0;
    assert_eq!(output, "ab\n^^\n\ncd\n^^");
}

#[test]
fn colored_output_reads_the_same_as_plain_output() {
    for span in [Span::new(31, 33), Span::new(8, 12)] {
        let colored =
            text_with_carats_for_test(&input_with_lines(10), None, span, 3, true).0;
        let plain =
            text_with_carats_for_test(&input_with_lines(10), None, span, 3, false).0;
        assert_eq!(stripped(&colored), plain);
    }
}

#[test]
fn colored_output_highlights_in_bright_red_and_plain_output_has_no_escapes() {
    let colored =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, true).0;
    assert!(colored.contains("\u{1b}[91m"));
    let plain =
        text_with_carats_for_test(&input_with_lines(10), None, Span::new(31, 33), 3, false).0;
    assert!(!plain.contains('\u{1b}'));
}
```

`an_outer_span_shifts_the_inner_span_by_its_start` asserts the fact that lets Change 2 delete the `outer_span` parameter: only `outer_span.start` participates in the output. The test is deleted again in Change 2 along with the parameter it describes.

## Change 2: the rewrite

The production half of the module is replaced wholesale. What gets deleted, by name: the `SpanState` enum, `text_with_carats_and_line_count_buffer_and_line_numbers` (the whole-file loop, the per-line `String` allocation, the per-character carat loop, the `output_lines` slice arithmetic), the `outer_span` defaulting, and the four `unwrap`/`expect` calls on row and column construction. The replacement:

```rust
// from crates/common_lang_types/src/text_with_carats.rs
use std::{num::NonZeroU32, ops::Range};

use colored::Colorize;

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
    match locate(file_text, &span) {
        LocatedSpan::OutsideText => (String::new(), None),
        LocatedSpan::OnLineBreaksOnly(row_col) => (String::new(), Some(row_col)),
        LocatedSpan::Highlighting(highlighted) => {
            let rendered =
                render_window(file_text, &span, &highlighted, line_count_buffer, color);
            (rendered, Some(highlighted.row_col))
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
            row_col = Some((
                OneIndexedRowNumber(one_indexed(index)),
                OneIndexedColNumber(one_indexed(col)),
            ));
        }
        if !highlight_on_line(span, &line_range, line.len()).is_empty() {
            highlighted_lines = match highlighted_lines {
                None => Some((index, index)),
                Some((first_line, _)) => Some((first_line, index)),
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
        let highlight = highlight_on_line(span, &line_range, line.len());
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
            colorize(&"^".repeat(highlight.len()), color),
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
```

Why `locate` may stop at the first line starting past `span.end`: a line prints carats exactly when its range's end exceeds `span.start` and its range's start does not exceed `span.end`, and every later line starts even further past `span.end`.

The window arithmetic matches the current slice semantics because every output line before the first carat line and after the last carat line is a plain source line, so counting `line_count_buffer` output lines on each side (the current behavior) equals counting `line_count_buffer` source lines on each side.

### Test updates in the same change

The helper takes the new signature; every call site drops the `None`/`Some(outer)` argument and replaces `false`/`true` with `CaratColor::Plain`/`CaratColor::Colored`.

```rust
// from crates/common_lang_types/src/text_with_carats.rs
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
```

`an_outer_span_shifts_the_inner_span_by_its_start` is deleted; the parameter it described no longer exists. Every other test from Change 1 and every pre-existing test passes unchanged apart from the argument spelling.

`common_lang_types/src/lib.rs` needs no edit: the module is glob-exported, which picks up `CaratColor`.

## Shipping order

1. Change 1: the test edits. `cargo test -p common_lang_types` passes against the current implementation.
2. Change 2: the rewrite plus its test updates, one commit.

## Landing checklist

- `cargo test -p common_lang_types` passes after each change.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
