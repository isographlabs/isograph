use schemars::JsonSchema;
use serde::Deserialize;

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, JsonSchema, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
}
