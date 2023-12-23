use std::path::PathBuf;

use isograph_schema::{CompilerConfig, ConfigOptions, OptionalValidationLevel};
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    /// The relative path to the folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The relative path to the folder where the compiler should create artifacts
    pub artifact_directory: PathBuf,
    /// The relative path to the GraphQL schema
    pub schema: PathBuf,
    /// The relative path to schema extensions
    pub schema_extensions: Vec<PathBuf>,

    /// Various that are of lesser importance
    #[serde(default = "Default::default")]
    pub options: ConfigFileOptions,
}

pub(crate) fn create_config(config_location: Option<&PathBuf>) -> CompilerConfig {
    let mut config_location = config_location
        .expect("--config must be provided for now.")
        .clone();
    let config_contents =
        std::fs::read_to_string(&config_location).expect("Expected config to be found");

    let config_parsed: ConfigFile = serde_json::from_str(&config_contents)
        .unwrap_or_else(|e| panic!("Error parsing config. Error: {}", e));

    config_location.pop();
    let config_dir = config_location;

    std::fs::create_dir_all(config_dir.join(&config_parsed.artifact_directory))
        .expect("Unable to create artifact directory");

    CompilerConfig {
        project_root: config_dir
            .join(&config_parsed.project_root)
            .canonicalize()
            .expect("Unable to canonicalize project root."),
        artifact_directory: config_dir
            .join(&config_parsed.artifact_directory)
            .canonicalize()
            .expect("Unable to canonicalize artifact directory."),
        schema: config_dir
            .join(&config_parsed.schema)
            .canonicalize()
            .expect("Unable to canonicalize schema path."),
        schema_extensions: config_parsed
            .schema_extensions
            .into_iter()
            .map(|schema_extensions| {
                config_dir
                    .join(schema_extensions)
                    .canonicalize()
                    .expect("Unable to canonicalize schema extension path.")
            })
            .collect(),
        options: create_options(config_parsed.options),
    }
}

#[derive(Deserialize, Default)]
#[serde(default)]
struct ConfigFileOptions {
    on_invalid_id_type: ConfigFileOptionalValidationLevel,
}

#[derive(Deserialize, Debug, Clone, Copy)]
#[serde(rename_all = "snake_case")]
enum ConfigFileOptionalValidationLevel {
    /// If this validation error is encountered, it will be ignored
    Ignore,
    /// If this validation error is encountered, a warning will be issued
    Warn,
    /// If this validation error is encountered, the compilation will fail
    Error,
}

impl Default for ConfigFileOptionalValidationLevel {
    fn default() -> Self {
        Self::Ignore
    }
}

fn create_options(options: ConfigFileOptions) -> ConfigOptions {
    ConfigOptions {
        on_invalid_id_type: create_optional_validation_level(options.on_invalid_id_type),
    }
}

fn create_optional_validation_level(
    optional_validation_level: ConfigFileOptionalValidationLevel,
) -> OptionalValidationLevel {
    match optional_validation_level {
        ConfigFileOptionalValidationLevel::Ignore => OptionalValidationLevel::Ignore,
        ConfigFileOptionalValidationLevel::Warn => OptionalValidationLevel::Warn,
        ConfigFileOptionalValidationLevel::Error => OptionalValidationLevel::Error,
    }
}
