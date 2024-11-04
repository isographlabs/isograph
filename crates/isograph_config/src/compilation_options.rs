use std::path::PathBuf;

use serde::Deserialize;
use tracing::warn;

pub static ISOGRAPH_FOLDER: &str = "__isograph";

use std::error::Error;

#[derive(Debug, Clone)]
pub struct CompilerConfig {
    // The absolute path to the config file
    pub config_location: PathBuf,
    /// The folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The folder where the compiler should create artifacts
    pub artifact_directory: PathBuf,
    /// The absolute path to the GraphQL schema
    pub schema: PathBuf,
    /// The absolute path to the schema extensions
    pub schema_extensions: Vec<PathBuf>,

    /// Various options that are of lesser importance
    pub options: ConfigOptions,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ConfigOptions {
    pub on_invalid_id_type: OptionalValidationLevel,
    pub generate_file_extensions: OptionalGenerateFileExtensions,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum OptionalGenerateFileExtensions {
    Yes,
    #[default]
    No,
}

impl OptionalGenerateFileExtensions {
    pub fn ts(&self) -> &str {
        match self {
            OptionalGenerateFileExtensions::No => "",
            OptionalGenerateFileExtensions::Yes => ".ts",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum OptionalValidationLevel {
    /// If this validation error is encountered, it will be ignored
    Ignore,
    /// If this validation error is encountered, a warning will be issued
    Warn,
    /// If this validation error is encountered, the compilation will fail
    Error,
}

impl OptionalValidationLevel {
    pub fn on_failure<E>(self, on_error: impl FnOnce() -> E) -> Result<(), E>
    where
        E: Error,
    {
        match self {
            OptionalValidationLevel::Ignore => Ok(()),
            OptionalValidationLevel::Warn => {
                let warning = on_error();
                warn!("{warning}");
                Ok(())
            }
            OptionalValidationLevel::Error => Err(on_error()),
        }
    }
}

impl Default for OptionalValidationLevel {
    fn default() -> Self {
        Self::Ignore
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    /// The relative path to the folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The relative path to the folder where the compiler should create artifacts
    /// Defaults to the project_root directory.
    pub artifact_directory: Option<PathBuf>,
    /// The relative path to the GraphQL schema
    pub schema: PathBuf,
    /// The relative path to schema extensions
    #[serde(default)]
    pub schema_extensions: Vec<PathBuf>,

    /// Various that are of lesser importance
    #[serde(default = "Default::default")]
    pub options: ConfigFileOptions,
}

pub fn create_config(config_location: PathBuf) -> CompilerConfig {
    let config_contents = match std::fs::read_to_string(&config_location) {
        Ok(contents) => contents,
        Err(_) => match config_location.to_str() {
            Some(loc) => {
                panic!("Expected config to be found at {}", loc)
            }
            None => {
                panic!("Expected config to be found.")
            }
        },
    };

    let config_parsed: ConfigFile = serde_json::from_str(&config_contents)
        .unwrap_or_else(|e| panic!("Error parsing config. Error: {}", e));

    let mut config = config_location.clone();
    config.pop();
    let config_dir = config;

    let artifact_dir = config_dir
        .join(
            config_parsed
                .artifact_directory
                .as_ref()
                .unwrap_or(&config_parsed.project_root),
        )
        .join(ISOGRAPH_FOLDER);
    std::fs::create_dir_all(&artifact_dir).expect("Unable to create artifact directory");

    let project_root_dir = config_dir.join(&config_parsed.project_root);
    std::fs::create_dir_all(&project_root_dir).expect("Unable to create project root directory");

    CompilerConfig {
        config_location: config_location.canonicalize().unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize config_file at {:?}.",
                config_location
            )
        }),
        project_root: project_root_dir.canonicalize().unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize project root at {:?}.",
                config_parsed.project_root
            )
        }),
        artifact_directory: artifact_dir.canonicalize().unwrap_or_else(|_| {
            panic!(
                "Unable to canonicalize artifact directory at {:?}.",
                config_parsed.artifact_directory
            )
        }),
        schema: config_dir
            .join(&config_parsed.schema)
            .canonicalize()
            .unwrap_or_else(|_| {
                panic!(
                    "Unable to canonicalize schema path. Does {:?} exist?",
                    config_parsed.schema
                )
            }),
        schema_extensions: config_parsed
            .schema_extensions
            .into_iter()
            .map(|schema_extension| {
                config_dir
                    .join(&schema_extension)
                    .canonicalize()
                    .unwrap_or_else(|_| {
                        panic!(
                            "Unable to canonicalize schema extension path. Does {:?} exist?",
                            schema_extension
                        )
                    })
            })
            .collect(),
        options: create_options(config_parsed.options),
    }
}

#[derive(Deserialize, Default)]
#[serde(default, deny_unknown_fields)]
struct ConfigFileOptions {
    on_invalid_id_type: ConfigFileOptionalValidationLevel,
    include_file_extensions_in_import_statements: bool,
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
        Self::Error
    }
}

fn create_options(options: ConfigFileOptions) -> ConfigOptions {
    ConfigOptions {
        on_invalid_id_type: create_optional_validation_level(options.on_invalid_id_type),
        generate_file_extensions: create_generate_file_extensions(
            options.include_file_extensions_in_import_statements,
        ),
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

fn create_generate_file_extensions(
    optional_generate_file_extensions: bool,
) -> OptionalGenerateFileExtensions {
    match optional_generate_file_extensions {
        true => OptionalGenerateFileExtensions::Yes,
        false => OptionalGenerateFileExtensions::No,
    }
}
