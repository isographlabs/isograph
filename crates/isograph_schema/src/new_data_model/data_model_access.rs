use std::collections::BTreeMap;

use common_lang_types::{EntityName, SelectableName, WithNoLocation};
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
    let schema = flattened_schema(db);
    schema
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
pub fn flattened_selectable_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
    selectable_name: SelectableName,
) -> Option<MemoRef<FlattenedDataModelSelectable<TCompilationProfile>>> {
    let entity = flattened_schema(db).get(&entity_name)?;
    let (selectable, _) = entity.item.1.item.get(selectable_name.reference())?;
    selectable.interned_ref(db).wrap_some()
}
