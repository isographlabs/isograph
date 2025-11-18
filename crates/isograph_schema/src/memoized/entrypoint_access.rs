use std::collections::HashMap;

use common_lang_types::{ClientScalarSelectableName, ServerObjectEntityName};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{DefinitionLocation, EntrypointDeclaration, SelectionType};
use pico_macros::memo;
use thiserror::Error;

use crate::{
    EntrypointDeclarationInfo, IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol,
    client_scalar_selectable_named, parse_iso_literal_in_source, selectable_named,
};

#[memo]
pub fn entrypoints<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Vec<EntrypointDeclaration> {
    let mut out = vec![];
    for (_relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for result in parse_iso_literal_in_source(db, *iso_literals_source_id) {
            if let Ok((IsoLiteralExtractionResult::EntrypointDeclaration(e), _)) = result {
                out.push(e.item.clone());
            }
            // intentionally ignore non-entrypoints
        }
    }

    out
}

#[memo]
pub fn validated_entrypoints<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> HashMap<
    (ServerObjectEntityName, ClientScalarSelectableName),
    Result<EntrypointDeclarationInfo, ValidatedEntrypointError<TNetworkProtocol>>,
> {
    let entrypoints = entrypoints(db);

    // To validate an entrypoint, we confirm that its parent type exists and the client field is defined,
    // which we can validate by ensuring that the client scalar selectable exists.
    //
    // We also validate that it is a fetchable type.
    entrypoints
        .iter()
        .map(|entrypoint_declaration_info| {
            let value = (|| {
                client_scalar_selectable_named(
                    db,
                    entrypoint_declaration_info.parent_type.item.0,
                    entrypoint_declaration_info.client_field_name.item.0,
                )
                .as_ref()
                .map_err(|e| e.clone())?
                .as_ref()
                .ok_or_else(|| {
                    // check if it has a different type
                    let selectable = selectable_named(
                        db,
                        entrypoint_declaration_info.parent_type.item.0,
                        entrypoint_declaration_info.client_field_name.item.0.into(),
                    );

                    if let Ok(Some(selectable)) = selectable {
                        let actual_type = match selectable {
                            DefinitionLocation::Server(SelectionType::Scalar(_)) => {
                                "a server scalar"
                            }
                            DefinitionLocation::Server(SelectionType::Object(_)) => {
                                "a server object"
                            }
                            DefinitionLocation::Client(SelectionType::Scalar(_)) => {
                                panic!("Unexpected client scalar")
                            }
                            DefinitionLocation::Client(SelectionType::Object(_)) => {
                                "a client pointer"
                            }
                        };

                        return ValidatedEntrypointError::IncorrectType {
                            parent_object_entity_name: entrypoint_declaration_info
                                .parent_type
                                .item
                                .0,
                            selectable_name: entrypoint_declaration_info.client_field_name.item.0,
                            actual_type,
                        };
                    }

                    // if not
                    ValidatedEntrypointError::NotDefined {
                        parent_object_entity_name: entrypoint_declaration_info.parent_type.item.0,
                        client_scalar_selectable_name: entrypoint_declaration_info
                            .client_field_name
                            .item
                            .0,
                    }
                })?;

                Ok(EntrypointDeclarationInfo {
                    iso_literal_text: entrypoint_declaration_info.iso_literal_text,
                    directive_set: entrypoint_declaration_info.entrypoint_directive_set,
                })
            })();

            let key = (
                entrypoint_declaration_info.parent_type.item.0,
                entrypoint_declaration_info.client_field_name.item.0,
            );
            (key, value)
        })
        .collect()
}

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum ValidatedEntrypointError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    MemoizedIsoLiteralError(#[from] MemoizedIsoLiteralError<TNetworkProtocol>),

    #[error("`{parent_object_entity_name}.{client_scalar_selectable_name}` was not defined")]
    NotDefined {
        parent_object_entity_name: ServerObjectEntityName,
        client_scalar_selectable_name: ClientScalarSelectableName,
    },

    #[error(
        "Expected `{parent_object_entity_name}.{selectable_name}` to be a client field, \
        but it was {actual_type}."
    )]
    IncorrectType {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: ClientScalarSelectableName,
        actual_type: &'static str,
    },
}
