use common_lang_types::{SelectableName, StringLiteralValue};

use serde::Deserialize;

use super::create_additional_fields_error::FieldMapItem;

// TODO move to graphql_network_protocol crate
#[derive(Deserialize, Eq, PartialEq, Debug, Hash, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExposeFieldDirective {
    // TODO make this a ScalarSelectableName
    #[serde(default)]
    #[serde(rename = "as")]
    pub expose_as: Option<SelectableName>,
    #[serde(default)]
    pub field_map: Vec<FieldMapItem>,
    pub field: StringLiteralValue,
}
