use std::ops::Deref;

use common_lang_types::{
    SelectableName, ServerObjectEntityName, Span, relative_path_from_absolute_and_working_directory,
};
use isograph_compiler::{
    CompilerState, IsoLiteralExtraction, extract_iso_literals_from_file_content,
    get_validated_schema, process_iso_literal_extraction,
    read_iso_literals_source_from_relative_path,
};
use isograph_lang_types::{Description, IsographResolvedNode, VariableDefinition};
use isograph_schema::{
    IsographDatabase, NetworkProtocol, SelectableTrait, ServerEntityName,
    get_parent_and_selectable_for_object_path, get_parent_and_selectable_for_scalar_path,
};
use lsp_types::{
    Hover, HoverContents, MarkupContent, MarkupKind, Position, Uri,
    request::{HoverRequest, Request},
};
use pico_macros::memo;
use resolve_position::ResolvePosition;

use crate::{
    lsp_runtime_error::LSPRuntimeResult, semantic_tokens::delta_line_delta_start,
    uri_file_path_ext::UriFilePathExt,
};

pub fn on_hover<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <HoverRequest as Request>::Params,
) -> LSPRuntimeResult<<HoverRequest as Request>::Result> {
    let db = &compiler_state.db;
    on_hover_impl(
        db,
        params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )
    .to_owned()
}

#[memo]
fn on_hover_impl<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<<HoverRequest as Request>::Result> {
    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let extraction_option =
        get_iso_literal_extraction_from_text_position_params(db, url, position.into()).to_owned();
    let (extraction, offset) = match extraction_option {
        Some(e) => e,
        None => return Ok(None),
    };

    let memo_ref = get_validated_schema(db);
    let (validated_schema, _stats) = match memo_ref.deref() {
        Ok(schema) => schema,
        Err(_) => return Ok(None),
    };

    let hover_markup = if let Ok((result, _text_source)) = process_iso_literal_extraction(
        db,
        &extraction,
        relative_path_to_source_file,
        current_working_directory,
    ) {
        match result.resolve((), Span::new(offset, offset)) {
            IsographResolvedNode::ClientFieldDeclaration(_) => None,
            IsographResolvedNode::ClientPointerDeclaration(_) => None,
            IsographResolvedNode::EntrypointDeclaration(_) => None,
            IsographResolvedNode::ServerObjectEntityNameWrapper(_) => None,
            IsographResolvedNode::Description(_) => None,
            IsographResolvedNode::ScalarSelection(scalar_path) => {
                if let Ok((parent_object, selectable)) =
                    get_parent_and_selectable_for_scalar_path(&scalar_path, validated_schema)
                {
                    Some(hover_text_for_selectable(
                        selectable.variant_name(),
                        selectable.name(),
                        selectable.description(),
                        selectable.arguments(),
                        parent_object.name.item,
                        parent_object.description,
                    ))
                } else {
                    None
                }
            }
            IsographResolvedNode::ObjectSelection(object_path) => {
                if let Ok((parent_object, selectable)) =
                    get_parent_and_selectable_for_object_path(&object_path, validated_schema)
                {
                    Some(hover_text_for_selectable(
                        selectable.variant_name(),
                        selectable.name(),
                        selectable.description(),
                        selectable.arguments(),
                        parent_object.name.item,
                        parent_object.description,
                    ))
                } else {
                    None
                }
            }
            IsographResolvedNode::ClientScalarSelectableNameWrapper(_) => None,
            IsographResolvedNode::ClientObjectSelectableNameWrapper(_) => None,
        }
    } else {
        None
    };

    Ok(hover_markup.map(|markup| Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
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
    url: Uri,
    line_char: LineChar,
) -> Option<(IsoLiteralExtraction, u32)> {
    let current_working_directory = db.get_current_working_directory();

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
                target_line_char.character - start_char_count
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

fn hover_text_for_selectable(
    server_or_client: &'static str,
    selectable_name: SelectableName,
    selectable_description: Option<Description>,
    selectable_arguments: Vec<&VariableDefinition<ServerEntityName>>,
    parent_type_name: ServerObjectEntityName,
    parent_description: Option<Description>,
) -> String {
    let parent_description = parent_description
        .map(|x| x.to_string())
        .unwrap_or_default();
    let selectable_description = selectable_description
        .map(|x| x.to_string())
        .unwrap_or_default();

    let argument_string = if selectable_arguments.is_empty() {
        "".to_string()
    } else {
        let mut s = "\nArguments:".to_string();
        for arg in selectable_arguments {
            s.push_str(&format!("\n- {}: `{}`", arg.name.item, arg.type_));
            // TODO display default values
        }
        s.push('\n');
        s
    };

    format!(
        "{server_or_client} field **{parent_type_name}.{selectable_name}**\n\
        {argument_string}\
        \n\
        {selectable_description}\n\
        \n\
        **{parent_type_name}**: {parent_description}",
    )
}
