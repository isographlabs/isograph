use crate::lsp_state::LspState;
use crate::{
    hover::get_iso_literal_extraction_from_text_position_params,
    lsp_runtime_error::LSPRuntimeResult, uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::Span;
use common_lang_types::relative_path_from_absolute_and_working_directory;
use isograph_lang_types::DefinitionLocation;
use isograph_lang_types::IsographResolvedNode;
use isograph_lang_types::SelectionType;
use isograph_schema::selectables_for_entity;
use isograph_schema::{CompilationProfile, get_parent_for_selection_set_path};
use isograph_schema::{process_iso_literal_extraction, server_entity_named};
use lsp_types::CompletionItemLabelDetails;
use lsp_types::{
    CompletionItem, CompletionResponse,
    request::{Completion, Request},
};
use prelude::Postfix;
use resolve_position::ResolvePosition;

pub fn on_completion<TCompilationProfile: CompilationProfile>(
    lsp_state: &LspState<TCompilationProfile>,
    params: <Completion as Request>::Params,
) -> LSPRuntimeResult<Option<CompletionResponse>> {
    let url = params.text_document_position.text_document.uri;
    let position = params.text_document_position.position;

    let db = &lsp_state.compiler_state.db;

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

    let completion_response = if let Ok((result, _text_source)) =
        process_iso_literal_extraction(db, &extraction, relative_path_to_source_file)
    {
        match result.resolve((), Span::new(offset, offset)) {
            IsographResolvedNode::SelectionSet(selection_set_path) => {
                if let Ok(parent_object_entity) =
                    get_parent_for_selection_set_path(db, &selection_set_path)
                {
                    if let Ok(map) =
                        selectables_for_entity(db, parent_object_entity.lookup(db).name.item)
                            .as_ref()
                    {
                        map.iter()
                            .flat_map(|result| result.as_ref().ok())
                            .map(|selectable| {
                                let (selectable_name, description) = match selectable {
                                    DefinitionLocation::Server(s) => {
                                        let server_selectable = s.lookup(db);
                                        (
                                            server_selectable.name.to_string(),
                                            server_selectable.description.map(|x| x.to_string()),
                                        )
                                    }
                                    DefinitionLocation::Client(c) => match c {
                                        SelectionType::Scalar(s) => {
                                            let scalar = s.lookup(db);
                                            (
                                                scalar.name.to_string(),
                                                scalar.description.map(|x| x.to_string()),
                                            )
                                        }
                                        SelectionType::Object(o) => {
                                            let object = o.lookup(db);
                                            (
                                                object.name.to_string(),
                                                object.description.map(|x| x.to_string()),
                                            )
                                        }
                                    },
                                };
                                CompletionItem {
                                    label_details: CompletionItemLabelDetails {
                                        detail: None,
                                        description,
                                    }
                                    .wrap_some(),
                                    insert_text: (|| match selectable {
                                        DefinitionLocation::Server(s) => {
                                            let selectable = s.lookup(db);
                                            let entity = server_entity_named(
                                                db,
                                                selectable
                                                    .target_entity
                                                    .item
                                                    .as_ref()
                                                    .ok()?
                                                    .inner()
                                                    .0,
                                            )
                                            .dereference()?
                                            .lookup(db);

                                            match entity.selection_info {
                                                SelectionType::Scalar(_) => None,
                                                SelectionType::Object(_) => {
                                                    (&selectable_name).wrap_some()
                                                }
                                            }
                                        }
                                        DefinitionLocation::Client(c) => {
                                            c.as_ref().as_object().map(|_| &selectable_name)
                                        }
                                    })()
                                    .map(|name| format!("{} {{\n}}", name)),
                                    label: selectable_name,
                                    ..Default::default()
                                }
                            })
                            .collect::<Vec<_>>()
                            .wrap_some()
                    } else {
                        None
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    } else {
        None
    };

    CompletionResponse::Array(completion_response.unwrap_or_default())
        .wrap_some()
        .wrap_ok()
}
