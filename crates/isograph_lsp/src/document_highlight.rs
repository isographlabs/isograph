use common_lang_types::{
    EmbeddedLocation, Span, relative_path_from_absolute_and_working_directory,
};
use isograph_lang_types::{
    ClientObjectSelectableNameWrapperParent, ClientScalarSelectableNameWrapperParent,
    DescriptionParent, EntityNameWrapperParent, IsographResolvedNode,
};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use isograph_schema::{process_iso_literal_extraction, read_iso_literals_source};
use lsp_types::DocumentHighlight;
use lsp_types::request::DocumentHighlightRequest;
use lsp_types::{Position, Uri, request::Request};
use pico_macros::memo;
use prelude::Postfix;
use resolve_position::ResolvePosition;

use crate::hover::get_iso_literal_extraction_from_text_position_params;
use crate::location_utils::isograph_location_to_lsp_location;
use crate::lsp_state::LspState;
use crate::{
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    uri_file_path_ext::UriFilePathExt,
};

pub fn on_document_highlight<TNetworkProtocol: NetworkProtocol>(
    lsp_state: &LspState<TNetworkProtocol>,
    params: <DocumentHighlightRequest as Request>::Params,
) -> LSPRuntimeResult<<DocumentHighlightRequest as Request>::Result> {
    let db = &lsp_state.compiler_state.db;
    on_document_highlight_impl(
        db,
        params.text_document_position_params.text_document.uri,
        params.text_document_position_params.position,
    )
    .to_owned()
}

#[memo]
fn on_document_highlight_impl<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    url: Uri,
    position: Position,
) -> LSPRuntimeResult<<DocumentHighlightRequest as Request>::Result> {
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
        // TODO this exposes the need to have incredibly fine grained resolution...
        match result.resolve((), Span::new(offset, offset)) {
            IsographResolvedNode::ClientFieldDeclaration(_) => None,
            IsographResolvedNode::ClientPointerDeclaration(_) => None,
            IsographResolvedNode::EntrypointDeclaration(_) => None,
            IsographResolvedNode::EntityNameWrapper(entity) => match entity.parent {
                EntityNameWrapperParent::EntrypointDeclaration(entrypoint) => {
                    entrypoint.inner.parent_type.embedded_location
                }
                EntityNameWrapperParent::ClientFieldDeclaration(field) => {
                    field.inner.parent_type.embedded_location
                }
                EntityNameWrapperParent::ClientPointerDeclaration(pointer) => {
                    pointer.inner.parent_type.embedded_location
                }
            }
            .wrap_some(),
            IsographResolvedNode::Description(s) => match s.parent {
                DescriptionParent::ClientFieldDeclaration(field) => {
                    field.inner.description.map(|x| x.embedded_location)
                }
                DescriptionParent::ClientPointerDeclaration(pointer) => {
                    pointer.inner.description.map(|x| x.embedded_location)
                }
            },
            IsographResolvedNode::ScalarSelection(scalar_path) => {
                scalar_path.inner.name.embedded_location.wrap_some()
            }
            IsographResolvedNode::ObjectSelection(object_path) => {
                object_path.inner.name.embedded_location.wrap_some()
            }
            IsographResolvedNode::ClientScalarSelectableNameWrapper(scalar) => {
                match scalar.parent {
                    ClientScalarSelectableNameWrapperParent::EntrypointDeclaration(entrypoint) => {
                        entrypoint.inner.client_field_name.embedded_location
                    }
                    ClientScalarSelectableNameWrapperParent::ClientFieldDeclaration(field) => {
                        field.inner.client_field_name.embedded_location
                    }
                }
                .wrap_some()
            }
            IsographResolvedNode::ClientObjectSelectableNameWrapper(object) => {
                match object.parent {
                    ClientObjectSelectableNameWrapperParent::ClientPointerDeclaration(pointer) => {
                        pointer.inner.client_pointer_name.embedded_location
                    }
                }
                .wrap_some()
            }
            IsographResolvedNode::SelectionSet(_) => None,
            IsographResolvedNode::TypeAnnotation(_) => None,
            IsographResolvedNode::VariableNameWrapper(_) => None,
            IsographResolvedNode::VariableDefinition(_) => None,
        }
    } else {
        None
    }
    .and_then(|location| location_to_document_highlight(db, location))
    .map(|x| vec![x])
    .ok_or(LSPRuntimeError::ExpectedError)?
    .wrap_some()
    .wrap_ok()
}

fn location_to_document_highlight<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    location: EmbeddedLocation,
) -> Option<DocumentHighlight> {
    let iso_literal_map = db.get_iso_literal_map();
    let source_id = iso_literal_map
        .tracked()
        .0
        .get(&location.text_source.relative_path_to_source_file)?;

    let source = read_iso_literals_source(db, *source_id);
    DocumentHighlight {
        range: isograph_location_to_lsp_location(db, location, &source.content)?.range,
        kind: None,
    }
    .wrap_some()
}
