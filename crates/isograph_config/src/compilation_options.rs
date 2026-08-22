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

#[cfg(test)]
mod tests {
    use super::IsographProjectConfig;

    #[test]
    fn empty_object_does_not_deserialize() {
        serde_json::from_str::<IsographProjectConfig>("{}").expect_err("source_files is required");
    }

    #[test]
    fn empty_source_files_deserializes() {
        let config: IsographProjectConfig =
            serde_json::from_str("{\"source_files\":[]}\n").expect("empty list is a config");
        assert!(config.source_files.is_empty());
        assert_eq!(config.json_schema, None);
    }

    #[test]
    fn source_files_deserializes_in_order() {
        let config: IsographProjectConfig =
            serde_json::from_str("{\"source_files\":[\"src/**/*.ts\",\"!src/**/*.test.ts\"]}\n")
                .expect("two globs is a config");
        assert_eq!(
            config.source_files,
            ["src/**/*.ts".to_owned(), "!src/**/*.test.ts".to_owned()]
        );
    }

    #[test]
    fn omitted_source_files_with_schema_does_not_deserialize() {
        serde_json::from_str::<IsographProjectConfig>("{\"$schema\":\"x\"}\n")
            .expect_err("source_files is required");
    }

    #[test]
    fn source_files_null_does_not_deserialize() {
        serde_json::from_str::<IsographProjectConfig>("{\"source_files\":null}\n")
            .expect_err("source_files is a list");
    }
}
