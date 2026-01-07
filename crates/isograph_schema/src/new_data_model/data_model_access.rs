use std::collections::BTreeMap;

use common_lang_types::{EmbeddedLocation, EntityName, SelectableName, WithNoLocation};
use isograph_lang_types::SelectionType;
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelEntity, FlattenedDataModelSelectable, IsographDatabase,
    NestedDataModelEntity, client_selectable_declaration,
    flatten::{BothFlattenedResults, Flatten},
};

#[memo]
fn flattened_server_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> BTreeMap<
    EntityName,
    WithNoLocation<BothFlattenedResults<NestedDataModelEntity<TCompilationProfile>>>,
> {
    #[expect(clippy::unnecessary_to_owned)]
    TCompilationProfile::parse_nested_data_model_schema(db)
        .item
        .to_owned()
        .into_iter()
        .map(|(key, value)| (key, value.drop_location().map(|x| x.flatten())))
        .collect()
}

#[memo]
pub fn flattened_entities<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> BTreeMap<EntityName, MemoRef<FlattenedDataModelEntity<TCompilationProfile>>> {
    flattened_server_schema(db)
        .iter()
        .map(|(key, value)| (*key, value.item.0.interned_ref(db)))
        .collect()
}

#[memo]
pub fn flattened_entity_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> Option<MemoRef<FlattenedDataModelEntity<TCompilationProfile>>> {
    flattened_entities(db)
        .get(&entity_name)
        .map(|x| x.dereference())
}

#[memo]
pub fn flattened_server_object_entities<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<MemoRef<FlattenedDataModelEntity<TCompilationProfile>>> {
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
pub fn entity_definition_location<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> Option<Option<EmbeddedLocation>> {
    TCompilationProfile::parse_nested_data_model_schema(db)
        .item
        .get(entity_name.reference())
        .map(|x| x.location)
}

#[memo]
pub fn flattened_selectables<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> Vec<MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>> {
    flattened_server_schema(db)
        .iter()
        .flat_map(|(_entity_name, value)| {
            value
                .item
                .1
                .item
                .values()
                .map(|value| value.0.interned_ref(db))
        })
        .collect()
}

#[memo]
pub fn flattened_selectables_for_entity<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> Option<BTreeMap<SelectableName, MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>>> {
    let entity = flattened_server_schema(db).get(entity_name.reference())?;

    entity
        .item
        .1
        .item
        .iter()
        .map(|(key, value)| (key.dereference(), value.0.interned_ref(db)))
        .collect::<BTreeMap<_, _>>()
        .wrap_some()
}

#[memo]
pub fn flattened_selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Option<MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>> {
    let entity = flattened_server_schema(db).get(&entity_name)?;
    let (selectable, _) = entity.item.1.item.get(selectable_name.reference())?;
    selectable.interned_ref(db).wrap_some()
}

// TODO this is hacky AF, clean it up. It also returns the wrong location,
// or goto def uses it incorrectly, in that it doubly offsets the iso literal location
// e.g. if the iso literal is on line 4, the resulting goto def will be line 8
#[memo]
pub fn selectable_definition_location<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Option<EmbeddedLocation> {
    match TCompilationProfile::parse_nested_data_model_schema(db)
        .item
        .get(entity_name.reference())
        .and_then(|x| {
            x.item
                .selectables
                .item
                .get(selectable_name.reference())
                // TODO the selectable should have a location
                .map(|x| x.name.location)
        }) {
        Some(Some(s)) => return s.wrap_some(),
        _ => {}
    };

    let client_selectable_declaration =
        client_selectable_declaration(db, entity_name, selectable_name);

    client_selectable_declaration.map(|x| match x {
        SelectionType::Scalar(s) => s.lookup(db).client_field_name.location,
        SelectionType::Object(o) => o.lookup(db).client_pointer_name.location,
    })
}
