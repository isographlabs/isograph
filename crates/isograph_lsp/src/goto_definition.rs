use crate::format::char_index_to_position;
use crate::hover::get_iso_literal_extraction_from_text_position_params;
use crate::{
    lsp_runtime_error::LSPRuntimeResult, semantic_tokens::delta_line_delta_start,
    uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{
    diff_paths_with_prefix, relative_path_from_absolute_and_working_directory,
    strip_windows_long_path_prefix, SelectableName, ServerObjectEntityName, Span,
};
use isograph_compiler::CompilerState;
use isograph_compiler::{
    extract_iso_literals_from_file_content, get_validated_schema, process_iso_literal_extraction,
    read_iso_literals_source_from_relative_path, IsoLiteralExtraction,
};
use isograph_lang_types::{
    DefinitionLocation, Description, IsographResolvedNode, VariableDefinition,
};
use isograph_schema::{
    get_parent_and_selectable_for_object_path, get_parent_and_selectable_for_scalar_path,
    SchemaSource, SelectableTrait, ServerEntityName,
};
use isograph_schema::{IsographDatabase, ServerScalarSelectable};
use isograph_schema::{NetworkProtocol, StandardSources};
use lsp_types::request::GotoDefinition;
use lsp_types::request::Request;
use lsp_types::GotoDefinitionResponse;
use lsp_types::Location;
use lsp_types::LocationLink;
use lsp_types::Range;
use lsp_types::{request::HoverRequest, Hover, HoverContents, MarkupContent, MarkupKind};
use lsp_types::{Position, TextDocumentIdentifier, TextDocumentPositionParams, Uri};
use pico_macros::memo;
use resolve_position::ResolvePosition;
use std::borrow::Cow;
use std::ops::Deref;
use std::path::PathBuf;
use std::str::FromStr;

use std::collections::BTreeMap;

use common_lang_types::{RelativePathToSourceFile, WithLocation};

use pico::{MemoRef, SourceId};

pub fn on_goto_definition<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <GotoDefinition as Request>::Params,
) -> LSPRuntimeResult<<GotoDefinition as Request>::Result> {
    let db = &compiler_state.db;
    Ok(on_goto_definition_impl(
        db,
        params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )
    .to_owned()?
    .map(|Location| GotoDefinitionResponse::Scalar(Location)))
}

#[memo]
pub fn on_goto_definition_impl<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<Option<Location>> {
    let current_working_directory = db.get_current_working_directory();

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &url.to_file_path().expect("Expected file path to be valid."),
    );

    let extraction_option =
        get_iso_literal_extraction_from_text_position_params(db, url.clone(), position.into())
            .to_owned();
    let (extraction, offset) = match extraction_option {
        Some(e) => e,
        None => return Ok(None),
    };

    let memo_ref = get_validated_schema(db);
    let (validated_schema, _stats) = match memo_ref.deref() {
        Ok(schema) => schema,
        Err(_) => return Ok(None),
    };

    let goto_location = if let Ok((result, _text_source)) = process_iso_literal_extraction(
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
                if let Ok((_, selectable)) =
                    get_parent_and_selectable_for_scalar_path(&scalar_path, validated_schema)
                {
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => {
                            server_definition_location(db, &server_selectable.name.location.span())
                                .to_owned()
                        }
                        DefinitionLocation::Client(_) => None,
                    }
                } else {
                    None
                }
            }
            IsographResolvedNode::ObjectSelection(object_path) => {
                if let Ok((_, selectable)) =
                    get_parent_and_selectable_for_object_path(&object_path, validated_schema)
                {
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => {
                            server_definition_location(db, &server_selectable.name.location.span())
                                .to_owned()
                        }
                        DefinitionLocation::Client(_) => None,
                    }
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

    Ok(goto_location)
}

#[memo]
fn server_definition_location<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    span: &Option<Span>,
) -> Option<Location> {
    let binding = strip_windows_long_path_prefix(&db.get_isograph_config().schema.absolute_path);
    let schema_path = binding.to_str()?;

    let normalized_schema_path = if cfg!(windows) {
        Cow::Owned(schema_path.replace("\\", "/"))
    } else {
        Cow::Borrowed(schema_path)
    };

    let uri = Uri::from_str(&format!("file:///{normalized_schema_path}"));

    let StandardSources {
        schema_source_id, ..
    } = db.get_standard_sources();

    let SchemaSource { content, .. } = db.get(*schema_source_id);

    Some(Location {
        uri: uri.ok()?,
        range: span
            .map(|span| Range {
                start: char_index_to_position(content, span.start.try_into().unwrap()),
                end: char_index_to_position(content, span.end.try_into().unwrap()),
            })
            .unwrap_or_default(),
    })
}
