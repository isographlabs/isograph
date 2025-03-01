use serde::Deserialize;

use crate::LoadableDirectiveParameters;

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Updatable {}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ScalarFieldSelectionVariant {
    Loadable(LoadableScalarSelectionVariant),
    Updatable(UpdatableScalarSelectionVariant),
    None(EmptyStruct),
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatableScalarSelectionVariant {
    pub updatable: Updatable,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoadableScalarSelectionVariant {
    pub loadable: LoadableDirectiveParameters,
}

// No directives -> an EmptyStruct is parsed!
#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EmptyStruct {}
