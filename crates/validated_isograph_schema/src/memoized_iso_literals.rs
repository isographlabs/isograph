use std::collections::HashMap;
use std::ops::Deref;

use common_lang_types::{ClientSelectableName, ServerObjectEntityName};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{ClientFieldDeclaration, ClientPointerDeclaration, SelectionType};
use isograph_schema::{IsographDatabase, NetworkProtocol};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::parse_iso_literal_in_source;

#[legacy_memo]
pub fn client_selectable_declaration_map<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientSelectableName),
    Vec<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>>,
> {
    let mut out: HashMap<(_, ClientSelectableName), Vec<_>> = HashMap::new();

    for (_relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for extraction in parse_iso_literal_in_source(db, *iso_literals_source_id).to_owned() {
            match extraction {
                Ok((extraction_result, _)) => match extraction_result {
                    IsoLiteralExtractionResult::ClientPointerDeclaration(
                        client_pointer_declaration,
                    ) => {
                        out.entry((
                            client_pointer_declaration.item.parent_type.item.0,
                            client_pointer_declaration
                                .item
                                .client_pointer_name
                                .item
                                .0
                                .into(),
                        ))
                        .or_default()
                        .push(SelectionType::Object(client_pointer_declaration.item));
                    }
                    IsoLiteralExtractionResult::ClientFieldDeclaration(
                        client_field_declaration,
                    ) => {
                        out.entry((
                            client_field_declaration.item.parent_type.item.0,
                            client_field_declaration
                                .item
                                .client_field_name
                                .item
                                .0
                                .into(),
                        ))
                        .or_default()
                        .push(SelectionType::Scalar(client_field_declaration.item));
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => {
                        // Intentionally ignored. TODO reconsider
                    }
                },

                Err(_) => {
                    // For now, we can only ignore these errors! We don't know a parent entity name
                    // and a selectable name. But. we should restructure this so that we get both,
                    // even if the rest fails to parse.
                }
            }
        }
    }

    out
}

#[legacy_memo]
pub fn client_selectable_declarations<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Vec<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>> {
    let memo_ref = client_selectable_declaration_map(db);

    memo_ref
        .deref()
        .get(&(parent_object_entity_name, client_selectable_name))
        .cloned()
        .unwrap_or_default()
}

#[legacy_memo]
pub fn client_selectable_declaration<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Result<
    Option<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>>,
    MemoizedIsoLiteralError,
> {
    let memo_ref =
        client_selectable_declarations(db, parent_object_entity_name, client_selectable_name);

    match memo_ref.deref().split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                Ok(Some(first.clone()))
            } else {
                Err(MemoizedIsoLiteralError::MultipleDefinitionsFound {
                    duplicate_entity_name: parent_object_entity_name,
                    duplicate_client_selectable_name: client_selectable_name,
                })
            }
        }
        None => {
            // Empty, this shouldn't happen. We can consider having a NonEmptyVec or something
            Ok(None)
        }
    }
}

#[derive(Clone, Error, Debug, Eq, PartialEq)]
pub enum MemoizedIsoLiteralError {
    #[error(
        "Multiple definitions of `{duplicate_entity_name}.{duplicate_client_selectable_name}` were found"
    )]
    MultipleDefinitionsFound {
        duplicate_entity_name: ServerObjectEntityName,
        duplicate_client_selectable_name: ClientSelectableName,
    },
}
