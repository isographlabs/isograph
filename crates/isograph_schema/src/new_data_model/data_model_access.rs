use std::collections::BTreeMap;

use common_lang_types::{EmbeddedLocation, EntityName, SelectableName, WithNoLocation};
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelEntity, FlattenedDataModelSelectable, IsographDatabase,
    NestedDataModelEntity,
    flatten::{BothFlattenedResults, Flatten},
};

#[memo]
fn flattened_schema<TCompilationProfile: CompilationProfile>(
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
    flattened_schema(db)
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
) -> Vec<(
    EntityName,
    SelectableName,
    MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>,
)> {
    flattened_schema(db)
        .iter()
        .flat_map(|(entity_name, value)| {
            value.item.1.item.iter().map(|(selectable_name, value)| {
                (
                    entity_name.dereference(),
                    selectable_name.dereference(),
                    value.0.interned_ref(db),
                )
            })
        })
        .collect()
}

#[memo]
pub fn flattened_selectables_for_entity<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> Option<BTreeMap<SelectableName, MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>>> {
    let entity = flattened_schema(db).get(entity_name.reference())?;

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
    let entity = flattened_schema(db).get(&entity_name)?;
    let (selectable, _) = entity.item.1.item.get(selectable_name.reference())?;
    selectable.interned_ref(db).wrap_some()
}
