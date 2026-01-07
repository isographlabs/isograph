use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    location_utils::isograph_location_to_lsp_location,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    lsp_state::LspState,
    uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{EntityName, Span, relative_path_from_absolute_and_working_directory};
use isograph_lang_types::{
    ClientObjectSelectableNameWrapperParent, ClientScalarSelectableNameWrapperParent,
    DefinitionLocation, IsographResolvedNode,
};
use isograph_schema::{CompilationProfile, process_iso_literal_extraction};
use isograph_schema::{
    IsographDatabase, entity_definition_location, get_parent_and_selectable_for_object_path,
    get_parent_and_selectable_for_scalar_path, selectable_definition_location,
};
use lsp_types::{
    GotoDefinitionResponse, Position, Uri,
    request::{GotoDefinition, Request},
};
use pico_macros::memo;
use prelude::Postfix;
use resolve_position::ResolvePosition;

pub fn on_goto_definition<TCompilationProfile: CompilationProfile>(
    lsp_state: &LspState<TCompilationProfile>,
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
pub fn on_goto_definition_impl<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
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

    let goto_response = if let Ok((result, _text_source)) =
        process_iso_literal_extraction(db, &extraction, relative_path_to_source_file)
    {
        match result.resolve((), Span::new(offset, offset)) {
            IsographResolvedNode::ClientFieldDeclaration(_) => None,
            IsographResolvedNode::ClientPointerDeclaration(_) => None,
            IsographResolvedNode::EntrypointDeclaration(_) => None,
            IsographResolvedNode::EntityNameWrapper(entity) => {
                goto_entity_definition(db, entity.inner.0)?
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
                                parent.name.item,
                                server_selectable.lookup(db).name.item,
                            )
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
                                parent.name.item,
                                client_selectable.lookup(db).name,
                            )
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
                                parent.name.item,
                                server_selectable.lookup(db).name.item,
                            )
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
                                parent.name.item,
                                client_selectable.lookup(db).name,
                            )
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
            IsographResolvedNode::TypeAnnotation(type_annotation_path) => {
                let target_entity_name = type_annotation_path.inner.inner();
                goto_entity_definition(db, target_entity_name.0)?
            }
            IsographResolvedNode::VariableNameWrapper(_) => None,
            IsographResolvedNode::VariableDeclarationInner(_) => None,
        }
    } else {
        None
    };

    goto_response.wrap_ok()
}

fn lsp_location_to_scalar_response(location: lsp_types::Location) -> GotoDefinitionResponse {
    GotoDefinitionResponse::Scalar(location)
}

fn goto_entity_definition<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> LSPRuntimeResult<Option<GotoDefinitionResponse>> {
    let location = entity_definition_location(db, entity_name)
        .ok_or(LSPRuntimeError::ExpectedError)?
        .ok_or(LSPRuntimeError::ExpectedError)?;

    GotoDefinitionResponse::Scalar(
        isograph_location_to_lsp_location(db, location, &db.get_schema_source().content)
            .ok_or(LSPRuntimeError::ExpectedError)?,
    )
    .wrap_some()
    .wrap_ok()
}
