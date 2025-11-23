use std::collections::HashMap;

use crate::{
    ClientObjectSelectable, ClientScalarSelectable, IsographDatabase, NetworkProtocol,
    OwnedClientSelectable, ProcessClientFieldDeclarationError,
    UnprocessedClientScalarSelectableSelectionSet, create_new_exposed_field,
    create_type_system_schema_with_server_selectables, get_link_fields_map,
    process_client_field_declaration_inner, process_client_pointer_declaration_inner,
};
use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, Diagnostic,
    ServerObjectEntityName, WithSpan,
};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{
    ClientFieldDeclaration, ClientPointerDeclaration, SelectionType, SelectionTypePostfix,
};
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::parse_iso_literal_in_source;

/// client selectables defined by iso literals.
/// Note: this is just the declarations, not the fields!
#[memo]
pub fn client_selectable_declaration_map_from_iso_literals<TNetworkProtocol: NetworkProtocol>(
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
                        .push(client_pointer_declaration.item.object_selected());
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
                        .push(client_field_declaration.item.scalar_selected());
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

#[memo]
pub fn client_selectable_declarations<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Vec<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>> {
    client_selectable_declaration_map_from_iso_literals(db)
        .get(&(parent_object_entity_name, client_selectable_name))
        .cloned()
        .unwrap_or_default()
}

#[memo]
pub fn client_selectable_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Result<
    Option<SelectionType<ClientFieldDeclaration, ClientPointerDeclaration>>,
    MemoizedIsoLiteralError,
> {
    match client_selectable_declarations(db, parent_object_entity_name, client_selectable_name)
        .split_first()
    {
        Some((first, rest)) => {
            if rest.is_empty() {
                first.clone().some().ok()
            } else {
                MemoizedIsoLiteralError::MultipleDefinitionsFound {
                    duplicate_entity_name: parent_object_entity_name,
                    duplicate_client_selectable_name: client_selectable_name,
                }
                .err()
            }
        }
        None => {
            // Empty, this shouldn't happen. We can consider having a NonEmptyVec or something
            Ok(None)
        }
    }
}

#[derive(Clone, Error, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum MemoizedIsoLiteralError {
    #[error(
        "Multiple definitions of `{duplicate_entity_name}.{duplicate_client_selectable_name}` were found"
    )]
    MultipleDefinitionsFound {
        duplicate_entity_name: ServerObjectEntityName,
        duplicate_client_selectable_name: ClientSelectableName,
    },

    #[error(
        "Expected `{parent_object_entity_name}.{client_selectable_name}` to be {intended_type}. But it was {actual_type}."
    )]
    SelectableIsWrongType {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
        intended_type: &'static str,
        actual_type: &'static str,
    },

    #[error("{0}")]
    ProcessClientFieldDeclarationError(WithSpan<ProcessClientFieldDeclarationError>),

    #[error("{0}")]
    Diagnostic(Diagnostic),
}

#[memo]
pub fn client_field_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
) -> Result<Option<ClientFieldDeclaration>, MemoizedIsoLiteralError> {
    let x = client_selectable_declaration(
        db,
        parent_object_entity_name,
        client_scalar_selectable_name.into(),
    )
    .as_ref()
    .map_err(|e| e.clone())?;

    let item = match x {
        Some(item) => item,
        None => return Ok(None),
    };
    match item {
        SelectionType::Scalar(client_field_declaration) => {
            client_field_declaration.clone().some().ok()
        }
        SelectionType::Object(_) => MemoizedIsoLiteralError::SelectableIsWrongType {
            parent_object_entity_name,
            client_selectable_name: client_scalar_selectable_name.into(),
            intended_type: "a scalar",
            actual_type: "an object",
        }
        .err(),
    }
}

