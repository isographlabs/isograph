use common_lang_types::{
    ClientScalarSelectableName, Span, relative_path_from_absolute_and_working_directory,
};
use isograph_compiler::CompilerState;
use isograph_lang_types::{
    ClientScalarSelectableNameWrapperParent, IsographResolvedNode, SelectionType,
};
use isograph_schema::{
    IsographDatabase, NetworkProtocol, accessible_client_selectables, client_selectable_map,
    process_iso_literal_extraction, read_iso_literals_source_from_relative_path,
};
use lsp_types::{
    Position, Uri,
    request::{References, Request},
};
use pico_macros::memo;
use prelude::Postfix;
use resolve_position::ResolvePosition;

use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    location_utils::isograph_location_to_lsp_location,
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    uri_file_path_ext::UriFilePathExt,
};

pub fn on_references<TNetworkProtocol: NetworkProtocol>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <References as Request>::Params,
) -> LSPRuntimeResult<<References as Request>::Result> {
    let db = &compiler_state.db;
    on_references_impl(
        db,
        params.text_document_position.text_document.uri,
        params.text_document_position.position,
    )
    .to_owned()
}

// #[memo]
fn on_references_impl<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<<References as Request>::Result> {
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

    if let Ok((result, _text_source)) = process_iso_literal_extraction(
        db,
        &extraction,
        relative_path_to_source_file,
        current_working_directory,
    ) {
        match result.resolve((), Span::new(offset, offset)) {
            IsographResolvedNode::EntrypointDeclaration(_) => None,
            IsographResolvedNode::ServerObjectEntityNameWrapper(_) => None,
            IsographResolvedNode::Description(_) => None,
            IsographResolvedNode::ClientFieldDeclaration(_) => None,
            IsographResolvedNode::ClientPointerDeclaration(_) => None,
            IsographResolvedNode::ScalarSelection(_) => None,
            IsographResolvedNode::ObjectSelection(_) => None,
            IsographResolvedNode::ClientScalarSelectableNameWrapper(
                scalar_selectable_name_path,
            ) => {
                let target_parent_object_entity_name = match scalar_selectable_name_path.parent {
                    ClientScalarSelectableNameWrapperParent::EntrypointDeclaration(
                        entrypoint_declaration_path,
                    ) => entrypoint_declaration_path.inner.parent_type.item,
                    ClientScalarSelectableNameWrapperParent::ClientFieldDeclaration(
                        client_field_declaration_path,
                    ) => client_field_declaration_path.inner.parent_type.item,
                }
                .dbg();
                let target_scalar_selectable_name = *scalar_selectable_name_path.inner.dbg();

                let map = client_selectable_map(db)
                    .as_ref()
                    .map_err(|_| LSPRuntimeError::ExpectedError)?;

                map.iter()
                    .flat_map(|(key, value)| {
                        let selection_type = match value {
                            Ok(val) => val,
                            Err(_) => return vec![],
                        };

                        eprintln!("about to search selectables in {:?}", key);

                        accessible_client_selectables(db, selection_type)
                            .filter_map(|value| {
                                match value.item {
                                    SelectionType::Scalar(scalar_selection) => {
                                        if scalar_selection.0 == target_parent_object_entity_name.0
                                            && scalar_selection.1 == target_scalar_selectable_name.0
                                        {
                                            return Some(value.location.as_embedded_location()?);
                                        }
                                    }
                                    SelectionType::Object(object_selection) => {
                                        let selection_name: ClientScalarSelectableName =
                                            object_selection.1.unchecked_conversion();
                                        if object_selection.0 == target_parent_object_entity_name.0
                                            && selection_name == target_scalar_selectable_name.0
                                        {
                                            return Some(value.location.as_embedded_location()?);
                                        }
                                    }
                                };
                                None
                            })
                            .flat_map(|location| {
                                let content = match read_iso_literals_source_from_relative_path(
                                    db,
                                    location.text_source.relative_path_to_source_file,
                                ) {
                                    Some(s) => &s.content,
                                    // Is this the correct behavior?
                                    None => return None,
                                };

                                isograph_location_to_lsp_location(db, location, content)
                            })
                            .collect()
                    })
                    .collect::<Vec<_>>()
                    .wrap_some()
            }
            IsographResolvedNode::ClientObjectSelectableNameWrapper(_) => None,
            IsographResolvedNode::SelectionSet(_) => None,
        }
    } else {
        None
    }
    .wrap_ok()
}
