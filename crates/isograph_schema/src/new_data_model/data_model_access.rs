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
) -> Option<MemoRef<FlattenedDataModelEntity<TCompilationProfile>>> {
    let entity = flattened_schema(db).get(&entity_name)?;

    entity.item.0.interned_ref(db).wrap_some()
}
