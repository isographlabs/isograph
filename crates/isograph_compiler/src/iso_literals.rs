use std::path::PathBuf;

use isograph_parser::{ParsedIsoLiteral, parse_iso_literal};
use pico_macros::memo;
use prelude::Postfix;

use crate::IsographState;
use crate::host_language::{HostLanguage, IsoLiteralExtraction};

#[derive(Debug, Eq, PartialEq, Copy, Clone, Hash)]
pub struct LineChar {
    pub line: u32,
    pub character: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub path: PathBuf,
    pub index: usize,
}

#[memo]
pub fn parsed_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    iso_literal_text: String,
) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}

#[memo]
pub fn literal_id_at_location<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    path: PathBuf,
    line_char: LineChar,
) -> Option<LiteralId> {
    let extractions = THostLanguage::extract_iso_literals(db, path.clone()).as_ref()?;
    let source_id = db.get_disk_file_map().untracked().0.get(&path).copied()?;
    let file_content = db.get(source_id).contents.reference();
    let index = find_iso_literal_index(line_char, file_content, extractions)?;
    LiteralId { path, index }.wrap_some()
}

#[memo]
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographState<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = THostLanguage::extract_iso_literals(db, literal_id.path.clone()).as_ref()?;
    extractions.get(literal_id.index).cloned()
}

fn find_iso_literal_index<THostLanguage: HostLanguage>(
    target_line_char: LineChar,
    file_content: &str,
    extracted_items: &[IsoLiteralExtraction<THostLanguage>],
) -> Option<usize> {
    let mut last_iteration_end_line_count = 0;
    let mut last_iteration_end_char_count = 0;
    let mut max_prev_span_end = 0;
    for (index, extract_item) in extracted_items.iter().enumerate() {
        let iso_literal_start_index = extract_item.iso_literal_start_index;
        let iso_literal_end_index = iso_literal_start_index + extract_item.iso_literal_text.len();

        let intermediate_content = &file_content[max_prev_span_end..iso_literal_start_index];
        let (intermediate_line, intermediate_char) = line_and_byte(intermediate_content);

        let start_line_count = last_iteration_end_line_count + intermediate_line;
        let start_char_count = if intermediate_line > 0 {
            intermediate_char
        } else {
            last_iteration_end_char_count + intermediate_char
        };

        let iso_content = &file_content[iso_literal_start_index..iso_literal_end_index];
        let (iso_line, iso_char) = line_and_byte(iso_content);

        let end_line_count = start_line_count + iso_line;
        let end_char_count = if iso_line > 0 {
            iso_char
        } else {
            start_char_count + iso_char
        };

        if position_in_range(
            (start_line_count, start_char_count),
            (end_line_count, end_char_count),
            target_line_char,
        ) {
            return index.wrap_some();
        }

        last_iteration_end_line_count = end_line_count;
        last_iteration_end_char_count = end_char_count;
        max_prev_span_end = iso_literal_end_index;
    }

    None
}

fn position_in_range(start: (u32, u32), end: (u32, u32), target: LineChar) -> bool {
    let (start_line_count, start_char_count) = start;
    let (end_line_count, end_char_count) = end;

    if target.line < start_line_count
        || (target.line == start_line_count && target.character < start_char_count)
        || target.line > end_line_count
        || (target.line == end_line_count && target.character >= end_char_count)
    {
        return false;
    }

    true
}

fn line_and_byte(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, byte) in text.as_bytes().iter().enumerate() {
        if *byte == b'\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }
    (line_break_count, text.len() as u32 - last_line_break_index)
}
