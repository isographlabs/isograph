use std::collections::BTreeMap;

use common_lang_types::{EntityName, WithNoLocation};
use pico::MemoRef;
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelEntity, IsographDatabase, NestedDataModelEntity,
    flatten::{BothFlattenedResults, Flatten},
};

#[memo]
fn flattened_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> BTreeMap<
    EntityName,
    WithNoLocation<BothFlattenedResults<NestedDataModelEntity<TCompilationProfile>>>,
> {
    let items = TCompilationProfile::parse_nested_data_model_schema(db)
        .item
        .to_owned()
        .into_iter()
        .map(|(key, value)| (key, value.drop_location().map(|x| x.flatten())))
        .collect();

    items
}

#[memo]
pub fn flattened_entity_named<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    entity_name: EntityName,
) -> Option<WithNoLocation<MemoRef<FlattenedDataModelEntity<TCompilationProfile>>>> {
    let item = flattened_schema(db).get(&entity_name)?;

    item.as_ref()
        // TODO have trait types include MemoRef, do not intern on the outside...?
        .map(|(flattened_entity, _)| flattened_entity.interned_ref(db))
        .wrap_some()
}
