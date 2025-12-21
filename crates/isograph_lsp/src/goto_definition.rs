use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    location_utils::isograph_location_to_lsp_location,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    lsp_state::LspState,
    uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{Span, relative_path_from_absolute_and_working_directory};
use isograph_lang_types::{
    ClientObjectSelectableNameWrapperParent, ClientScalarSelectableNameWrapperParent,
    DefinitionLocation, IsographResolvedNode,
};
use isograph_schema::process_iso_literal_extraction;
use isograph_schema::{
    IsographDatabase, NetworkProtocol, entity_definition_location,
    get_parent_and_selectable_for_object_path, get_parent_and_selectable_for_scalar_path,
    selectable_definition_location,
};
use lsp_types::{
    GotoDefinitionResponse, Position, Uri,
    request::{GotoDefinition, Request},
};
use pico_macros::memo;
use prelude::Postfix;
use resolve_position::ResolvePosition;

pub fn on_goto_definition<TNetworkProtocol: NetworkProtocol>(
    lsp_state: &LspState<TNetworkProtocol>,
    params: <GotoDefinition as Request>::Params,
) -> LSPRuntimeResult<Option<GotoDefinitionResponse>> {
    let db = &lsp_state.compiler_state.db;
    on_goto_definition_impl(
        db,
        params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )
    .to_owned()
}

#[memo]
pub fn on_goto_definition_impl<TNetworkProtocol: NetworkProtocol>(
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
            IsographResolvedNode::EntityNameWrapper(entity) => {
                let location = entity_definition_location(db, entity.inner.0)
                    .to_owned()
                    .ok()
                    .ok_or(LSPRuntimeError::ExpectedError)?
                    .ok_or(LSPRuntimeError::ExpectedError)?;

                GotoDefinitionResponse::Scalar(
                    isograph_location_to_lsp_location(
                        db,
                        location
                            .as_embedded_location()
                            .ok_or(LSPRuntimeError::ExpectedError)?,
                        &db.get_schema_source().content,
                    )
                    .ok_or(LSPRuntimeError::ExpectedError)?,
                )
                .wrap_some()
            }
            IsographResolvedNode::Description(_) => None,
            IsographResolvedNode::ScalarSelection(scalar_path) => {
                if let Ok((parent, selectable)) =
                    get_parent_and_selectable_for_scalar_path(db, &scalar_path)
                {
                    let parent = parent.lookup(db);
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name,
                                server_selectable.lookup(db).name,
                            )
                            .as_ref()
                            .ok()
                            .copied()
                            .flatten()
                            .and_then(|x| x.as_embedded_location())
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
                        DefinitionLocation::Client(client_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name,
                                client_selectable.lookup(db).name,
                            )
                            .as_ref()
                            .ok()
                            .copied()
                            .flatten()
                            .and_then(|x| x.as_embedded_location())
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
                    }
                } else {
                    None
                }
            }
            IsographResolvedNode::ObjectSelection(object_path) => {
                if let Ok((parent, selectable)) =
                    get_parent_and_selectable_for_object_path(db, &object_path)
                {
                    let parent = parent.lookup(db);
                    match selectable {
                        DefinitionLocation::Server(server_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name,
                                server_selectable.lookup(db).name,
                            )
                            .as_ref()
                            .ok()
                            .copied()
                            .flatten()
                            .and_then(|x| x.as_embedded_location())
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
                        DefinitionLocation::Client(client_selectable) => {
                            selectable_definition_location(
                                db,
                                parent.name,
                                client_selectable.lookup(db).name,
                            )
                            .as_ref()
                            .ok()
                            .copied()
                            .flatten()
                            .and_then(|x| x.as_embedded_location())
                            .and_then(|location| {
                                isograph_location_to_lsp_location(
                                    db,
                                    location,
                                    &db.get_schema_source().content,
                                )
                            })
                            .map(lsp_location_to_scalar_response)
                        }
                    }
                } else {
                    None
                }
            }
            IsographResolvedNode::ClientScalarSelectableNameWrapper(wrapper) => {
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

                selectable_definition_location(db, parent_type_name.0, wrapper.inner.0)
                    .as_ref()
                    .ok()
                    .copied()
                    .flatten()
                    .and_then(|x| x.as_embedded_location())
                    .and_then(|location| {
                        isograph_location_to_lsp_location(
                            db,
                            location,
                            &db.get_schema_source().content,
                        )
                    })
                    .map(lsp_location_to_scalar_response)
            }
            IsographResolvedNode::ClientObjectSelectableNameWrapper(object_wrapper_path) => {
                // This is a pretty useless goto def! It just takes the user to the pointer that they're currently hovering on.
                // But, (pre-adding this), the behavior was to say that "No definition found", which is a bad UX.
                let parent_type_name = match object_wrapper_path.parent {
                    ClientObjectSelectableNameWrapperParent::ClientPointerDeclaration(
                        position_resolution_path,
                    ) => position_resolution_path.inner.parent_type.item,
                };

                selectable_definition_location(db, parent_type_name.0, object_wrapper_path.inner.0)
                    .as_ref()
                    .ok()
                    .copied()
                    .flatten()
                    .and_then(|x| x.as_embedded_location())
                    .and_then(|location| {
                        isograph_location_to_lsp_location(
                            db,
                            location,
                            &db.get_schema_source().content,
                        )
                    })
                    .map(lsp_location_to_scalar_response)
            }
            IsographResolvedNode::SelectionSet(_) => None,
            IsographResolvedNode::TypeAnnotation(_) => None,
        }
    } else {
        None
    };

    goto_response.wrap_ok()
}

fn lsp_location_to_scalar_response(location: lsp_types::Location) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Scalar(location)
}
