use common_lang_types::{
    DiagnosticResult, EntityName, ExpectSelectableToExist, SelectableName, WithEmbeddedLocation,
};
use isograph_lang_types::SelectionSet;
use prelude::ErrClone;

use crate::{
    ClientFieldVariant, CompilationProfile, IsographDatabase,
    refetch_strategy_for_client_scalar_selectable_named, selectable_named,
    selectable_reader_selection_set,
};

pub fn client_scalar_selectable_selection_set_for_parent_query<
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    client_scalar_selectable_name: SelectableName,
) -> DiagnosticResult<WithEmbeddedLocation<SelectionSet>> {
    let selectable = selectable_named(db, parent_object_entity_name, client_scalar_selectable_name)
        .clone_err()?
        .expect_selectable_to_exist(parent_object_entity_name, client_scalar_selectable_name)
        .as_client()
        .expect(
            "Expected client selectable. \
        This is indicative of a bug in Isograph.",
        )
        .as_scalar()
        .expect(
            "Expected client scalar selectable. \
        This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    Ok(match selectable.variant {
        ClientFieldVariant::ImperativelyLoadedField(_) => {
            let refetch_strategy = refetch_strategy_for_client_scalar_selectable_named(
                db,
                parent_object_entity_name,
                client_scalar_selectable_name,
            )
            .as_ref()
            .expect(
                "Expected imperatively loaded field to have refetch selection set. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected imperatively loaded field to have refetch selection set. \
                This is indicative of a bug in Isograph.",
            );

            refetch_strategy
                .refetch_selection_set()
                .expect(
                    "Expected imperatively loaded field to have refetch selection set. \
                    This is indicative of a bug in Isograph.",
                )
                // TODO don't clone
                .clone()
        }
        _ => {
            // TODO don't clone
            selectable_reader_selection_set(
                db,
                parent_object_entity_name,
                client_scalar_selectable_name,
            )
            .as_ref()
            .expect("Expected selection set to be valid.")
            .lookup(db)
            .clone()
        }
    })
}
