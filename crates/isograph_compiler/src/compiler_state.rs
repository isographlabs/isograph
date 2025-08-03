use std::{
    ops::Deref,
    path::PathBuf,
    time::{Duration, Instant},
};

use common_lang_types::{CurrentWorkingDirectory, WithLocation};
use generate_artifacts::get_artifact_path_and_content;
use isograph_config::create_config;
use isograph_schema::{validate_use_of_arguments, NetworkProtocol, ValidateUseOfArgumentsError};
use pico::Database;
use thiserror::Error;

use crate::{
    batch_compile::CompilationStats,
    create_schema::{
        create_schema, process_iso_literals_for_schema, CreateSchemaError,
        ProcessIsoLiteralsForSchemaError,
    },
    db_singletons::get_isograph_config,
    source_files::{initialize_sources, SourceError},
    write_artifacts::{write_artifacts_to_disk, GenerateArtifactsError},
};

const GC_DURATION_SECONDS: u64 = 60;

#[derive(Debug)]
pub struct CompilerState {
    pub db: Database,
    pub last_gc_run: Instant,
}

impl CompilerState {
    pub fn new<TNetworkProtocol: NetworkProtocol + 'static>(
        config_location: &PathBuf,
        current_working_directory: CurrentWorkingDirectory,
    ) -> Result<Self, BatchCompileError<TNetworkProtocol>> {
        let mut db = Database::new();
        db.set(current_working_directory);
        db.set(create_config(config_location, current_working_directory));
        initialize_sources(&mut db)?;
        Ok(Self {
            db,
            last_gc_run: Instant::now(),
        })
    }

    pub fn run_garbage_collection(&mut self) {
        if self.last_gc_run.elapsed() >= Duration::from_secs(GC_DURATION_SECONDS) {
            self.db.run_garbage_collection();
            self.last_gc_run = Instant::now();
        }
    }
}

/// This the "workhorse" command of batch compilation.
///
/// ## Overall plan
///
/// When the compiler runs in batch mode, we must do the following things. This
/// description is a bit simplified.
///
/// - Read and parse things:
///   - Read and parse the GraphQL schema
///   - Read and parse the Isograph literals
/// - Combine everything into an Schema.
/// - Turn the Schema into a Schema
///   - Note: at this point, we do most of the validations, like ensuring that
///     all selected fields exist and are of the correct types, parameters are
///     passed when needed, etc.
/// - Generate an in-memory representation of all of the generated files
///   (called artifacts). This step should not fail. It should panic if any
///   invariant is violated, or represent that invariant in the type system.
/// - Delete and recreate the artifacts on disk.
///
/// ## Additional things we do
///
/// In addition to the things we do above, we also do some specific things like:
///
/// - if a client field is defined on an interface, add it to each concrete
///   type. So, if User implements Actor, you can define Actor.NameDisplay, and
///   select User.NameDisplay
/// - create fields from exposeAs directives
///
/// These are less "core" to the overall mission, and thus invite the question
/// of whether they belong in this function, or at all.
pub fn compile<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &Database,
) -> Result<CompilationStats, BatchCompileError<TNetworkProtocol>> {
    // Create schema
    let (unvalidated_isograph_schema, unprocessed_items) =
        create_schema::<TNetworkProtocol>(db).deref().clone()?;
    let (isograph_schema, stats) = process_iso_literals_for_schema::<TNetworkProtocol>(
        db,
        unvalidated_isograph_schema,
        unprocessed_items,
    )?;

    validate_use_of_arguments(&isograph_schema)?;

    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.

    let config = get_isograph_config(db);
    let artifacts = get_artifact_path_and_content(&isograph_schema, config);
    let total_artifacts_written =
        write_artifacts_to_disk(artifacts, &config.artifact_directory.absolute_path)?;

    Ok(CompilationStats {
        client_field_count: stats.client_field_count,
        entrypoint_count: stats.entrypoint_count,
        total_artifacts_written,
    })
}

#[derive(Error, Debug)]
pub enum BatchCompileError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error("{error}")]
    CreateSchemaError {
        #[from]
        error: CreateSchemaError<TNetworkProtocol>,
    },

    #[error("{error}")]
    ProcessIsoLiteralsForSchemaError {
        #[from]
        error: ProcessIsoLiteralsForSchemaError,
    },

    #[error("{error}")]
    SourceError {
        #[from]
        error: SourceError,
    },

    #[error("{error}")]
    GenerateArtifacts {
        #[from]
        error: GenerateArtifactsError,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{x}"));
            output
        })
    )]
    ValidateUseOfArguments {
        messages: Vec<WithLocation<ValidateUseOfArgumentsError>>,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{x}"));
            output
        })
    )]
    NotifyErrors { messages: Vec<notify::Error> },
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<Vec<WithLocation<ValidateUseOfArgumentsError>>> for BatchCompileError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<ValidateUseOfArgumentsError>>) -> Self {
        BatchCompileError::ValidateUseOfArguments { messages }
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> From<Vec<notify::Error>>
    for BatchCompileError<TNetworkProtocol>
{
    fn from(messages: Vec<notify::Error>) -> Self {
        BatchCompileError::NotifyErrors { messages }
    }
}
