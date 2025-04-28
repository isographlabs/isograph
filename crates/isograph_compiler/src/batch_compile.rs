use std::{path::PathBuf, str::Utf8Error};

use crate::{
    compiler_state::{compile, StandardSources},
    source_files::SourceFiles,
    with_duration::WithDuration,
};
use colored::Colorize;
use common_lang_types::{CurrentWorkingDirectory, WithLocation};
use isograph_lang_parser::IsographLiteralParseError;
use isograph_schema::{NetworkProtocol, ProcessClientFieldDeclarationError};
use pretty_duration::pretty_duration;
use thiserror::Error;
use tracing::{error, info};

use crate::compiler_state::CompilerState;

pub struct CompilationStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TNetworkProtocol: NetworkProtocol<Sources = StandardSources>>(
    config_location: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("{}", "Starting to compile.".cyan());
    print_result(WithDuration::new(|| {
        let mut state = CompilerState::new(config_location, current_working_directory);
        let sources = SourceFiles::read_all(&mut state.db, &state.config)?;
        compile::<TNetworkProtocol>(&state.db, &sources, &state.config)
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

    #[error("Schema file not found. Cannot proceed without a schema.")]
    SchemaNotFound,

    #[error("The project root at the following path: \"{path:?}\", is not a directory.")]
    ProjectRootNotADirectory { path: PathBuf },

    #[error("Unable to read the file at the following path: {path:?}.\nReason: {message}")]
    UnableToReadFile { path: PathBuf, message: String },

    #[error(
        "Attempted to load the schema at the following path: {path:?}, but that is not a file."
    )]
    SchemaNotAFile { path: PathBuf },

    #[error("Unable to traverse directory.\nReason: {message}")]
    UnableToTraverseDirectory { message: String },

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

    #[error("Error when doing additional schema processing.\nReason: {0}")]
    UnableToCreateSchema(#[from] WithLocation<isograph_schema::CreateAdditionalFieldsError>),

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

    #[error("Unable to strip prefix.\nReason: {0}")]
    UnableToStripPrefix(#[from] std::path::StripPrefixError),

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
    MultipleErrors {
        messages: Vec<Box<dyn std::error::Error>>,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    MultipleErrorsWithLocations {
        messages: Vec<WithLocation<Box<dyn std::error::Error>>>,
    },
}

impl From<Vec<WithLocation<IsographLiteralParseError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<IsographLiteralParseError>>) -> Self {
        BatchCompileError::UnableToParseIsographLiterals { messages }
    }
}

impl From<Vec<WithLocation<ProcessClientFieldDeclarationError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ProcessClientFieldDeclarationError>>) -> Self {
        BatchCompileError::ErrorWhenProcessingClientFieldDeclaration { messages }
    }
}
