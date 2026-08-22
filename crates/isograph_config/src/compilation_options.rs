use schemars::JsonSchema;
use serde::Deserialize;

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, JsonSchema, Debug, Default)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
}
