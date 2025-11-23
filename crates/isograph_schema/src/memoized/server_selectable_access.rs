use std::collections::HashMap;

use common_lang_types::{
    Diagnostic, DiagnosticResult, ServerObjectEntityName, ServerSelectableName,
};
use isograph_lang_types::SelectionType;
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    ID_ENTITY_NAME, ID_FIELD_NAME, IsographDatabase, NetworkProtocol, OwnedServerSelectable,
    ServerObjectSelectable, ServerScalarSelectable, entity_definition_location,
    field_to_insert_to_server_selectable, server_scalar_entity_named,
};

type OwnedSelectableResult<TNetworkProtocol> =
    DiagnosticResult<OwnedServerSelectable<TNetworkProtocol>>;

#[expect(clippy::type_complexity)]
#[memo]
pub fn server_selectables_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<
    HashMap<
        ServerObjectEntityName,
        Vec<(
            ServerSelectableName,
            OwnedSelectableResult<TNetworkProtocol>,
        )>,
    >,
> {
    let (items, _fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    Ok(items
        .iter()
        .flat_map(|selection_type| selection_type.as_ref().as_object())
        .map(|object_outcome| {
            let fields = object_outcome
                .fields_to_insert
                .iter()
                .map(|field_to_insert| {
                    (
                        field_to_insert.item.name.item,
                        field_to_insert_to_server_selectable(
                            db,
                            object_outcome.server_object_entity.item.name,
                            field_to_insert,
                        )
                        .map(|x| x.map_scalar(|(scalar, _)| scalar)),
                    )
                })
                .collect();

            (object_outcome.server_object_entity.item.name, fields)
        })
        .collect())
}

/// A vector of all server selectables that are defined in the type system schema
/// for a given entity
#[memo]
pub fn server_selectables_vec_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<
    // TODO return the SelectableId with each Result, i.e. we should know
    // the parent type and selectable name infallibly
    Vec<(
        ServerSelectableName,
        OwnedSelectableResult<TNetworkProtocol>,
    )>,
> {
    let (items, _fetchable_types) = TNetworkProtocol::parse_type_system_documents(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    Ok(items
        .iter()
        .flat_map(|selection_type| selection_type.as_ref().as_object())
        .filter(|o| o.server_object_entity.item.name == parent_server_object_entity_name)
        .flat_map(|o| {
            o.fields_to_insert.iter().map(|field_to_insert| {
                (
                    field_to_insert.item.name.item,
                    field_to_insert_to_server_selectable(
                        db,
                        parent_server_object_entity_name,
                        field_to_insert,
                    )
                    .map(|x| x.map_scalar(|(scalar, _)| scalar)),
                )
            })
        })
        .collect())
}

#[memo]
pub fn server_selectables_map_for_entity<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<HashMap<ServerSelectableName, Vec<OwnedSelectableResult<TNetworkProtocol>>>> {
    let server_selectables =
        server_selectables_vec_for_entity(db, parent_server_object_entity_name).to_owned()?;
    let mut map: HashMap<_, Vec<_>> = HashMap::new();

    for (name, item) in server_selectables {
        map.entry(name).or_default().push(item);
    }

    Ok(map)
}

#[memo]
pub fn server_selectables_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    server_selectable_name: ServerSelectableName,
) -> DiagnosticResult<Vec<OwnedSelectableResult<TNetworkProtocol>>> {
    let map = server_selectables_map_for_entity(db, parent_server_object_entity_name)
        .as_ref()
        .map_err(|e| e.clone())?;

    Ok(map
        .get(&server_selectable_name)
        .cloned()
        .unwrap_or_default())
}

#[memo]
pub fn server_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    server_selectable_name: ServerSelectableName,
) -> DiagnosticResult<Option<OwnedSelectableResult<TNetworkProtocol>>> {
    let vec =
        server_selectables_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    match vec.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                Ok(Some(first.clone()))
            } else {
                Diagnostic::new(
                    format!(
                        "Multiple definitions of \
                        `{parent_server_object_entity_name}.{server_selectable_name}` were found"
                    ),
                    Result::ok(
                        entity_definition_location(db, parent_server_object_entity_name.into())
                            .as_ref(),
                    )
                    .cloned()
                    .flatten(),
                )
                .wrap_err()
            }
        }
        None => Ok(None),
    }
}

