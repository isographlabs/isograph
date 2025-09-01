use crate::{
    format::char_index_to_position, hover::get_iso_literal_extraction_from_text_position_params,
    lsp_runtime_error::LSPRuntimeResult, uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{
    EmbeddedLocation, Location, Span, relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Lookup;
use isograph_compiler::{
    CompilerState, get_validated_schema, process_iso_literal_extraction,
    read_iso_literals_source_from_relative_path,
};
use isograph_lang_types::{DefinitionLocation, IsographResolvedNode};
use isograph_schema::{
    IsoLiteralsSource, IsographDatabase, NetworkProtocol,
    get_parent_and_selectable_for_object_path, get_parent_and_selectable_for_scalar_path,
};
use lsp_types::{
    GotoDefinitionResponse, Position, Range, Uri,
    request::{GotoDefinition, Request},
};
use pico_macros::memo;
use resolve_position::ResolvePosition;
use std::{borrow::Cow, ops::Deref, path::PathBuf, str::FromStr};

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
    .map(GotoDefinitionResponse::Scalar))
}

#[memo]
pub fn on_goto_definition_impl<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<Option<lsp_types::Location>> {
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
            IsographResolvedNode::ServerObjectEntityNameWrapper(entity) => {
                let server_object_entity = match validated_schema
                    .server_entity_data
                    .server_object_entity(entity.inner.0.unchecked_conversion())
                {
                    Some(server_object_entity) => server_object_entity,
                    None => {
                        return Ok(None);
                    }
                };

                let schema_source_id = db.get_standard_sources();
                isograph_location_to_lsp_location(
                    db,
                    &server_object_entity.name.location,
                    &db.get(schema_source_id.schema_source_id).content,
                )
            }
            IsographResolvedNode::Description(_) => None,
            IsographResolvedNode::ScalarSelection(scalar_path) => {
                if let Ok((_, selectable)) =
                    get_parent_and_selectable_for_scalar_path(&scalar_path, validated_schema)
                {
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => {
                            match &server_selectable.name.location {
                                Location::Generated => None,
                                Location::Embedded(location) => {
                                    let schema_source_id = db.get_standard_sources();
                                    isograph_location_to_lsp_location(
                                        db,
                                        location,
                                        &db.get(schema_source_id.schema_source_id).content,
                                    )
                                }
                            }
                        }
                        DefinitionLocation::Client(client_selectable) => {
                            match &client_selectable.name.location {
                                Location::Generated => None,
                                Location::Embedded(location) => {
                                    let memo_ref = read_iso_literals_source_from_relative_path(
                                        db,
                                        location.text_source.relative_path_to_source_file,
                                    );

                                    let IsoLiteralsSource {
                                        relative_path: _,
                                        content,
                                    } = memo_ref
                                        .deref()
                                        .as_ref()
                                        .expect("Expected relative path to exist");
                                    isograph_location_to_lsp_location(db, location, content)
                                }
                            }
                        }
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
                            match &server_selectable.name.location {
                                Location::Generated => None,
                                Location::Embedded(location) => {
                                    let schema_source_id = db.get_standard_sources();

                                    isograph_location_to_lsp_location(
                                        db,
                                        location,
                                        &db.get(schema_source_id.schema_source_id).content,
                                    )
                                }
                            }
                        }
                        DefinitionLocation::Client(client_selectable) => {
                            match &client_selectable.name.location {
                                Location::Generated => None,
                                Location::Embedded(location) => {
                                    let memo_ref = read_iso_literals_source_from_relative_path(
                                        db,
                                        location.text_source.relative_path_to_source_file,
                                    );

                                    let IsoLiteralsSource {
                                        relative_path: _,
                                        content,
                                    } = memo_ref
                                        .deref()
                                        .as_ref()
                                        .expect("Expected relative path to exist");

                                    isograph_location_to_lsp_location(db, location, content)
                                }
                            }
                        }
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

fn isograph_location_to_lsp_location<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    location: &EmbeddedLocation,
    content: &str,
) -> Option<lsp_types::Location> {
    let path_buf = PathBuf::from(db.get_current_working_directory().lookup())
        .join(location.text_source.relative_path_to_source_file.lookup());

    let path = path_buf.to_str()?;
    let normalized_path = if cfg!(windows) {
        Cow::Owned(format!(
            "/{}",
            path.strip_prefix(r"\\?\")
                .unwrap_or(path)
                .replace("\\", "/")
        ))
    } else {
        Cow::Borrowed(path)
    };
    let uri = Uri::from_str(&format!("file://{normalized_path}")).ok()?;

    let text_source_start = location
        .text_source
        .span
        .map(|span| span.start)
        .unwrap_or_default();

    Some(lsp_types::Location {
        uri,
        range: Range {
            start: char_index_to_position(
                content,
                (text_source_start + location.span.start)
                    .try_into()
                    .unwrap(),
            ),
            end: char_index_to_position(
                content,
                (text_source_start + location.span.end).try_into().unwrap(),
            ),
        },
    })
}
