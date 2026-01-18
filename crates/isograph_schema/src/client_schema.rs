use pico_macros::memo;
use prelude::Postfix;

use crate::{
    CompilationProfile, FlattenedDataModelSchema, IsographDatabase, NestedDataModelSchema,
    flatten::Flatten,
};

#[memo]
pub fn flattened_client_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> FlattenedDataModelSchema<TCompilationProfile> {
    nested_client_schema(db)
        .item
        .iter()
        .map(|(key, value)| {
            (
                key.dereference(),
                value.clone().drop_location().map(|x| x.flatten()),
            )
        })
        .collect()
}

// TODO return a schema that is flattened, but contains locations
#[memo]
pub fn nested_client_schema<TCompilationProfile: CompilationProfile>(
    _db: &IsographDatabase<TCompilationProfile>,
) -> NestedDataModelSchema<TCompilationProfile> {
    Default::default()
}
