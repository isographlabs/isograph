use common_lang_types::StringLiteralValue;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct CanonicalIdDirective {
    pub field_name: StringLiteralValue,
}
