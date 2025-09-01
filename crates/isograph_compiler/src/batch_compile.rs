use std::{ops::Deref, path::PathBuf};

use crate::{
    GetValidatedSchemaError, SourceError,
    compiler_state::CompilerState,
    get_validated_schema,
    with_duration::WithDuration,
    write_artifacts::{GenerateArtifactsError, write_artifacts_to_disk},
};
use colored::Colorize;
use common_lang_types::CurrentWorkingDirectory;
use generate_artifacts::get_artifact_path_and_content;
use isograph_schema::{IsographDatabase, NetworkProtocol};
use pretty_duration::pretty_duration;
use thiserror::Error;
use tracing::{error, info};

pub struct CompilationStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TNetworkProtocol: NetworkProtocol + 'static>(
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), BatchCompileError<TNetworkProtocol>> {
    info!("{}", "Starting to compile.".cyan());
    let state = CompilerState::new(config_location, current_working_directory)?;
    print_result(WithDuration::new(|| compile::<TNetworkProtocol>(&state.db)))
}

pub fn print_result<TNetworkProtocol: NetworkProtocol + 'static>(
    result: WithDuration<Result<CompilationStats, BatchCompileError<TNetworkProtocol>>>,
) -> Result<(), BatchCompileError<TNetworkProtocol>> {
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
#[tracing::instrument(skip(db))]
pub fn compile<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<CompilationStats, BatchCompileError<TNetworkProtocol>> {
    let validated_schema = get_validated_schema(db);
    let (isograph_schema, stats) = match validated_schema.deref() {
        Ok((schema, stats)) => (schema, stats),
        Err(e) => return Err(e.clone().into()),
    };

    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.

    let config = db.get_isograph_config();
    let artifacts = get_artifact_path_and_content(isograph_schema, config);
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
    NotifyErrors { messages: Vec<notify::Error> },

    #[error("{error}")]
    GetValidatedSchemaError {
        #[from]
        error: GetValidatedSchemaError<TNetworkProtocol>,
    },
}

impl<TNetworkProtocol: NetworkProtocol + 'static> From<Vec<notify::Error>>
    for BatchCompileError<TNetworkProtocol>
{
    fn from(messages: Vec<notify::Error>) -> Self {
        BatchCompileError::NotifyErrors { messages }
    }
}
