use std::collections::{HashMap, hash_map::Entry};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, ClientSelectableName,
    ParentObjectEntityNameAndSelectableName, ServerObjectEntityName, WithLocation, WithSpan,
};
use isograph_lang_types::SelectionType;
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    AddSelectionSetsError, IsographDatabase, MemoizedIsoLiteralError, NetworkProtocol,
    ObjectSelectableId, ProcessClientFieldDeclarationError, RefetchStrategy, ScalarSelectableId,
    client_selectable_declaration_map_from_iso_literals, expose_field_map,
    get_unvalidated_refetch_stategy, get_validated_refetch_strategy,
};

#[expect(clippy::type_complexity)]
#[legacy_memo]
pub fn unvalidated_refetch_strategy_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    HashMap<
        (ServerObjectEntityName, ClientSelectableName),
        Result<
            SelectionType<Option<RefetchStrategy<(), ()>>, RefetchStrategy<(), ()>>,
            RefetchStrategyAccessError<TNetworkProtocol>,
        >,
    >,
    RefetchStrategyAccessError<TNetworkProtocol>,
> {
    // TODO use a "list of iso declarations" fn
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);
    let expose_field_map = expose_field_map(db).as_ref().map_err(|e| e.clone())?;

    let mut out = HashMap::new();

    for (key, value) in declaration_map {
        for item in value {
            match out.entry(*key) {
                Entry::Occupied(mut occupied_entry) => {
                    // TODO check for length instead
                    *occupied_entry.get_mut() =
                        Err(RefetchStrategyAccessError::DuplicateDefinition {
                            parent_object_entity_name: key.0,
                            client_selectable_name: key.1,
                        })
                }
                Entry::Vacant(vacant_entry) => match item {
                    SelectionType::Scalar(_) => {
                        let refetch_strategy = get_unvalidated_refetch_stategy(db, key.0)
                            .map_err(|e| e.into())
                            .map(SelectionType::Scalar);
                        vacant_entry.insert(refetch_strategy);
                    }
                    SelectionType::Object(o) => {
                        // HACK ALERT
                        // For client pointers, the refetch strategy is based on the "to" object type.
                        // This is extremely weird, and we should fix this!
                        let refetch_strategy =
                            get_unvalidated_refetch_stategy(db, o.target_type.inner().0)
                                .map_err(|e| e.into())
                                .map(|item| {
                                    SelectionType::Object(item.expect(
                                "Expected client object selectable to have a refetch strategy. \
                                This is indicative of a bug in Isograph.",
                            ))
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
                *occupied_entry.get_mut() = Err(RefetchStrategyAccessError::DuplicateDefinition {
                    parent_object_entity_name: key.0,
                    client_selectable_name: key.1.into(),
                });
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(Ok(SelectionType::Scalar(
                    selection_set.refetch_strategy.clone(),
                )));
            }
        }
    }

    Ok(out)
}

#[expect(clippy::type_complexity)]
#[legacy_memo]
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
            RefetchStrategyAccessError<TNetworkProtocol>,
        >,
    >,
    RefetchStrategyAccessError<TNetworkProtocol>,
> {
    let map = unvalidated_refetch_strategy_map(db).clone()?;

    Ok(map
        .into_iter()
        .map(|(key, value)| {
            let value: Result<_, RefetchStrategyAccessError<_>> = value.and_then(|opt| match opt {
                SelectionType::Scalar(refetch_strategy) => {
                    Ok(SelectionType::Scalar(refetch_strategy.map(|refetch_strategy| {
                        get_validated_refetch_strategy(
                            db,
                            refetch_strategy,
                            key.0,
                            SelectionType::Scalar(ParentObjectEntityNameAndSelectableName::new(
                                key.0,
                                key.1.into(),
                            )),
                        )
                    }).transpose()?))
                }
                SelectionType::Object(refetch_strategy) => Ok(SelectionType::Object(get_validated_refetch_strategy(
                    db,
                    refetch_strategy,
                    key.0,
                    SelectionType::Object(ParentObjectEntityNameAndSelectableName::new(
                        key.0,
                        key.1.into(),
                    )),
                )
                .map_err(|e| {
                    RefetchStrategyAccessError::ValidateAddSelectionSetsResultWithMultipleErrors {
                        errors: e,
                    }
                })?)),
            });

            (key, value)
        })
        .collect())
}

#[legacy_memo]
pub fn validated_refetch_strategy_for_client_scalar_selectable_named<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_scalar_selectable_name: ClientScalarSelectableName,
) -> Result<
    Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
    RefetchStrategyAccessError<TNetworkProtocol>,
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
                SelectionType::Object(_) => Err(RefetchStrategyAccessError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: client_scalar_selectable_name.into(),
                    expected_type: "a scalar",
                    actual_type: "an object",
                }),
                SelectionType::Scalar(s) => Ok(s.clone()),
            },
            Err(e) => Err(e.clone()),
        },
        None => Err(RefetchStrategyAccessError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_scalar_selectable_name.into(),
        }),
    }
}

#[legacy_memo]
pub fn validated_refetch_strategy_for_object_scalar_selectable_named<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: ClientObjectSelectableName,
) -> Result<
    RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,
    RefetchStrategyAccessError<TNetworkProtocol>,
> {
    let map = validated_refetch_strategy_map(db)
        .as_ref()
        .map_err(|e| e.clone())?;

    match map.get(&(
        parent_server_object_entity_name,
        client_object_selectable_name.into(),
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Scalar(_) => Err(RefetchStrategyAccessError::IncorrectType {
                    parent_object_entity_name: parent_server_object_entity_name,
                    selectable_name: client_object_selectable_name.into(),
                    expected_type: "an object",
                    actual_type: "a scalar",
                }),
                SelectionType::Object(s) => Ok(s.clone()),
            },
            Err(e) => Err(e.clone()),
        },
        None => Err(RefetchStrategyAccessError::NotFound {
            parent_server_object_entity_name,
            selectable_name: client_object_selectable_name.into(),
        }),
    }
}
#[derive(Clone, Error, Eq, PartialEq, Debug)]
pub enum RefetchStrategyAccessError<TNetworkProtocol: NetworkProtocol> {
    #[error("{0}")]
    ProcessClientFieldDeclarationError(
        #[from] WithSpan<ProcessClientFieldDeclarationError<TNetworkProtocol>>,
    ),

    #[error("{0}")]
    MemoizedIsoLiteralError(#[from] MemoizedIsoLiteralError<TNetworkProtocol>),

    #[error("`{parent_object_entity_name}.{client_selectable_name}` has been defined twice.")]
    DuplicateDefinition {
        parent_object_entity_name: ServerObjectEntityName,
        client_selectable_name: ClientSelectableName,
    },

    #[error("{0}", 
        errors.iter().map(|error| format!("{}", error.for_display())).collect::<Vec<_>>().join("\n")
    )]
    ValidateAddSelectionSetsResultWithMultipleErrors {
        #[from]
        errors: Vec<WithLocation<AddSelectionSetsError<TNetworkProtocol>>>,
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