#[memo]
pub fn client_pointer_declaration<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: ClientObjectSelectableName,
) -> Result<Option<ClientPointerDeclaration>, MemoizedIsoLiteralError> {
    let x = client_selectable_declaration(
        db,
        parent_object_entity_name,
        client_object_selectable_name.into(),
    )
    .as_ref()
    .map_err(|e| e.clone())?;

    let item = match x {
        Some(item) => item,
        None => return Ok(None),
    };
    match item {
        SelectionType::Object(client_pointer_declaration) => {
            client_pointer_declaration.clone().some().ok()
        }
        SelectionType::Scalar(_) => MemoizedIsoLiteralError::SelectableIsWrongType {
            parent_object_entity_name,
            client_selectable_name: client_object_selectable_name.into(),
            intended_type: "an object",
            actual_type: "a scalar",
        }
        .err(),
    }
}

#[memo]
pub fn client_scalar_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
) -> Result<Option<ClientScalarSelectable<TNetworkProtocol>>, MemoizedIsoLiteralError> {
    let declaration =
        client_field_declaration(db, parent_object_entity_name, client_scalar_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    let declaration = match declaration {
        Some(declaration) => declaration.clone(),
        None => {
            // This is an awkward situation! We didn't find any client scalar selectable defined
            // by an iso literal. But, we still need to check for linked fields.
            //
            // What's nice, though, is that we don't actually need the schema to successfully
            // compile if we've already found the field we need! That's neat.
            //
            // We could theoretically skip this if the name is not *LINK_FIELD_NAME /shrug
            //
            // This is also problematic, because we really actually want a "all client fields map" fn,
            // but we don't really have one, since we're adding this here. Oh well. See the awkwardness in
            // selection_set_access.
            let link_fields = get_link_fields_map(db)
                .as_ref()
                .map_err(|e| MemoizedIsoLiteralError::Diagnostic(e.clone()))?;

            if let Some(link_field) = link_fields
                .get(&(parent_object_entity_name, client_scalar_selectable_name))
                .cloned()
            {
                return link_field.some().ok();
            }

            // Awkward! We also need to check for expose fields. Ay ay ay
            return expose_field_map(db)
                .as_ref()
                .map_err(|e| e.clone())?
                .get(&(parent_object_entity_name, client_scalar_selectable_name))
                .cloned()
                .map(|(selectable, _)| selectable)
                .ok();
        }
    };

    let (_, scalar_selectable) = process_client_field_declaration_inner(db, declaration)
        .as_ref()
        .map_err(|e| MemoizedIsoLiteralError::ProcessClientFieldDeclarationError(e.clone()))?;

    scalar_selectable.clone().some().ok()
}

#[memo]
pub fn client_object_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: ClientObjectSelectableName,
) -> Result<Option<ClientObjectSelectable<TNetworkProtocol>>, MemoizedIsoLiteralError> {
    let declaration =
        client_pointer_declaration(db, parent_object_entity_name, client_object_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    let declaration = match declaration {
        Some(declaration) => declaration.clone(),
        None => return Ok(None),
    };

    let (_, object_selectable) = process_client_pointer_declaration_inner(db, declaration)
        .as_ref()
        .map_err(|e| MemoizedIsoLiteralError::ProcessClientFieldDeclarationError(e.clone()))?;

    object_selectable.clone().some().ok()
}

#[memo]
#[expect(clippy::type_complexity)]
pub fn client_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    client_selectable_name: ClientSelectableName,
) -> Result<
    Option<
        SelectionType<
            ClientScalarSelectable<TNetworkProtocol>,
            ClientObjectSelectable<TNetworkProtocol>,
        >,
    >,
    MemoizedIsoLiteralError,
> {
    // we can do this better by reordering functions in this file
    // just in general, we can do better! This is awkward!
    // TODO don't call to_owned, since that clones an error unnecessarily

    let object_selectable = client_object_selectable_named(
        db,
        parent_object_entity_name,
        client_selectable_name.unchecked_conversion(),
    )
    .to_owned();

    let client_selectable = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        client_selectable_name.unchecked_conversion(),
    )
    .to_owned();

    match (object_selectable, client_selectable) {
        (Ok(Some(_)), Ok(Some(_))) => panic!(
            "Unexpected duplicate. \
            This is indicative of a bug in Isograph."
        ),
        (Ok(object), Ok(scalar)) => {
            // If we received two Ok's, that can only be because the field is not defined.
            //
            // Just kidding! That's true if the field is defined in an iso literal, but for __link
            // or an exposed field, object will be None and scalar might be Some.
            //
            // So it's sufficient to ensure that at least one is None.
            assert!(
                object.is_none() || scalar.is_none(),
                "Expected at least one case to be None. \
                This is indicative of a bug in Isograph."
            );
            object
                .map(SelectionType::Object)
                .or(scalar.map(SelectionType::Scalar))
                .ok()
        }
        (Ok(object_selectable), Err(_)) => object_selectable.map(SelectionType::Object).ok(),
        (Err(_), Ok(scalar_selectable)) => scalar_selectable.map(SelectionType::Scalar).ok(),
        (Err(e), Err(_)) => e.err(),
    }
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn expose_field_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        (
            ClientScalarSelectable<TNetworkProtocol>,
            UnprocessedClientScalarSelectableSelectionSet,
        ),
    >,
    MemoizedIsoLiteralError,
