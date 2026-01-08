use std::collections::BTreeMap;

use pico_macros::memo;

use crate::{CompilationProfile, FlattenedDataModelSchema, IsographDatabase};

#[memo]
pub fn flattened_client_schema<TCompilationProfile: CompilationProfile>(
    _db: &IsographDatabase<TCompilationProfile>,
) -> FlattenedDataModelSchema<TCompilationProfile> {
    BTreeMap::new()
}
