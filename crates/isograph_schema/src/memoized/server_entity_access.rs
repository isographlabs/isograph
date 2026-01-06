use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location};
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{CompilationProfile, IsographDatabase, MemoRefServerEntity, flattened_entity_named};

#[memo]
pub fn deprecated_server_object_entities<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<Vec<MemoRefServerEntity<TCompilationProfile>>> {
    let (outcome, _) =
        TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .iter()
        .filter_map(|(_, x)| {
            if x.item.lookup(db).selection_info.as_object().is_some() {
                x.item.wrap_some()
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}

#[memo]
pub fn server_entity_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    name: EntityName,
) -> Option<MemoRefServerEntity<TCompilationProfile>> {
    flattened_entity_named(db, name).dereference()
}

#[memo]
pub fn entity_definition_location<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> DiagnosticResult<Option<Location>> {
    let (outcome, _) =
        TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .get(&entity_name)
        .map(|x| x.location)
        .wrap_ok()
}

pub fn entity_not_defined_diagnostic(entity_name: EntityName, location: Location) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}
