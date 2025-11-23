use std::collections::HashMap;

use common_lang_types::{ServerObjectEntityName, ServerSelectableName};
use intern::Lookup;
use isograph_lang_types::SelectionType;
use pico::MemoRef;
use pico_macros::memo;
use thiserror::Error;

use crate::{
    EntityAccessError, FieldToInsertToServerSelectableError, ID_ENTITY_NAME, ID_FIELD_NAME,
    IsographDatabase, NetworkProtocol, OwnedServerSelectable, ServerObjectSelectable,
    ServerScalarSelectable, field_to_insert_to_server_selectable, server_scalar_entity_named,
};

type OwnedSelectableResult<TNetworkProtocol> =
    Result<OwnedServerSelectable<TNetworkProtocol>, FieldToInsertToServerSelectableError>;

#[expect(clippy::type_complexity)]
#[memo]
pub fn server_selectables_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        ServerObjectEntityName,
        Vec<(
            ServerSelectableName,
            OwnedSelectableResult<TNetworkProtocol>,
        )>,
    >,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
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
) -> Result<
    // TODO return the SelectableId with each Result, i.e. we should know
    // the parent type and selectable name infallibly
    Vec<(
        ServerSelectableName,
        OwnedSelectableResult<TNetworkProtocol>,
    )>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
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
) -> Result<
    HashMap<ServerSelectableName, Vec<OwnedSelectableResult<TNetworkProtocol>>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
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
) -> Result<
    Vec<OwnedSelectableResult<TNetworkProtocol>>,
    TNetworkProtocol::ParseTypeSystemDocumentsError,
> {
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
) -> Result<
    Option<OwnedSelectableResult<TNetworkProtocol>>,
    ServerSelectableNamedError<TNetworkProtocol>,
> {
    let vec =
        server_selectables_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| ServerSelectableNamedError::ParseTypeSystemDocumentsError(e.clone()))?;

    match vec.split_first() {
        Some((first, rest)) => {
            if rest.is_empty() {
                Ok(Some(first.clone()))
            } else {
                Err(ServerSelectableNamedError::MultipleDefinitionsFound {
                    parent_object_entity_name: parent_server_object_entity_name,
                    duplicate_selectable_name: server_selectable_name,
                })
            }
        }
        None => Ok(None),
    }
}

#[memo]
pub fn server_id_selectable<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
) -> Result<
    Option<MemoRef<ServerScalarSelectable<TNetworkProtocol>>>,
    ServerSelectableNamedError<TNetworkProtocol>,
> {
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
            return Err(ServerSelectableNamedError::IncorrectType {
                parent_object_entity_name: parent_server_object_entity_name,
                selectable_name: (*ID_FIELD_NAME).into(),
                expected_type: "a scalar",
                actual_type: "an object",
            });
        }
    };

    let target_scalar_entity = selectable.target_scalar_entity.inner();
    let target_scalar_entity = server_scalar_entity_named(db, *target_scalar_entity)
        .as_ref()
        .map_err(|e| e.clone())?
        .as_ref()
        // It must exist
        .ok_or_else(|| ServerSelectableNamedError::IdFieldMustBeNonNullIdType {
            strong_field_name: selectable.name.item.lookup(),
            parent_object_entity_name: parent_server_object_entity_name,
        })?;

    let options = &db.get_isograph_config().options;

    // And must have the right inner type
    if target_scalar_entity.name != *ID_ENTITY_NAME {
        options.on_invalid_id_type.on_failure(|| {
            ServerSelectableNamedError::IdFieldMustBeNonNullIdType {
                strong_field_name: "id",
                parent_object_entity_name: parent_server_object_entity_name,
            }
        })?;
    }

    // TODO disallow [ID] etc, ID, etc.

    Ok(Some(db.intern_ref(selectable)))
}

#[derive(Clone, Debug, Error, PartialEq, Eq, PartialOrd, Ord)]
pub enum ServerSelectableNamedError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ParseTypeSystemDocumentsError(TNetworkProtocol::ParseTypeSystemDocumentsError),

    // TODO include additional locations
    #[error(
        "Multiple definitions of `{parent_object_entity_name}.{duplicate_selectable_name}` were found"
    )]
    MultipleDefinitionsFound {
        parent_object_entity_name: ServerObjectEntityName,
        duplicate_selectable_name: ServerSelectableName,
    },

    #[error(
        "Expected `{parent_object_entity_name}.{selectable_name}` to be {expected_type}, \
        but it was {actual_type}."
    )]
    IncorrectType {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: ServerSelectableName,
        expected_type: &'static str,
        actual_type: &'static str,
    },

    // TODO this is probably indicative of bad modeling
    #[error("{0}")]
    FieldToInsertToServerSelectableError(#[from] FieldToInsertToServerSelectableError),

    #[error(
        "The `{strong_field_name}` field on `{parent_object_entity_name}` must have type `ID!`.\n\
        This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_object_entity_name: ServerObjectEntityName,
        strong_field_name: &'static str,
    },

    #[error("{0}")]
    EntityAccessError(#[from] EntityAccessError<TNetworkProtocol>),
}

#[memo]
pub fn server_object_selectable_named<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    server_selectable_name: ServerSelectableName,
) -> Result<
    Option<ServerObjectSelectable<TNetworkProtocol>>,
    ServerSelectableNamedError<TNetworkProtocol>,
> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    match item {
        Some(item) => {
            let item = item.as_ref().map_err(|e| e.clone())?;
            match item.as_ref().as_object() {
                Some(obj) => Ok(Some(obj.clone())),
                None => Err(ServerSelectableNamedError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: server_selectable_name,
                    expected_type: "an object",
                    actual_type: "a scalar",
                }),
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
) -> Result<
    Option<ServerScalarSelectable<TNetworkProtocol>>,
    ServerSelectableNamedError<TNetworkProtocol>,
> {
    let item =
        server_selectable_named(db, parent_server_object_entity_name, server_selectable_name)
            .as_ref()
            .map_err(|e| e.clone())?;

    match item {
        Some(item) => {
            let item = item.as_ref().map_err(|e| e.clone())?;
            match item.as_ref().as_scalar() {
                Some(scalar) => Ok(Some(scalar.clone())),
                None => Err(ServerSelectableNamedError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: server_selectable_name,
                    expected_type: "a scalar",
                    actual_type: "an object",
                }),
            }
        }
        None => Ok(None),
    }
}
