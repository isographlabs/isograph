use serde::Deserialize;

use crate::LoadableDirectiveParameters;

#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatableDirectiveParameters {}

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ScalarFieldValidDirectiveSet {
    Loadable(LoadableDirectiveSet),
    Updatable(UpdatableDirectiveSet),
    None(EmptyDirectiveSet),
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct UpdatableDirectiveSet {
    pub updatable: UpdatableDirectiveParameters,
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LoadableDirectiveSet {
    pub loadable: LoadableDirectiveParameters,
}

// No directives -> an EmptyStruct is parsed!
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EmptyDirectiveSet {}
