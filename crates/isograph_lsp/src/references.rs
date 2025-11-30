use common_lang_types::{Span, relative_path_from_absolute_and_working_directory};
use isograph_compiler::CompilerState;
use isograph_lang_types::{ClientScalarSelectableNameWrapperParent, IsographResolvedNode};
use isograph_schema::{IsographDatabase, NetworkProtocol, process_iso_literal_extraction};
use lsp_types::{
    Position, Uri,
    request::{References, Request},
};
use pico_macros::memo;
use prelude::Postfix;
use resolve_position::ResolvePosition;

use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    lsp_runtime_error::LSPRuntimeResult, uri_file_path_ext::UriFilePathExt,
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
                let parent_object_entity_name = match scalar_selectable_name_path.parent {
                    ClientScalarSelectableNameWrapperParent::EntrypointDeclaration(
                        entrypoint_declaration_path,
                    ) => entrypoint_declaration_path.inner.parent_type.item,
                    ClientScalarSelectableNameWrapperParent::ClientFieldDeclaration(
                        client_field_declaration_path,
                    ) => client_field_declaration_path.inner.parent_type.item,
                };
                let scalar_selectable_name = *scalar_selectable_name_path.inner;

                None
            }
            IsographResolvedNode::ClientObjectSelectableNameWrapper(_) => None,
            IsographResolvedNode::SelectionSet(_) => None,
        }
    } else {
        None
    }
    .wrap_ok()
}
