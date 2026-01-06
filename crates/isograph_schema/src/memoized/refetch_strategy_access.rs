use std::collections::{HashMap, hash_map::Entry};

use common_lang_types::{
    DiagnosticResult, DiagnosticVecResult, EntityName, Location, SelectableName,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{
    CompilationProfile, IsographDatabase, RefetchStrategy,
    client_selectable_declaration_map_from_iso_literals, get_refetch_stategy,
    multiple_selectable_definitions_found_diagnostic, selectable_is_not_defined_diagnostic,
    selectable_is_wrong_type_diagnostic,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn refetch_strategy_map<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticVecResult<
    HashMap<
        (EntityName, SelectableName),
        DiagnosticResult<SelectionType<Option<RefetchStrategy>, ()>>,
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
                *occupied_entry.get_mut() =
                    multiple_selectable_definitions_found_diagnostic(key.0, key.1, None).wrap_err()
            }
            Entry::Vacant(vacant_entry) => match item {
                SelectionType::Scalar(_) => {
                    let refetch_strategy =
                        get_refetch_stategy(db, key.0).map(SelectionType::Scalar);
                    vacant_entry.insert(refetch_strategy);
                }
                SelectionType::Object(_) => {
                    // client pointers do not have a refetch strategy. They aren't refetchable
                    // at the moment.
                    //
                    // Or we get it through some other way. Either way, we just put in
                    // a dummy value, which is weird, but at least we're not doing a bunch of extra
                    // work
                    vacant_entry.insert(().object_selected().wrap_ok());
                }
            },
        }
    }

    let outcome = TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;
    let expose_fields = &outcome.0.item.client_scalar_refetch_strategies;

    for with_location in expose_fields.iter().flatten() {
        let (parent_object_entity_name, selectable_name, refetch_strategy) = &with_location.item;
        match out.entry((*parent_object_entity_name, (*selectable_name))) {
            Entry::Occupied(mut occupied_entry) => {
                *occupied_entry.get_mut() = multiple_selectable_definitions_found_diagnostic(
                    *parent_object_entity_name,
                    *selectable_name,
                    with_location.location.wrap_some(),
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

#[memo]
pub fn refetch_strategy_for_client_scalar_selectable_named<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticVecResult<Option<RefetchStrategy>> {
    let map = refetch_strategy_map(db).clone_err()?;

    match map.get(&(
        parent_server_object_entity_name,
        client_scalar_selectable_name,
    )) {
        Some(result) => match result {
            Ok(selection_type) => match selection_type {
                SelectionType::Object(_) => vec![selectable_is_wrong_type_diagnostic(
                    parent_server_object_entity_name,
                    client_scalar_selectable_name,
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
            Err(e) => e.clone().wrap_vec().wrap_err(),
        },
        None => vec![selectable_is_not_defined_diagnostic(
            parent_server_object_entity_name,
            client_scalar_selectable_name,
            Location::Generated,
        )]
        .wrap_err(),
    }
}
