use std::path::PathBuf;

use common_lang_types::{
    relative_path_from_absolute_and_working_directory, CurrentWorkingDirectory,
    RelativePathToSourceFile,
};
use schemars::JsonSchema;
use serde::Deserialize;
use tracing::warn;

pub static ISOGRAPH_FOLDER: &str = "__isograph";

use std::error::Error;

#[derive(Debug, Clone)]
pub struct AbsolutePathAndRelativePath {
    pub absolute_path: PathBuf,
    pub relative_path: RelativePathToSourceFile,
}

/// This struct is the internal representation of the schema. It
/// is a transformed version of IsographProjectConfig.
#[derive(Debug, Clone)]
pub struct CompilerConfig {
    // The absolute path to the config file
    pub config_location: PathBuf,
    /// The folder where the compiler should look for Isograph literals
    pub project_root: PathBuf,
    /// The folder where the compiler should create artifacts
    pub artifact_directory: AbsolutePathAndRelativePath,
    /// The absolute path to the GraphQL schema
    pub schema: AbsolutePathAndRelativePath,
    /// The absolute path to the schema extensions
    pub schema_extensions: Vec<AbsolutePathAndRelativePath>,

    /// Various options that are of lesser importance
    pub options: CompilerConfigOptions,

    pub current_working_directory: CurrentWorkingDirectory,
}

#[derive(Default, Debug, Clone)]
pub struct CompilerConfigOptions {
    pub on_invalid_id_type: OptionalValidationLevel,
    pub no_babel_transform: bool,
    pub include_file_extensions_in_import_statements: GenerateFileExtensionsOption,
    pub module: JavascriptModule,
}

#[derive(Default, Debug, Clone, Copy)]
pub enum GenerateFileExtensionsOption {
    IncludeExtensionsInFileImports,
    #[default]
    ExcludeExtensionsInFileImports,
}

impl GenerateFileExtensionsOption {
    pub fn ts(&self) -> &str {
        match self {
            GenerateFileExtensionsOption::ExcludeExtensionsInFileImports => "",
            GenerateFileExtensionsOption::IncludeExtensionsInFileImports => ".ts",
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

#[derive(Default, Debug, Clone, Copy)]
pub enum JavascriptModule {
    CommonJs,
    #[default]
    EsModule,
}

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
    #[allow(dead_code)]
    pub json_schema: Option<String>,
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

    /// Various options of less importance
    #[serde(default)]
    pub options: ConfigFileOptions,
}

pub fn create_config(
    config_location: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> CompilerConfig {
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

    let config_parsed: IsographProjectConfig = serde_json::from_str(&config_contents)
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
        artifact_directory: absolute_and_relative_paths(
            current_working_directory,
            artifact_dir.canonicalize().unwrap_or_else(|_| {
                panic!(
                    "Unable to canonicalize artifact directory at {:?}.",
                    config_parsed.artifact_directory
                )
            }),
        ),
        schema: absolute_and_relative_paths(
            current_working_directory,
            config_dir
                .join(&config_parsed.schema)
                .canonicalize()
                .unwrap_or_else(|_| {
                    panic!(
                        "Unable to canonicalize schema path. Does {:?} exist?",
                        config_parsed.schema
                    )
                }),
        ),
        schema_extensions: config_parsed
            .schema_extensions
            .into_iter()
            .map(|schema_extension| {
                absolute_and_relative_paths(
                    current_working_directory,
                    config_dir
                        .join(&schema_extension)
                        .canonicalize()
                        .unwrap_or_else(|_| {
                            panic!(
                                "Unable to canonicalize schema extension path. Does {:?} exist?",
                                schema_extension
                            )
                        }),
                )
            })
            .collect(),
        options: create_options(config_parsed.options),

        current_working_directory,
    }
}

#[derive(Deserialize, Default, JsonSchema)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFileOptions {
    on_invalid_id_type: ConfigFileOptionalValidationLevel,
    no_babel_transform: bool,
    include_file_extensions_in_import_statements: bool,
    module: ConfigFileJavascriptModule,
}

#[derive(Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ConfigFileOptionalValidationLevel {
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

#[derive(Deserialize, Default, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFileJavascriptModule {
    CommonJs,
    #[default]
    EsModule,
}

fn create_options(options: ConfigFileOptions) -> CompilerConfigOptions {
    CompilerConfigOptions {
        on_invalid_id_type: create_optional_validation_level(options.on_invalid_id_type),
        no_babel_transform: options.no_babel_transform,
        include_file_extensions_in_import_statements: create_generate_file_extensions(
            options.include_file_extensions_in_import_statements,
        ),
        module: create_module(options.module),
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
) -> GenerateFileExtensionsOption {
    match optional_generate_file_extensions {
        true => GenerateFileExtensionsOption::IncludeExtensionsInFileImports,
        false => GenerateFileExtensionsOption::ExcludeExtensionsInFileImports,
    }
}

fn create_module(module: ConfigFileJavascriptModule) -> JavascriptModule {
    match module {
        ConfigFileJavascriptModule::CommonJs => JavascriptModule::CommonJs,
        ConfigFileJavascriptModule::EsModule => JavascriptModule::EsModule,
    }
}

pub fn absolute_and_relative_paths(
    current_working_directory: CurrentWorkingDirectory,
    absolute_path: PathBuf,
) -> AbsolutePathAndRelativePath {
    let relative_path = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &absolute_path,
    );

    AbsolutePathAndRelativePath {
        absolute_path,
        relative_path,
    }
}
