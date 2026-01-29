use common_lang_types::{Diagnostic, DiagnosticResult, EntityName, Location};
use pico_macros::memo;
use prelude::{ErrClone as _, Postfix};

use crate::{
    CompilationProfile, ID_ENTITY_NAME, ID_FIELD_NAME, IsographDatabase, MemoRefServerSelectable,
    entity_definition_location, entity_not_defined_diagnostic, flattened_entity_named,
    flattened_selectable_named,
};

#[memo]
pub fn server_id_selectable<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
) -> DiagnosticResult<Option<MemoRefServerSelectable<TCompilationProfile>>> {
    let selectable =
        flattened_selectable_named(db, parent_server_object_entity_name, *ID_FIELD_NAME);

    let memo_ref = match selectable {
        Some(s) => s,
        None => return Ok(None),
    };

    let selectable = memo_ref.lookup(db);

    let target_entity_name = selectable.target_entity.item.clone_err()?.inner().0;
    let target_scalar_entity = flattened_entity_named(db, target_entity_name)
        .ok_or_else(|| {
            entity_not_defined_diagnostic(
                target_entity_name,
                Location::Generated
                    .note_todo("Don't be lazy, get a location")
                    .wrap_some(),
            )
        })?
        .lookup(db);

    let options = &db.get_isograph_config().options;

    // And must have the right inner type
    if target_scalar_entity
        .name
        .item
        .note_todo("Compare with *target_scalar_entity_name here")
        != *ID_ENTITY_NAME
    {
        options.on_invalid_id_type.on_failure(|| {
            let strong_field_name = *ID_FIELD_NAME;
            (
                Diagnostic::new(
                    format!(
                        "The `{strong_field_name}` field on \
                        `{parent_server_object_entity_name}` must have type `ID!`.\n\
                        This error can be suppressed using the \
                        \"on_invalid_id_type\" config parameter."
                    ),
                    // TODO use the id field name location
                    entity_definition_location(db, parent_server_object_entity_name)
                        .flatten()
                        .map(|x| x.to::<Location>()),
                ),
                db.print_location_fn(true)
                    .note_todo("It's a bad sign we're calling this fn here"),
            )
        })?;
    }

    // TODO disallow [ID] etc, ID, etc.

    memo_ref.dereference().wrap_some().wrap_ok()
}
