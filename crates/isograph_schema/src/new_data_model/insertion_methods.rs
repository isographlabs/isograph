use std::collections::{BTreeMap, btree_map::Entry};

use common_lang_types::{
    Diagnostic, EntityName, Location, SelectableName, WithNonFatalDiagnostics, WithOptionalLocation,
};
use prelude::Postfix;

use crate::{
    CompilationProfile, NestedDataModelEntity, NestedDataModelSchema, NestedDataModelSelectable,
    multiple_selectable_definitions_found_diagnostic,
};

// TODO these should be one method
pub fn insert_entity_into_schema_or_emit_multiple_definitions_diagnostic<
    TCompilationProfile: CompilationProfile,
>(
    schema: &mut NestedDataModelSchema<TCompilationProfile>,
    entity: WithOptionalLocation<NestedDataModelEntity<TCompilationProfile>>,
) {
    let key = entity.item.name.item;
    match schema.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            // TODO parse graphql schema should wrap the items with locations
            vacant_entry.insert(entity);
        }
        Entry::Occupied(occupied_entry) => {
            schema
                .non_fatal_diagnostics
                .push(multiple_entity_definitions_found_diagnostic(
                    key,
                    occupied_entry.get().location.map(|x| x.to::<Location>()),
                ));
        }
    }
}

pub fn insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic<
    TCompilationProfile: CompilationProfile,
>(
    selectables: &mut WithNonFatalDiagnostics<
        BTreeMap<SelectableName, NestedDataModelSelectable<TCompilationProfile>>,
    >,
    item: NestedDataModelSelectable<TCompilationProfile>,
) {
    let key = item.name.item;
    match selectables.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            // TODO parse graphql schema should wrap the items with locations
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => {
            selectables.non_fatal_diagnostics.push(
                multiple_selectable_definitions_found_diagnostic(
                    item.parent_entity_name.item,
                    key,
                    // TODO proper location
                    None,
                ),
            );
        }
    }
}

fn multiple_entity_definitions_found_diagnostic(
    server_object_entity_name: EntityName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("Multiple definitions of `{server_object_entity_name}` were found."),
        location,
    )
}
