use std::collections::{HashMap, hash_map::Entry};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName, Diagnostic,
    ParentObjectEntityNameAndSelectableName, ServerObjectEntityName, WithLocation,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::Postfix;
use thiserror::Error;

use crate::{
    AddSelectionSetsError, IsographDatabase, NetworkProtocol, ObjectSelectableId, RefetchStrategy,
    ScalarSelectableId, client_selectable_declaration_map_from_iso_literals, expose_field_map,
    get_unvalidated_refetch_stategy, get_validated_refetch_strategy,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn unvalidated_refetch_strategy_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientSelectableName),
        Result<
            SelectionType<Option<RefetchStrategy<(), ()>>, RefetchStrategy<(), ()>>,
            RefetchStrategyAccessError,
        >,
    >,
    RefetchStrategyAccessError,
> {
    // TODO use a "list of iso declarations" fn
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);
    let expose_field_map = expose_field_map(db)
        .as_ref()
        .map_err(|e| RefetchStrategyAccessError::Diagnostic(e.clone()))?;

    let mut out = HashMap::new();

    for (key, value) in declaration_map {
        for item in value {
            match out.entry(*key) {
                Entry::Occupied(mut occupied_entry) => {
                    // TODO check for length instead
                    *occupied_entry.get_mut() = RefetchStrategyAccessError::DuplicateDefinition {
                        parent_object_entity_name: key.0,
                        client_selectable_name: key.1,
                    }
                    .wrap_err()
                }
                Entry::Vacant(vacant_entry) => match item {
                    SelectionType::Scalar(_) => {
                        let refetch_strategy = get_unvalidated_refetch_stategy(db, key.0)
                            .map_err(RefetchStrategyAccessError::Diagnostic)
                            .map(SelectionType::Scalar);
                        vacant_entry.insert(refetch_strategy);
                    }
                    SelectionType::Object(o) => {
                        // HACK ALERT
                        // For client pointers, the refetch strategy is based on the "to" object type.
                        // This is extremely weird, and we should fix this!
                        let refetch_strategy =
                            get_unvalidated_refetch_stategy(db, o.target_type.inner().0)
                                .map_err(RefetchStrategyAccessError::Diagnostic)
                                .map(|item| {
                                    item.expect(
                                "Expected client object selectable to have a refetch strategy. \
                                This is indicative of a bug in Isograph.",
                            )
                            .object_selected()
                                });
                        vacant_entry.insert(refetch_strategy);
                    }
                },
            }
        }
    }

    for (key, (_, selection_set)) in expose_field_map {
        match out.entry((key.0, key.1.into())) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = RefetchStrategyAccessError::DuplicateDefinition {
                    parent_object_entity_name: key.0,
                    client_selectable_name: key.1.into(),
                }
                .wrap_err();
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(
                    selection_set
                        .refetch_strategy
                        .clone()
                        .scalar_selected()
                        .wrap_ok(),
                );
            }
        }
    }

    Ok(out)
}

#[expect(clippy::type_complexity)]
#[memo]
pub fn validated_refetch_strategy_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientSelectableName),
        Result<
            SelectionType<
                Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
                RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,
            >,
            RefetchStrategyAccessError,
        >,
    >,
    RefetchStrategyAccessError,
> {
    let map = unvalidated_refetch_strategy_map(db).clone()?;

    map.into_iter()
        .map(|(key, value)| {
            let value: Result<_, RefetchStrategyAccessError> = value.and_then(|opt| match opt {
                SelectionType::Scalar(refetch_strategy) => refetch_strategy
                    .map(|refetch_strategy| {
                        get_validated_refetch_strategy(
                            db,
                            refetch_strategy,
                            key.0,
                            ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into())
                                .scalar_selected(),
                        )
                    })
                    .transpose()
                    .map_err(|e| RefetchStrategyAccessError::AddSelectionSetErrors { errors: e })?
                    .scalar_selected()
                    .wrap_ok(),
                SelectionType::Object(refetch_strategy) => get_validated_refetch_strategy(
                    db,
                    refetch_strategy,
                    key.0,
                    ParentObjectEntityNameAndSelectableName::new(key.0, key.1.into())
                        .object_selected(),
                )
                .map_err(|e| RefetchStrategyAccessError::AddSelectionSetErrors { errors: e })?
                .object_selected()
                .wrap_ok(),
            });

            (key, value)
        })
        .collect::<HashMap<_, _>>()
        .wrap_ok()
}

#[memo]
pub fn validated_refetch_strategy_for_client_scalar_selectable_named<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
) -> Result<
    Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
    RefetchStrategyAccessError,
> {
    let map = validated_refetch_strategy_map(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    match map.get(&(
        parent_server_object_entity_name,
        client_scalar_selectable_name.into(),
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Object(_) => RefetchStrategyAccessError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: client_scalar_selectable_name.into(),
                    expected_type: "a scalar",
                    actual_type: "an object",
                }
                .wrap_err(),
                SelectionType::Scalar(s) => s.clone().wrap_ok(),
            },
            Err(e) => e.clone().wrap_err(),
        },
        None => RefetchStrategyAccessError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_scalar_selectable_name.into(),
        }
        .wrap_err(),
    }
}

#[memo]
pub fn validated_refetch_strategy_for_object_scalar_selectable_named<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: ClientObjectSelectableName,
) -> Result<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>, RefetchStrategyAccessError> {
    let map = validated_refetch_strategy_map(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    match map.get(&(
        parent_server_object_entity_name,
        client_object_selectable_name.into(),
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Scalar(_) => RefetchStrategyAccessError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: client_object_selectable_name.into(),
                    expected_type: "an object",
                    actual_type: "a scalar",
                }
                .wrap_err(),
                SelectionType::Object(s) => s.clone().wrap_ok(),
            },
            Err(e) => e.clone().wrap_err(),
        },
        None => RefetchStrategyAccessError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_object_selectable_name.into(),
        }
        .wrap_err(),
    }
}
#[derive(Clone, Error, Eq, PartialEq, Debug)]
pub enum RefetchStrategyAccessError {
    #[error("{0}")]
    Diagnostic(Diagnostic),

    #[error("`{parent_object_entity_name}.{client_selectable_name}` has been defined twice.")]
    DuplicateDefinition {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
    },

    #[error("{0}", 
        errors.iter().map(|error| format!("{}", error.for_display())).collect::<Vec<_>>().join("\n")
    )]
    AddSelectionSetErrors {
        errors: Vec<WithLocation<AddSelectionSetsError>>,
    },

    #[error(
        "Expected `{parent_object_entity_name}.{selectable_name}` to be {expected_type}, \
        but it was {actual_type}."
    )]
    IncorrectType {
        parent_object_entity_name: ServerObjectEntityName,
        selectable_name: ClientSelectableName,
        expected_type: &'static str,
        actual_type: &'static str,
    },

    // TODO this should be an option in the return value, not an error variant, but
    // realistically, that's super annoying.
    #[error("`{parent_server_object_entity_name}.{selectable_name}` is not defined.")]
    NotFound {
        parent_server_object_entity_name: ServerObjectEntityName,
        selectable_name: ClientSelectableName,
    },
}
