use common_lang_types::{Diagnostic, EntityName, Location};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, IsographDatabase, MemoRefServerEntity, flattened_entities,
    flattened_entity_named,
};

#[memo]
pub fn deprecated_server_object_entities<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<MemoRefServerEntity<TCompilationProfile>> {
    let entities = flattened_entities(db);

    entities
        .iter()
        .filter_map(|(_, x)| {
            if x.lookup(db).selection_info.as_object().is_some() {
                x.dereference().wrap_some()
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
}

#[memo]
pub fn server_entity_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    name: EntityName,
) -> Option<MemoRefServerEntity<TCompilationProfile>> {
    flattened_entity_named(db, name).dereference()
}

pub fn entity_not_defined_diagnostic(entity_name: EntityName, location: Location) -> Diagnostic {
    Diagnostic::new(
        format!("`{entity_name}` is not defined."),
        location.wrap_some(),
    )
}
