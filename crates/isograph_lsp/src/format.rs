use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use isograph_compiler::CompilerState;
use isograph_lang_types::{
    InlineBehavior, LineBehavior, SpaceAfter, SpaceBefore, semantic_token_legend::IndentChange,
};
use isograph_schema::{
    IsoLiteralExtraction, extract_iso_literals_from_file_content, process_iso_literal_extraction,
    read_iso_literals_source_from_relative_path,
};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::{
    Position, Range, TextEdit,
    request::{Formatting, Request},
};
use pico_macros::legacy_memo;

use crate::{lsp_runtime_error::LSPRuntimeResult, uri_file_path_ext::UriFilePathExt};

pub fn on_format<TNetworkProtocol: NetworkProtocol>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <Formatting as Request>::Params,
) -> LSPRuntimeResult<<Formatting as Request>::Result> {
    let db = &compiler_state.db;
    let url = params.text_document.uri;

    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let content = match read_iso_literals_source_from_relative_path(
        db,
        relative_path_to_source_file,
    )
    .lookup()
    {
        Some(s) => &s.content,
        // Is this the correct behavior?
        None => return Ok(None),
    };

    let extracted_items =
        extract_iso_literals_from_file_content(db, relative_path_to_source_file).lookup();

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

pub fn char_index_to_position(content: &str, char_index: usize) -> Position {
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

#[legacy_memo]
fn format_extraction<TNetworkProtocol: NetworkProtocol>(
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