> {
    let expose_as_field_queue = create_type_system_schema_with_server_selectables(db)
        .as_ref()
        .map_err(|e| MemoizedIsoLiteralError::Diagnostic(e.clone()))?;

    let mut map = HashMap::new();
    for (parent_object_entity_name, expose_as_fields_to_insert) in expose_as_field_queue {
        for expose_as_field in expose_as_fields_to_insert {
            let (unprocessed_client_scalar_selection_set, exposed_field_client_scalar_selectable) =
                create_new_exposed_field(db, expose_as_field, *parent_object_entity_name)
                    .clone()
                    .map_err(MemoizedIsoLiteralError::Diagnostic)?;

            map.insert(
                (
                    exposed_field_client_scalar_selectable.parent_object_entity_name,
                    exposed_field_client_scalar_selectable.name.item,
                ),
                (
                    exposed_field_client_scalar_selectable,
                    unprocessed_client_scalar_selection_set,
                ),
            );
        }
    }

    Ok(map)
}

// TODO use this as a source for the other functions, especially for
// client_scalar_selectable_named
#[expect(clippy::type_complexity)]
#[memo]
pub fn client_selectable_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientSelectableName),
        Result<OwnedClientSelectable<TNetworkProtocol>, MemoizedIsoLiteralError>,
    >,
    MemoizedIsoLiteralError,
> {
    let iso_literal_map = client_selectable_declaration_map_from_iso_literals(db);

    iso_literal_map
        .iter()
        .map(|(key, value)| {
            let value = (|| match value.split_first() {
                Some((first, rest)) => {
                    if rest.is_empty() {
                        Ok(match first.clone() {
                            SelectionType::Scalar(scalar_declaration) => {
                                process_client_field_declaration_inner(db, scalar_declaration)
                                    .clone()
                                    .map(|(_, selectable)| selectable)
                                    .map_err(|e| {
                                        MemoizedIsoLiteralError::ProcessClientFieldDeclarationError(
                                            e,
                                        )
                                    })?
                                    .scalar_selected()
                            }
                            SelectionType::Object(object_declaration) => {
                                process_client_pointer_declaration_inner(db, object_declaration)
                                    .clone()
                                    .map(|(_, selectable)| selectable)
                                    .map_err(|e| {
                                        MemoizedIsoLiteralError::ProcessClientFieldDeclarationError(
                                            e,
                                        )
                                    })?
                                    .object_selected()
                            }
                        })
                    } else {
                        Err(MemoizedIsoLiteralError::MultipleDefinitionsFound {
                            duplicate_entity_name: key.0,
                            duplicate_client_selectable_name: key.1,
                        })
                    }
                }
                None => panic!("Unexpected empty vec. This is indicative of a bug in Isograph."),
            })();

            (*key, value)
        })
        .chain(
            get_link_fields_map(db)
                .clone()
                .map_err(MemoizedIsoLiteralError::Diagnostic)?
                .into_iter()
                .map(|(key, value)| ((key.0, key.1.into()), Ok(value.scalar_selected()))),
        )
        .chain(
            expose_field_map(db)
                .clone()?
                .into_iter()
                .map(|(key, (selectable, _))| {
                    ((key.0, key.1.into()), Ok(selectable.scalar_selected()))
                }),
        )
        .collect::<HashMap<_, _>>()
        .ok()
}