#[memo]
pub fn server_id_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> DiagnosticResult<Option<MemoRef<ServerScalarSelectable<TNetworkProtocol>>>> {
    let selectable = server_selectable_named(
        db,
        parent_server_object_entity_name,
        (*ID_FIELD_NAME).into(),
    )
    .as_ref()
    .map_err(|e| e.clone())?;

    let selectable = match selectable {
        Some(s) => s.as_ref().map_err(|e| e.clone())?,
        None => return Ok(None),
    };

    // TODO check if it is a client field...
    let selectable = match selectable {
        SelectionType::Scalar(s) => s,
        SelectionType::Object(_) => {
            let selectable_name = *ID_FIELD_NAME;
            return Diagnostic::new(
                format!(
                    "Expected `{parent_server_object_entity_name}.{selectable_name}` \
                    to be a scalar, but it was an object."
                ),
                Result::ok(
                    entity_definition_location(db, parent_server_object_entity_name.into())
                        .as_ref(),
                )
                .cloned()
                .flatten(),
            )
            .wrap_err();
        }
    };

    let target_scalar_entity_name = selectable.target_scalar_entity.inner();
    let target_scalar_entity = server_scalar_entity_named(db, *target_scalar_entity_name)
        .as_ref()
        .map_err(|e| e.clone())?
        .as_ref()
        // It must exist
        .ok_or_else(|| {
            let id_field_name = *ID_FIELD_NAME;
            Diagnostic::new(
                // TODO: it doesn't seem like this error is actually suppresable (here).
                format!(
                    "The `{id_field_name}` field on \
                    `{target_scalar_entity_name}` must have type `ID!`.\n\
                    This error can be suppressed using the \
                    \"on_invalid_id_type\" config parameter."
                ),
                // TODO use the location of the selectable
                Result::ok(
                    entity_definition_location(db, (*target_scalar_entity_name).into()).as_ref(),
                )
                .cloned()
                .flatten(),
            )
        })?;

    let options = &db.get_isograph_config().options;

    // And must have the right inner type
    if target_scalar_entity.name != *ID_ENTITY_NAME {
        options.on_invalid_id_type.on_failure(|| {
            let strong_field_name = *ID_FIELD_NAME;
            Diagnostic::new(
                format!(
                    "The `{strong_field_name}` field on \
                    `{parent_server_object_entity_name}` must have type `ID!`.\n\
                    This error can be suppressed using the \
                    \"on_invalid_id_type\" config parameter."
                ),
                Result::ok(
                    entity_definition_location(db, parent_server_object_entity_name.into())
                        .as_ref(),
                )
                .cloned()
                .flatten(),
            )
        })?;
    }

    // TODO disallow [ID] etc, ID, etc.

    Ok(Some(db.intern_ref(selectable)))
}

#[memo]
pub fn server_object_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    server_selectable_name: ServerSelectableName,
) -> DiagnosticResult<Option<ServerObjectSelectable<TNetworkProtocol>>> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    match item {
        Some(item) => {
            let item = item.as_ref().map_err(|e| e.clone())?;
            match item.as_ref().as_object() {
                Some(obj) => Ok(Some(obj.clone())),
                None => Diagnostic::new(
                    format!(
                        "Expected `{parent_server_object_entity_name}.{server_selectable_name}`\
                        to be an object, but it was a scalar."
                    ),
                    Result::ok(
                        entity_definition_location(db, parent_server_object_entity_name.into())
                            .as_ref(),
                    )
                    .cloned()
                    .flatten(),
                )
                .wrap_err(),
            }
        }
        None => Ok(None),
    }
}

#[memo]
pub fn server_scalar_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    server_selectable_name: ServerSelectableName,
) -> DiagnosticResult<Option<ServerScalarSelectable<TNetworkProtocol>>> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    match item {
        Some(item) => {
            let item = item.as_ref().map_err(|e| e.clone())?;
            match item.as_ref().as_scalar() {
                Some(scalar) => Ok(Some(scalar.clone())),
                None => Diagnostic::new(
                    format!(
                        "Expected `{parent_server_object_entity_name}.{server_selectable_name}` \
                        to be a scalar, but it was an object."
                    ),
                    Result::ok(
                        entity_definition_location(db, parent_server_object_entity_name.into())
                            .as_ref(),
                    )
                    .cloned()
                    .flatten(),
                )
                .wrap_err(),
            }
        }
        None => Ok(None),
    }
}
