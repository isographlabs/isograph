use std::collections::HashMap;

use common_lang_types::{
    DiagnosticResult, EmbeddedLocation, EntityName, SelectableName, WithEmbeddedLocation,
    WithLocationPostfix,
};
use isograph_lang_types::{SelectionSet, SelectionType, SelectionTypePostfix};
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, IsographDatabase, RefetchStrategy,
    client_selectable_declaration_map_from_iso_literals, get_link_fields,
};

#[expect(clippy::type_complexity)]
#[memo]
pub fn reader_selection_set_map<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> HashMap<
    (EntityName, SelectableName),
    DiagnosticResult<
        SelectionType<
            MemoRef<WithEmbeddedLocation<SelectionSet>>,
            MemoRef<WithEmbeddedLocation<SelectionSet>>,
        >,
    >,
> {
    // TODO use client_selectable_map
    let declaration_map = client_selectable_declaration_map_from_iso_literals(db);

    let mut map: HashMap<_, _> = declaration_map
        .item
        .iter()
        .map(|(key, declaration)| match declaration.item {
            SelectionType::Scalar(s) => (
                *key,
                s.lookup(db)
                    .selection_set
                    .reference()
                    .interned_ref(db)
                    .scalar_selected()
                    .wrap_ok(),
            ),
            SelectionType::Object(o) => (
                *key,
                o.lookup(db)
                    .selection_set
                    .reference()
                    .interned_ref(db)
                    .object_selected()
                    .wrap_ok(),
            ),
        })
        .collect();

    // we must add empty selection sets for __link fields, and TODO perhaps others.
    // TODO this should be cleaned up and thought about holistically.
    let fields = get_link_fields(db);
    for field in fields {
        let scalar_selectable = field.lookup(db);
        map.insert(
            (scalar_selectable.parent_entity_name, scalar_selectable.name),
            SelectionSet { selections: vec![] }
                .with_location(EmbeddedLocation::todo_generated())
                .interned_value(db)
                .scalar_selected()
                .wrap_ok(),
        );
    }

    if let Ok(outcome) = TCompilationProfile::deprecated_parse_type_system_documents(db) {
        let expose_fields = &outcome.0.item.client_scalar_refetch_strategies;

        // And we must also do it for expose fields. Ay ay ay
        for with_location in expose_fields.iter().flatten() {
            let (parent_object_entity_name, selectable_name, refetch_strategy) =
                &with_location.item;

            if let RefetchStrategy::UseRefetchField(refetch_strategy) = refetch_strategy {
                map.insert(
                    (*parent_object_entity_name, (*selectable_name)),
                    refetch_strategy
                        .refetch_selection_set
                        .reference()
                        .interned_ref(db)
                        .note_todo("This seems really wonky and wrong")
                        .scalar_selected()
                        .wrap_ok(),
                );
            }
        }
    }

    map
}

pub fn selectable_reader_selection_set<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_server_object_entity_name: EntityName,
    selectable_name: SelectableName,
) -> DiagnosticResult<MemoRef<WithEmbeddedLocation<SelectionSet>>> {
    let map = reader_selection_set_map(db);

    map.get(&(parent_server_object_entity_name, selectable_name))
        .unwrap_or_else(|| panic!("Expected selectable to have been defined. \
            This is indicative of a bug in Isograph. {parent_server_object_entity_name}.{selectable_name}"))
        .clone()
        .map(|x| x.inner())
        .note_todo("Do not clone. Use a MemoRef.")
}
