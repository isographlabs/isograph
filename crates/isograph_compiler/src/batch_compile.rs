use std::{path::PathBuf, time::Duration};

use crate::{
    SourceError,
    compiler_state::CompilerState,
    with_duration::WithDuration,
    write_artifacts::{GenerateArtifactsError, write_artifacts_to_disk},
};
use artifact_content::{
    generate_artifacts::GetArtifactPathAndContentError, get_artifact_path_and_content,
};
use colored::Colorize;
use common_lang_types::CurrentWorkingDirectory;
use isograph_schema::{
    IsographDatabase, NetworkProtocol, fetchable_types, fetchable_types_2, server_entity_map,
};
use pretty_duration::pretty_duration;
use thiserror::Error;
use tracing::{error, info};

pub struct CompilationStats {
    pub client_field_count: usize,
    pub client_pointer_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TNetworkProtocol: NetworkProtocol>(
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), BatchCompileError<TNetworkProtocol>> {
    info!("{}", "Starting to compile.".cyan());
    let state = CompilerState::new(config_location, current_working_directory)?;
    print_result(WithDuration::new(|| compile::<TNetworkProtocol>(&state.db)))
}

pub fn print_result<TNetworkProtocol: NetworkProtocol>(
    result: WithDuration<Result<CompilationStats, BatchCompileError<TNetworkProtocol>>>,
) -> Result<(), BatchCompileError<TNetworkProtocol>> {
    match result.item {
        Ok(stats) => {
            print_stats(result.elapsed_time, stats);
            Ok(())
        }
        Err(err) => {
            error!(
                "{}\n{}\n{}",
                "Error when compiling.\n".bright_red(),
                err,
                format!(
                    "Compilation took {}.",
                    pretty_duration(&result.elapsed_time, None)
                )
                .bright_red()
            );
            Err(err)
        }
    }
}

fn print_stats(elapsed_time: Duration, stats: CompilationStats) {
    let s_if_plural = |count: usize| {
        if count == 1 { "" } else { "s" }
    };

    info!(
        "Successfully compiled {} client field{}, {} client pointer{}, {} \
        entrypoint{}, and wrote {} artifact{}, in {}.",
        stats.client_field_count,
        s_if_plural(stats.client_field_count),
        stats.client_pointer_count,
        s_if_plural(stats.client_pointer_count),
        stats.entrypoint_count,
        s_if_plural(stats.entrypoint_count),
        stats.total_artifacts_written,
        s_if_plural(stats.total_artifacts_written),
        pretty_duration(&elapsed_time, None)
    );
}

/// This the "workhorse" command of batch compilation.
#[tracing::instrument(skip(db))]
pub fn compile<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<CompilationStats, BatchCompileError<TNetworkProtocol>> {
    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.

    // let fetch_types_2 = fetchable_types_2(db).as_ref().expect(
    //     "Expected parsing to have succeeded. \
    //         This is indicative of a bug in Isograph.",
    // );

    // eprintln!("owned fetch type len={}", fetch_types_2.len());

    let fetch_types = fetchable_types(db)
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
            This is indicative of a bug in Isograph.",
        )
        .lookup_tracked(db);
    eprintln!("entrypoint fetch type len={}", fetch_types.len());

    if fetch_types.len() == 0 {
        panic!("should not be empty");
    }
    // let config = db.get_isograph_config();
    // let (artifacts, stats) = get_artifact_path_and_content(db)?;

    Ok(CompilationStats {
        client_field_count: 0,
        client_pointer_count: 0,
        entrypoint_count: 0,
        total_artifacts_written: 100,
    })

    // Ok(CompilationStats {
    //     client_field_count: stats.client_field_count,
    //     client_pointer_count: stats.client_pointer_count,
    //     entrypoint_count: stats.entrypoint_count,
    //     total_artifacts_written,
    // })
}

#[derive(Error, Debug)]
pub enum BatchCompileError<TNetworkProtocol: NetworkProtocol> {
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
    GenerateArtifactsError {
        #[from]
        error: GetArtifactPathAndContentError<TNetworkProtocol>,
    },
}

impl<TNetworkProtocol: NetworkProtocol> From<Vec<notify::Error>>
    for BatchCompileError<TNetworkProtocol>
{
    fn from(messages: Vec<notify::Error>) -> Self {
        BatchCompileError::NotifyErrors { messages }
    }
}
