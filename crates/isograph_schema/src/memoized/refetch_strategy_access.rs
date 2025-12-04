use std::collections::{HashMap, hash_map::Entry};

use common_lang_types::{
    DiagnosticResult, DiagnosticVecResult, Location, ParentObjectEntityNameAndSelectableName,
    SelectableName, ServerObjectEntityName,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    IsographDatabase, NetworkProtocol, ObjectSelectableId, RefetchStrategy, ScalarSelectableId,
    client_selectable_declaration_map_from_iso_literals, get_unvalidated_refetch_stategy,
    get_validated_refetch_strategy, multiple_selectable_definitions_found_diagnostic,
    selectable_is_not_defined_diagnostic, selectable_is_wrong_type_diagnostic,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn unvalidated_refetch_strategy_map<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<
    HashMap<
        (ServerObjectEntityName, SelectableName),
        DiagnosticResult<SelectionType<Option<RefetchStrategy<(), ()>>, RefetchStrategy<(), ()>>>,
    >,
> {
    // TODO use a "list of iso declarations" fn
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);

    let mut out = HashMap::new();

    for (key, with_location) in &declaration_map.item {
        let item = with_location.item;
        match out.entry(*key) {
            Entry::Occupied(mut occupied_entry) => {
                // TODO check for length instead
                *occupied_entry.get_mut() = multiple_selectable_definitions_found_diagnostic(
                    key.0,
                    key.1,
                    Location::Generated,
                )
                .wrap_err()
            }
            Entry::Vacant(vacant_entry) => match item {
                SelectionType::Scalar(_) => {
                    let refetch_strategy =
                        get_unvalidated_refetch_stategy(db, key.0).map(SelectionType::Scalar);
                    vacant_entry.insert(refetch_strategy);
                }
                SelectionType::Object(o) => {
                    // HACK ALERT
                    // For client pointers, the refetch strategy is based on the "to" object type.
                    // This is extremely weird, and we should fix this!
                    let refetch_strategy =
                        get_unvalidated_refetch_stategy(db, o.lookup(db).target_type.inner().0)
                            .map(|item| {
                                item.expect(
                                    "Expected client object selectable \
                                        to have a refetch strategy. \
                                        This is indicative of a bug in Isograph.",
                                )
                                .object_selected()
                            });
                    vacant_entry.insert(refetch_strategy);
                }
            },
        }
    }

    let outcome = TNetworkProtocol::parse_type_system_documents(db).clone_err()?;
    let expose_fields = &outcome.0.item.client_scalar_refetch_strategies;

    for with_location in expose_fields.iter().flatten() {
        let (parent_object_entity_name, selectable_name, refetch_strategy) = &with_location.item;
        match out.entry((*parent_object_entity_name, (*selectable_name).into())) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = multiple_selectable_definitions_found_diagnostic(
                    *parent_object_entity_name,
                    (*selectable_name).into(),
                    with_location.location,
                )
                .wrap_err();
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(
                    refetch_strategy
                        .clone()
                        .wrap_some()
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
) -> DiagnosticVecResult<
    HashMap<
        (ServerObjectEntityName, SelectableName),
        DiagnosticVecResult<
            SelectionType<
                Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>,
                RefetchStrategy<ScalarSelectableId, ObjectSelectableId>,
            >,
        >,
    >,
> {
    let map = unvalidated_refetch_strategy_map(db)
        .note_todo("Do not clone. Use a MemoRef.")
        .clone()?;

    map.into_iter()
        .map(|(key, value)| {
            let value = value.map_err(|e| vec![e]).and_then(|opt| match opt {
                SelectionType::Scalar(refetch_strategy) => refetch_strategy
                    .map(|refetch_strategy| {
                        get_validated_refetch_strategy(
                            db,
                            refetch_strategy,
                            key.0,
                            ParentObjectEntityNameAndSelectableName::new(key.0, key.1)
                                .scalar_selected(),
                        )
                    })
                    .transpose()?
                    .scalar_selected()
                    .wrap_ok(),
                SelectionType::Object(refetch_strategy) => get_validated_refetch_strategy(
                    db,
                    refetch_strategy,
                    key.0,
                    ParentObjectEntityNameAndSelectableName::new(key.0, key.1).object_selected(),
                )?
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
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticVecResult<Option<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>>> {
    let map = validated_refetch_strategy_map(db).clone_err()?;

    match map.get(&(
        parent_server_object_entity_name,
        client_scalar_selectable_name.into(),
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Object(_) => vec![selectable_is_wrong_type_diagnostic(
                    parent_server_object_entity_name,
                    client_scalar_selectable_name.into(),
                    "a scalar",
                    "an object",
                    Location::Generated,
                )]
                .wrap_err(),
                SelectionType::Scalar(s) => s
                    .clone()
                    .note_todo("Do not clone. Use a MemoRef.")
                    .wrap_ok(),
            },
            Err(e) => e.clone().wrap_err(),
        },
        None => vec![selectable_is_not_defined_diagnostic(
            parent_server_object_entity_name,
            client_scalar_selectable_name.into(),
            Location::Generated,
        )]
        .wrap_err(),
    }
}

#[memo]
pub fn validated_refetch_strategy_for_object_scalar_selectable_named<
    TNetworkProtocol: NetworkProtocol,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    parent_server_object_entity_name: ServerObjectEntityName,
    client_object_selectable_name: SelectableName,
) -> DiagnosticVecResult<RefetchStrategy<ScalarSelectableId, ObjectSelectableId>> {
    let map = validated_refetch_strategy_map(db).clone_err()?;

    match map.get(&(
        parent_server_object_entity_name,
        client_object_selectable_name.into(),
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Scalar(_) => vec![selectable_is_wrong_type_diagnostic(
                    parent_server_object_entity_name,
                    client_object_selectable_name.into(),
                    "an object",
                    "a scalar",
                    Location::Generated,
                )]
                .wrap_err(),
                SelectionType::Object(s) => s
                    .clone()
                    .note_todo("Do not clone. Use a MemoRef.")
                    .wrap_ok(),
            },
            Err(e) => e.clone().wrap_err(),
        },
        None => vec![selectable_is_not_defined_diagnostic(
            parent_server_object_entity_name,
            client_object_selectable_name.into(),
            Location::Generated,
        )]
        .wrap_err(),
    }
}
