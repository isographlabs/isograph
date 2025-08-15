use std::ops::Deref;

use common_lang_types::{relative_path_from_absolute_and_working_directory, Span};
use isograph_compiler::{
    extract_iso_literals_from_file_content, get_current_working_directory,
    process_iso_literal_extraction, read_iso_literals_source_from_relative_path, CompilerState,
    IsoLiteralExtraction,
};
use isograph_lang_types::{
    get_path_to_root_from_object, get_path_to_root_from_scalar, IsographResolvedNode,
};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::{
    request::{HoverRequest, Request},
    Hover, HoverContents, MarkupContent, MarkupKind, Position, Url,
};
use pico_macros::memo;
use resolve_position::ResolvePosition;

use crate::{lsp_runtime_error::LSPRuntimeResult, semantic_tokens::delta_line_delta_start};

pub fn on_hover<TNetworkProtocol: NetworkProtocol + 'static + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <HoverRequest as Request>::Params,
) -> LSPRuntimeResult<<HoverRequest as Request>::Result> {
    let db = &compiler_state.db;
    let url = params.text_document_position_params.text_document.uri;

    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let extraction_option = get_iso_literal_extraction_from_text_position_params(
        db,
        url,
        params.text_document_position_params.position.into(),
    )
    .to_owned();

    Ok(extraction_option
        .and_then(|(extraction, offset)| {
            if let Ok((result, _text_source)) = process_iso_literal_extraction(
                db,
                &extraction,
                relative_path_to_source_file,
                current_working_directory,
            ) {
                return Some(match result.resolve((), Span::new(offset, offset)) {
                    IsographResolvedNode::ClientFieldDeclaration(_) => {
                        "Client field decl".to_string()
                    }
                    IsographResolvedNode::ClientPointerDeclaration(_) => "pointer".to_string(),
                    IsographResolvedNode::EntrypointDeclaration(_) => "entrypoint decl".to_string(),
                    IsographResolvedNode::ServerObjectEntityNameWrapper(_) => {
                        "parent type".to_string()
                    }
                    IsographResolvedNode::Description(_) => "description".to_string(),
                    IsographResolvedNode::ScalarSelection(scalar_path) => {
                        get_path_to_root_from_scalar(&scalar_path).join(" -> ")
                    }
                    IsographResolvedNode::ObjectSelection(object_path) => {
                        get_path_to_root_from_object(&object_path).join(" -> ")
                    }
                    IsographResolvedNode::ClientScalarSelectableNameWrapper(_) => {
                        "name of entrypoint or client field".to_string()
                    }
                    IsographResolvedNode::ClientObjectSelectableNameWrapper(_) => {
                        "name of pointer".to_string()
                    }
                });
            }
            None
        })
        .map(|markup| Hover {
            contents: HoverContents::Markup(MarkupContent {
                kind: MarkupKind::PlainText,
                value: markup,
            }),
            range: None,
        }))
}

#[memo]
pub fn get_iso_literal_extraction_from_text_position_params<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Url,
    line_char: LineChar,
) -> Option<(IsoLiteralExtraction, u32)> {
    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let memo_ref = read_iso_literals_source_from_relative_path(db, relative_path_to_source_file);
    let content = match memo_ref.deref() {
        Some(s) => &s.content,
        // Is this the correct behavior?
        None => return None,
    };

    let memo_ref = extract_iso_literals_from_file_content(db, relative_path_to_source_file);
    let extracted_items = memo_ref.deref();
    find_iso_literal_extraction_under_cursor(line_char, content, extracted_items)
}

fn find_iso_literal_extraction_under_cursor<'a>(
    target_line_char: LineChar,
    content: &str,
    extracted_items: impl IntoIterator<Item = &'a IsoLiteralExtraction> + 'a,
) -> Option<(IsoLiteralExtraction, u32)> {
    let mut last_iteration_end_line_count = 0;
    let mut last_iteration_end_char_count = 0;
    let mut max_prev_span_end = 0;
    for extract_item in extracted_items {
        let iso_literal_start_index = extract_item.iso_literal_start_index;
        let iso_literal_end_index = iso_literal_start_index + extract_item.iso_literal_text.len();

        // This is the file content that we have yet to process
        let intermediate_content = &content[max_prev_span_end..iso_literal_start_index];
        let (intermediate_line, intermediate_char) = delta_line_delta_start(intermediate_content);

        // start_line_count and start_char_count represent the position at the start of the iso literal
        let start_line_count = last_iteration_end_line_count + intermediate_line;
        let start_char_count = if intermediate_line > 0 {
            intermediate_char
        } else {
            last_iteration_end_char_count + intermediate_char
        };

        let iso_content = &content[iso_literal_start_index..iso_literal_end_index];
        let (iso_line, iso_char) = delta_line_delta_start(iso_content);

        // end_line_count and end_char_count represent the position at the end of the iso literal
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
            let diff_line = target_line_char.line - start_line_count;
            let diff_char = if diff_line > 0 {
                target_line_char.character
            } else {
                target_line_char.character - start_line_count
            };

            // This is the character offset in the extract item where we are hovering
            let index_of_line_char = get_index_of_line_char(
                &extract_item.iso_literal_text,
                LineChar {
                    line: diff_line,
                    character: diff_char,
                },
            );
            return Some((extract_item.clone(), index_of_line_char));
        }

        last_iteration_end_line_count = end_line_count;
        last_iteration_end_char_count = end_char_count;
        max_prev_span_end =
            extract_item.iso_literal_start_index + extract_item.iso_literal_text.len();
    }

    None
}

/// A duplicate of an lsp_types type that exists solely to work with `#[memo]`.
/// The original does not implement hash.
#[derive(Debug, Eq, PartialEq, Ord, PartialOrd, Copy, Clone, Default, Hash)]
pub struct LineChar {
    pub line: u32,
    pub character: u32,
}

impl From<Position> for LineChar {
    fn from(Position { line, character }: Position) -> Self {
        LineChar { line, character }
    }
}

/// If the position is in range, returns an offset we can use
fn position_in_range(start: (u32, u32), end: (u32, u32), target: LineChar) -> bool {
    let (start_line_count, start_char_count) = start;
    let (end_line_count, end_char_count) = end;

    if target.line < start_line_count
        || (target.line == start_line_count && target.character < start_char_count)
        || target.line > end_line_count
        || (target.line == end_line_count && target.character > end_char_count)
    {
        return false;
    }

    true
}

fn get_index_of_line_char(source: &str, line_char: LineChar) -> u32 {
    let mut remaining_line_breaks = line_char.line;
    for (index, char) in source.chars().enumerate() {
        if char == '\n' {
            remaining_line_breaks -= 1;
        }

        if remaining_line_breaks == 0 {
            // Why were we off by one to begin with? This is a bad fix!
            return index as u32 + line_char.character + 1;
        }
    }

    // Should we panic?
    source.len() as u32
}
