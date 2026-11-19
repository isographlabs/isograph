use std::path::PathBuf;

use serde::Deserialize;

pub static ISOGRAPH_FOLDER: &str = "__isograph";

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    pub json_schema: Option<String>,
    /// Glob patterns relative to the config file's directory.
    pub source_files: Vec<String>,
    /// Glob patterns relative to the config file's directory.
    #[serde(default)]
    pub schema: Vec<String>,
    /// The directory, relative to the config file's directory, whose `__isograph` folder holds
    /// the generated artifacts. Absent: the compiler writes no artifacts, and the transforms
    /// look in the config file's directory.
    #[serde(default)]
    pub output_directory: Option<PathBuf>,
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use prelude::Postfix;

    use super::{ISOGRAPH_FOLDER, IsographProjectConfig};

    #[test]
    fn isograph_folder_is_the_generated_directory_name() {
        assert_eq!(ISOGRAPH_FOLDER, "__isograph");
    }

    #[test]
    fn a_config_without_schema_has_no_schema_globs() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":["src/**"]}"#).expect("the config is valid");
        assert_eq!(config.source_files, ["src/**"]);
        assert_eq!(config.schema, Vec::<String>::new());
    }

    #[test]
    fn a_config_with_schema_has_those_globs() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[],"schema":["schema/**","!schema/old/**"]}"#)
                .expect("the config is valid");
        assert_eq!(config.schema, ["schema/**", "!schema/old/**"]);
    }

    #[test]
    fn a_config_with_an_output_directory_has_it() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[],"output_directory":"src"}"#)
                .expect("the config is valid");
        assert_eq!(config.output_directory, PathBuf::from("src").wrap_some());
    }

    #[test]
    fn a_config_without_an_output_directory_has_none() {
        let config: IsographProjectConfig =
            serde_json::from_str(r#"{"source_files":[]}"#).expect("the config is valid");
        assert_eq!(config.output_directory, None);
    }

    #[test]
    fn a_config_with_an_unknown_key_is_an_error() {
        let error =
            serde_json::from_str::<IsographProjectConfig>(r#"{"source_files":[],"schemas":[]}"#)
                .expect_err("deny_unknown_fields rejects schemas");
        assert!(error.to_string().contains("schemas"), "{error}");
    }
}
