use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    location_utils::isograph_location_to_lsp_location,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{Span, relative_path_from_absolute_and_working_directory};
use isograph_compiler::{
    CompilerState, get_validated_schema, process_iso_literal_extraction,
    read_iso_literals_source_from_relative_path,
};
use isograph_lang_types::{
    ClientObjectSelectableNameWrapperParent, ClientScalarSelectableNameWrapperParent,
    DefinitionLocation, IsographResolvedNode,
};
use isograph_schema::{
    IsoLiteralsSource, IsographDatabase, NetworkProtocol,
    get_parent_and_selectable_for_object_path, get_parent_and_selectable_for_scalar_path,
    server_entities_named,
};
use lsp_types::{
    GotoDefinitionResponse, Position, Uri,
    request::{GotoDefinition, Request},
};
use pico_macros::memo;
use resolve_position::ResolvePosition;
use std::ops::Deref;

pub fn on_goto_definition<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <GotoDefinition as Request>::Params,
) -> LSPRuntimeResult<Option<GotoDefinitionResponse>> {
    let db = &compiler_state.db;
    on_goto_definition_impl(
        db,
        params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )
    .to_owned()
}

#[memo]
pub fn on_goto_definition_impl<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<Option<GotoDefinitionResponse>> {
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

    let goto_response = if let Ok((result, _text_source)) = process_iso_literal_extraction(
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
                let memo_ref = server_entities_named(db, entity.inner.0.into());
                let server_entities = match memo_ref.deref() {
                    Ok(s) => s,
                    Err(_) => return Err(LSPRuntimeError::ExpectedError),
                };

                Some(GotoDefinitionResponse::Array(
                    server_entities
                        .iter()
                        .flat_map(|entity| {
                            isograph_location_to_lsp_location(
                                db,
                                entity.location().as_embedded_location()?,
                                &db.get_schema_source().content,
                            )
                        })
                        .collect(),
                ))
            }
            IsographResolvedNode::Description(_) => None,
            IsographResolvedNode::ScalarSelection(scalar_path) => {
                let memo_ref = get_validated_schema(db);
                let (validated_schema, _stats) = match memo_ref.deref() {
                    Ok(schema) => schema,
                    Err(_) => return Ok(None),
                };

                if let Ok((_, selectable)) =
                    get_parent_and_selectable_for_scalar_path(db, &scalar_path, validated_schema)
                {
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => server_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response),
                        DefinitionLocation::Client(client_selectable) => client_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
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
                            })
                            .map(lsp_location_to_scalar_response),
                    }
                } else {
                    None
                }
            }
            IsographResolvedNode::ObjectSelection(object_path) => {
                let memo_ref = get_validated_schema(db);
                let (validated_schema, _stats) = match memo_ref.deref() {
                    Ok(schema) => schema,
                    Err(_) => return Ok(None),
                };

                if let Ok((_, selectable)) =
                    get_parent_and_selectable_for_object_path(db, &object_path, validated_schema)
                {
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => server_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response),
                        DefinitionLocation::Client(client_selectable) => client_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
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
                            })
                            .map(lsp_location_to_scalar_response),
                    }
                } else {
                    None
                }
            }
            IsographResolvedNode::ClientScalarSelectableNameWrapper(wrapper) => {
                let memo_ref = get_validated_schema(db);
                let (validated_schema, _stats) = match memo_ref.deref() {
                    Ok(schema) => schema,
                    Err(_) => return Ok(None),
                };

                let parent_type_name = match wrapper.parent {
                    ClientScalarSelectableNameWrapperParent::EntrypointDeclaration(
                        position_resolution_path,
                    ) => position_resolution_path.inner.parent_type.item,
                    ClientScalarSelectableNameWrapperParent::ClientFieldDeclaration(
                        position_resolution_path,
                    ) => {
                        // This is a pretty useless goto def! It just takes the user to the client field that they're currently hovering on.
                        // But, (pre-adding this), the behavior was to say that "No definition found", which is a bad UX.
                        position_resolution_path.inner.parent_type.item
                    }
                };

                validated_schema
                    .client_scalar_selectable(
                        parent_type_name.0.unchecked_conversion(),
                        wrapper.inner.0,
                    )
                    .and_then(|referenced_selectable| {
                        referenced_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
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
                            })
                    })
                    .map(lsp_location_to_scalar_response)
            }
            IsographResolvedNode::ClientObjectSelectableNameWrapper(object_wrapper_path) => {
                let memo_ref = get_validated_schema(db);
                let (validated_schema, _stats) = match memo_ref.deref() {
                    Ok(schema) => schema,
                    Err(_) => return Ok(None),
                };

                // This is a pretty useless goto def! It just takes the user to the pointer that they're currently hovering on.
                // But, (pre-adding this), the behavior was to say that "No definition found", which is a bad UX.
                let parent_type_name = match object_wrapper_path.parent {
                    ClientObjectSelectableNameWrapperParent::ClientPointerDeclaration(
                        position_resolution_path,
                    ) => position_resolution_path.inner.parent_type.item,
                };
                validated_schema
                    .client_object_selectable(
                        parent_type_name.0.unchecked_conversion(),
                        object_wrapper_path.inner.0,
                    )
                    .and_then(|referenced_selectable| {
                        referenced_selectable
                            .name
                            .location
                            .as_embedded_location()
                            .and_then(|location| {
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
                            })
                    })
                    .map(lsp_location_to_scalar_response)
            }
        }
    } else {
        None
    };

    Ok(goto_response)
}

fn lsp_location_to_scalar_response(location: lsp_types::Location) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Scalar(location)
}
