use std::collections::BTreeMap;

use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location};
use pico::MemoRef;
use pico_macros::memo;
use prelude::{ErrClone, Postfix};

use crate::{CompilationProfile, IsographDatabase, MemoRefServerEntity, flattened_entity_named};

/// This function just drops the locations
#[memo]
pub fn deprecated_server_entities_map_without_locations<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Result<MemoRef<BTreeMap<EntityName, MemoRefServerEntity<TCompilationProfile>>>, Diagnostic> {
    let (outcome, _fetchable_types) =
        TCompilationProfile::deprecated_parse_type_system_documents(db).clone_err()?;

    outcome
        .item
        .entities
        .iter()
        .map(|(entity_name, entities)| (*entity_name, entities.item))
        .collect::<BTreeMap<_, _>>()
        .interned_value(db)
        .wrap_ok()
}

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
) -> DiagnosticResult<Option<MemoRefServerEntity<TCompilationProfile>>> {
    (*flattened_entity_named(db, name)).wrap_ok()
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
