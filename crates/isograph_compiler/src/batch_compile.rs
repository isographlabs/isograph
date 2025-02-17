use std::{path::PathBuf, str::Utf8Error};

use crate::{with_duration::WithDuration, write_artifacts::GenerateArtifactsError};
use colored::Colorize;
use common_lang_types::{CurrentWorkingDirectory, WithLocation};
use graphql_schema_parser::SchemaParseError;
use isograph_lang_parser::IsographLiteralParseError;
use isograph_schema::{OutputFormat, ProcessClientFieldDeclarationError, ValidateSchemaError};
use pretty_duration::pretty_duration;
use thiserror::Error;
use tracing::{error, info};

use crate::compiler_state::CompilerState;

pub struct CompilationStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TOutputFormat: OutputFormat>(
    config_location: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("{}", "Starting to compile.".cyan());
    print_result(WithDuration::new(|| {
        CompilerState::<TOutputFormat>::new(config_location, current_working_directory)
            .batch_compile()
    }))
}

pub fn print_result(
    result: WithDuration<Result<CompilationStats, Box<dyn std::error::Error>>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let elapsed_time = result.elapsed_time;
    match result.item {
        Ok(stats) => {
            info!(
                "{}",
                format!(
                    "Successfully compiled {} client fields and {} \
                        entrypoints, and wrote {} artifacts, in {}.",
                    stats.client_field_count,
                    stats.entrypoint_count,
                    stats.total_artifacts_written,
                    pretty_duration(&elapsed_time, None)
                )
            );
            Ok(())
        }
        Err(err) => {
            error!(
                "{}\n{}\n{}",
                "Error when compiling.\n".bright_red(),
                err,
                format!("Compilation took {}.", pretty_duration(&elapsed_time, None)).bright_red()
            );
            Err(err)
        }
    }
}

#[derive(Error, Debug)]
pub enum BatchCompileError {
    #[error("Unable to load schema file at path {path:?}.\nReason: {message}")]
    UnableToLoadSchema { path: PathBuf, message: String },

    #[error("Attempted to load the graphql schema at the following path: {path:?}, but that is not a file.")]
    SchemaNotAFile { path: PathBuf },

    #[error("Schema file not found. Cannot proceed without a schema.")]
    SchemaNotFound,

    #[error("The project root at the following path: \"{path:?}\", is not a directory.")]
    ProjectRootNotADirectory { path: PathBuf },

    #[error("Unable to read the file at the following path: {path:?}.\nReason: {message}")]
    UnableToReadFile { path: PathBuf, message: String },

    #[error("Unable to traverse directory.\nReason: {message}")]
    UnableToTraverseDirectory { message: String },

    #[error("Unable to parse schema.\n\n{0}")]
    UnableToParseSchema(#[from] WithLocation<SchemaParseError>),

    #[error(
        "{}{}",
        if messages.len() == 1 { "Unable to parse Isograph literal:" } else { "Unable to parse Isograph literals:" },
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    UnableToParseIsographLiterals {
        messages: Vec<WithLocation<IsographLiteralParseError>>,
    },

    #[error("Unable to create schema.\nReason: {0}")]
    UnableToCreateSchema(#[from] WithLocation<isograph_schema::ProcessTypeDefinitionError>),

    #[error(
        "{}{}",
        if messages.len() == 1 {
            "Error when processing a client field declaration:"
        } else {
            "Errors when processing client field declarations:"
        },
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    ErrorWhenProcessingClientFieldDeclaration {
        messages: Vec<WithLocation<isograph_schema::ProcessClientFieldDeclarationError>>,
    },

    #[error("Error when processing an entrypoint declaration.\nReason: {0}")]
    ErrorWhenProcessingEntrypointDeclaration(
        #[from] WithLocation<isograph_schema::ValidateEntrypointDeclarationError>,
    ),

    #[error("Unable to strip prefix.\nReason: {0}")]
    UnableToStripPrefix(#[from] std::path::StripPrefixError),

    #[error(
        "{} when validating schema, client fields and entrypoint declarations.{}",
        if messages.len() == 1 { "Error" } else { "Errors" },
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    UnableToValidateSchema {
        messages: Vec<WithLocation<isograph_schema::ValidateSchemaError>>,
    },

    #[error("Unable to print.\nReason: {0}")]
    UnableToPrint(#[from] GenerateArtifactsError),

    #[error("Unable to convert file {path:?} to utf8.\nDetailed reason: {reason}")]
    UnableToConvertToString { path: PathBuf, reason: Utf8Error },

    #[error("The __refetch field was already defined. Isograph creates it automatically; you cannot create it.")]
    DuplicateRefetchField,

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    MultipleErrors { messages: Vec<BatchCompileError> },
}

impl From<Vec<WithLocation<IsographLiteralParseError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<IsographLiteralParseError>>) -> Self {
        BatchCompileError::UnableToParseIsographLiterals { messages }
    }
}

impl From<Vec<WithLocation<ValidateSchemaError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ValidateSchemaError>>) -> Self {
        BatchCompileError::UnableToValidateSchema { messages }
    }
}

impl From<Vec<WithLocation<ProcessClientFieldDeclarationError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ProcessClientFieldDeclarationError>>) -> Self {
        BatchCompileError::ErrorWhenProcessingClientFieldDeclaration { messages }
    }
}
