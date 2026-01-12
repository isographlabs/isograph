use common_lang_types::{
    AbsolutePathAndRelativePath, CurrentWorkingDirectory, Diagnostic, GeneratedFileHeader,
    PrintLocationFn, relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Intern;
use pico_macros::Singleton;
use schemars::JsonSchema;
use serde::Deserialize;
use std::path::PathBuf;
use tracing::warn;

pub static ISOGRAPH_FOLDER: &str = "__isograph";

/// This struct is the internal representation of the config. It
/// is a transformed version of IsographProjectConfig.
#[derive(Debug, Clone, Singleton, Eq, PartialEq)]
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
    /// Network protocol kind
    pub kind: Kind,

    /// Various options that are of lesser importance
    pub options: CompilerConfigOptions,
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct CompilerConfigOptions {
    pub on_invalid_id_type: OptionalValidationLevel,
    pub no_babel_transform: bool,
    pub include_file_extensions_in_import_statements: GenerateFileExtensionsOption,
    pub module: JavascriptModule,
    pub generated_file_header: Option<GeneratedFileHeader>,
    pub persisted_documents: Option<PersistedDocumentsOptions>,
    pub open_telemetry: Option<OpenTelemetryOptions>,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    #[default]
    GraphQL,
    SQL,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OptionalValidationLevel {
    /// If this validation error is encountered, it will be ignored
    #[default]
    Ignore,
    /// If this validation error is encountered, a warning will be issued
    Warn,
    /// If this validation error is encountered, the compilation will fail
    Error,
}

impl OptionalValidationLevel {
    pub fn on_failure<'a>(
        self,
        on_error: impl FnOnce() -> (Diagnostic, PrintLocationFn<'a>),
    ) -> Result<(), Diagnostic> {
        match self {
            OptionalValidationLevel::Ignore => Ok(()),
            OptionalValidationLevel::Warn => {
                let (warning, print_location) = on_error();
                let printable = warning.printable(print_location);
                warn!("{printable}");
                Ok(())
            }
            OptionalValidationLevel::Error => Err(on_error().0),
        }
    }
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum JavascriptModule {
    CommonJs,
    #[default]
    EsModule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PersistedDocumentsOptions {
    pub file: Option<PathBuf>,
    pub algorithm: PersistedDocumentsHashAlgorithm,
    pub include_extra_info: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpenTelemetryOptions {
    pub enable_tracing: bool,
    pub collector_endpoint: String,
    pub service_name: String,
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum PersistedDocumentsHashAlgorithm {
    Md5,
    #[default]
    Sha256,
}

/// This struct is deserialized from an isograph.config.json file.
#[derive(Deserialize, JsonSchema, Debug)]
#[serde(deny_unknown_fields)]
pub struct IsographProjectConfig {
    /// The user may hard-code the JSON Schema for their version of the config.
    #[serde(rename = "$schema")]
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
    /// Network protocol kind
    #[serde(default)]
    pub kind: Kind,

    /// Various options of less importance
    #[serde(default)]
    pub options: ConfigFileOptions,
}

pub fn create_config(
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> CompilerConfig {
    let config_contents = match std::fs::read_to_string(config_location) {
        Ok(contents) => contents,
        Err(_) => match config_location.to_str() {
            Some(loc) => {
                panic!("Expected config to be found at {loc}")
            }
            None => {
                panic!("Expected config to be found.")
            }
        },
    };

    let config_parsed: IsographProjectConfig = serde_json::from_str(&config_contents)
        .unwrap_or_else(|e| panic!("Error parsing config. Error: {e}"));

    let mut config_dir = config_location.clone();
    config_dir.pop();

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
            panic!("Unable to canonicalize config_file at {config_location:?}.")
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
                                "Unable to canonicalize schema extension path. \
                                Does {schema_extension:?} exist?"
                            )
                        }),
                )
            })
            .collect(),
        kind: config_parsed.kind,
        options: create_options(config_parsed.options),
    }
}

#[derive(Deserialize, Default, JsonSchema, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFileOptions {
    /// What the compiler should do if it encounters an id field whose
    /// type is not ID! or ID.
    on_invalid_id_type: ConfigFileOptionalValidationLevel,
    /// Set this to true if you don't have the babel transform enabled.
    no_babel_transform: bool,
    /// Should the compiler include file extensions in import statements in
    /// generated files? e.g. should it import ./param_type or ./param_type.ts?
    include_file_extensions_in_import_statements: bool,
    /// The babel plugin transforms isograph literals containing entrypoints
    /// into imports or requires of the generated entrypoint.ts file. Should
    /// it generate require calls or esmodule imports?
    pub module: ConfigFileJavascriptModule,
    /// A string to generate, in a comment, at the top of every generated file.
    generated_file_header: Option<String>,
    /// Persisted documents options
    persisted_documents: Option<ConfigFilePersistedDocumentsOptions>,
    /// OpenTelemetry tracing configuration
    open_telemetry: Option<ConfigFileOpenTelemetryOptions>,
}

