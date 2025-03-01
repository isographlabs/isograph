use serde::Deserialize;

use crate::LoadableDirectiveParameters;

#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatableDirectiveParameters {}

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ScalarFieldSelectionVariant {
    Loadable(LoadableScalarSelectionVariant),
    Updatable(UpdatableScalarSelectionVariant),
    None(EmptyStruct),
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatableScalarSelectionVariant {
    pub updatable: UpdatableDirectiveParameters,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoadableScalarSelectionVariant {
    pub loadable: LoadableDirectiveParameters,
}

// No directives -> an EmptyStruct is parsed!
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EmptyStruct {}
