use std::path::PathBuf;

#[derive(Debug)]
pub struct CompilerConfig {
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

impl Default for OptionalValidationLevel {
    fn default() -> Self {
        Self::Ignore
    }
}
