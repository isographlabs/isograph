use std::ops::Deref;

use common_lang_types::{
    relative_path_from_absolute_and_working_directory, CurrentWorkingDirectory,
    RelativePathToSourceFile,
};
use isograph_compiler::{
    extract_iso_literals_from_file_content, get_current_working_directory,
    process_iso_literal_extraction, read_iso_literals_source_from_relative_path, CompilerState,
    IsoLiteralExtraction,
};
use isograph_lang_types::{
    semantic_token_legend::IndentChange, InlineBehavior, LineBehavior, SpaceAfter, SpaceBefore,
};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::{
    request::{Formatting, Request},
    Position, Range, TextEdit,
};
use pico_macros::memo;

use crate::lsp_runtime_error::LSPRuntimeResult;

pub fn on_format<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <Formatting as Request>::Params,
) -> LSPRuntimeResult<<Formatting as Request>::Result> {
    let db = &compiler_state.db;
    let url = params.text_document.uri;

    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let memo_ref = read_iso_literals_source_from_relative_path(db, relative_path_to_source_file);
    let content = match memo_ref.deref() {
        Some(s) => &s.content,
        // Is this the correct behavior?
        None => return Ok(None),
    };

    let memo_ref = extract_iso_literals_from_file_content(db, relative_path_to_source_file);
    let extracted_items = memo_ref.deref();

    let text_edits = extracted_items
        .iter()
        .flat_map(|extraction| {
            Some(TextEdit {
                range: get_range_of_extraction(extraction, content),
                new_text: format_extraction(
                    db,
                    extraction,
                    relative_path_to_source_file,
                    current_working_directory,
                )
                .to_owned()?,
            })
        })
        .collect::<Vec<_>>();

    Ok(Some(text_edits))
}

fn get_range_of_extraction(extraction: &IsoLiteralExtraction, content: &str) -> Range {
    let start_index = extraction.iso_literal_start_index;
    let end_index = start_index + extraction.iso_literal_text.len();

    let start_position = char_index_to_position(content, start_index);
    let end_position = char_index_to_position(content, end_index);

    Range {
        start: start_position,
        end: end_position,
    }
}

fn char_index_to_position(content: &str, char_index: usize) -> Position {
    let text_before = &content[..char_index];
    let mut line = 0;
    let mut last_line_start = 0;

    for (index, ch) in text_before.char_indices() {
        if ch == '\n' {
            line += 1;
            last_line_start = index + 1;
        }
    }

    let character = char_index - last_line_start;

    Position {
        line: line as u32,
        character: character as u32,
    }
}

#[memo]
fn format_extraction<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    extraction: &IsoLiteralExtraction,
    relative_path_to_source_file: RelativePathToSourceFile,
    current_working_directory: CurrentWorkingDirectory,
) -> Option<String> {
    let (extraction_result, _) = process_iso_literal_extraction(
        db,
        extraction,
        relative_path_to_source_file,
        current_working_directory,
    )
    .ok()?;

    let semantic_tokens = extraction_result.semantic_tokens();

    let mut output = String::new();
    let mut last_line_behavior = LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(false),
    });
    let mut indent: i8 = 1;

    for token in semantic_tokens {
        let content = &extraction.iso_literal_text[token.span.as_usize_range()];
        let new_line_behavior = token.item.line_behavior;
        let indent_change = token.item.indent_change;

        if let IndentChange::Dedent = indent_change {
            indent -= 1;
        }

        if last_line_behavior.ends_line() || new_line_behavior.start_new_line() {
            push_indented_line_break(&mut output, indent as usize);
            // for spacing, the least amount of spacing wins
        } else if *last_line_behavior.has_space_after() && *new_line_behavior.has_space_before() {
            output.push(' ');
        }

        output.push_str(content);

        last_line_behavior = new_line_behavior;

        if let IndentChange::Indent = indent_change {
            indent += 1;
        }
    }

    if last_line_behavior.ends_line() {
        output.push('\n');
    }

    Some(output)
}

fn push_indented_line_break(output: &mut String, indent: usize) {
    output.push_str(&format!("\n{}", "  ".repeat(indent)));
}