#[derive(Deserialize, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ConfigFileOptionalValidationLevel {
    /// If this validation error is encountered, it will be ignored
    Ignore,
    /// If this validation error is encountered, a warning will be issued
    Warn,
    /// If this validation error is encountered, the compilation will fail
    #[default]
    Error,
}

#[derive(Deserialize, Default, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFileJavascriptModule {
    CommonJs,
    #[default]
    EsModule,
}

#[derive(Deserialize, Default, JsonSchema, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFilePersistedDocumentsOptions {
    /// The file name for the persisted documents
    /// Defaults to `persisted_documents.json`
    pub file: Option<PathBuf>,
    /// The hashing algorithm used to compute document hashes.
    pub algorithm: ConfigFilePersistedDocumentsHashAlgorithm,
    /// Include extra info to the operation text
    pub include_extra_info: bool,
}

#[derive(Deserialize, Default, JsonSchema, Debug)]
#[serde(default, deny_unknown_fields)]
pub struct ConfigFileOpenTelemetryOptions {
    /// Enable sending traces to a collector
    pub enable_tracing: bool,
    /// OTLP collector endpoint (e.g., http://localhost:4317)
    pub collector_endpoint: Option<String>,
    /// Service name for traces
    pub service_name: Option<String>,
}

#[derive(Deserialize, Default, Debug, Clone, Copy, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFilePersistedDocumentsHashAlgorithm {
    Md5,
    #[default]
    Sha256,
}

fn create_options(options: ConfigFileOptions) -> CompilerConfigOptions {
    if let Some(header) = options.generated_file_header.as_ref() {
        let line_count = header.lines().count();
        if line_count > 1 {
            panic!("config.options.generated_file_header should not be a multi-line string.")
        }
    }

    let generated_file_header = options.generated_file_header.map(|x| x.intern().into());

    CompilerConfigOptions {
        on_invalid_id_type: create_optional_validation_level(options.on_invalid_id_type),
        no_babel_transform: options.no_babel_transform,
        include_file_extensions_in_import_statements: create_generate_file_extensions(
            options.include_file_extensions_in_import_statements,
        ),
        module: create_module(options.module),
        generated_file_header,
        persisted_documents: create_persisted_documents(options.persisted_documents),
        open_telemetry: create_open_telemetry(options.open_telemetry),
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
    if optional_generate_file_extensions {
        GenerateFileExtensionsOption::IncludeExtensionsInFileImports
    } else {
        GenerateFileExtensionsOption::ExcludeExtensionsInFileImports
    }
}

fn create_module(module: ConfigFileJavascriptModule) -> JavascriptModule {
    match module {
        ConfigFileJavascriptModule::CommonJs => JavascriptModule::CommonJs,
        ConfigFileJavascriptModule::EsModule => JavascriptModule::EsModule,
    }
}

fn create_persisted_documents(
    persisted_documents: Option<ConfigFilePersistedDocumentsOptions>,
) -> Option<PersistedDocumentsOptions> {
    persisted_documents.map(|options| {
        let algorithm = match options.algorithm {
            ConfigFilePersistedDocumentsHashAlgorithm::Md5 => PersistedDocumentsHashAlgorithm::Md5,
            ConfigFilePersistedDocumentsHashAlgorithm::Sha256 => {
                PersistedDocumentsHashAlgorithm::Sha256
            }
        };
        PersistedDocumentsOptions {
            file: options.file,
            algorithm,
            include_extra_info: options.include_extra_info,
        }
    })
}

fn create_open_telemetry(
    open_telemetry: Option<ConfigFileOpenTelemetryOptions>,
) -> Option<OpenTelemetryOptions> {
    open_telemetry.map(|options| OpenTelemetryOptions {
        enable_tracing: options.enable_tracing,
        collector_endpoint: options
            .collector_endpoint
            .unwrap_or("http://localhost:4317".to_string()),
        service_name: options.service_name.unwrap_or("isograph_cli".to_string()),
    })
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
