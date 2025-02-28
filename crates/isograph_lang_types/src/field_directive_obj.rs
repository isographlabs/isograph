use serde::Deserialize;

use crate::LoadableDirectiveParameters;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ScalarFieldValidParsedDirectives {
    #[serde(default)]
    pub loadable: Option<LoadableDirectiveParameters>,
    #[serde(default)]
    pub updatable: Option<Updatable>,
}

#[derive(Deserialize, Debug, Default)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct Updatable {}
