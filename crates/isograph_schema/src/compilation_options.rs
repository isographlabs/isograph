use std::{error::Error, path::PathBuf};

use colorize::AnsiColor;

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

impl OptionalValidationLevel {
    pub fn on_failure<E>(self, on_error: impl FnOnce() -> E) -> Result<(), E>
    where
        E: Error,
    {
        match self {
            OptionalValidationLevel::Ignore => Ok(()),
            OptionalValidationLevel::Warn => {
                let warning = on_error();
                // TODO pass to some sort of warning gatherer, this is weird!
                // The fact that we know about colorize here is weird!
                eprintln!("{}\n{}\n", "Warning:".yellow(), warning);
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
