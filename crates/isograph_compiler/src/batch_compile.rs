use std::time::Duration;

use crate::{
    compiler_state::CompilerState,
    with_duration::WithDuration,
    write_artifacts::{apply_file_system_operations, get_file_system_operations},
};
use artifact_content::get_artifact_path_and_content;
use colored::Colorize;
use common_lang_types::{CurrentWorkingDirectory, Diagnostic, DiagnosticVecResult};
use isograph_config::CompilerConfig;
use isograph_schema::NetworkProtocol;
use prelude::Postfix;
use pretty_duration::pretty_duration;
use tracing::{error, info};

pub struct CompilationStats {
    pub client_field_count: usize,
    pub client_pointer_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TNetworkProtocol: NetworkProtocol>(
    config: CompilerConfig,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), ()> {
    info!("{}", "Starting to compile.".cyan());
    let mut state = match CompilerState::new(config, current_working_directory) {
        Ok(s) => s,
        Err(e) => {
            error!("{}", e);
            return ().wrap_err();
        }
    };
    print_result(WithDuration::new(|| {
        compile::<TNetworkProtocol>(&mut state)
    }))
}

pub fn print_result(result: WithDuration<DiagnosticVecResult<CompilationStats>>) -> Result<(), ()> {
    match result.item {
        Ok(stats) => {
            print_stats(result.elapsed_time, stats);
            ().wrap_ok()
        }
        Err(err) => {
            error!(
                "{}\n{}\n{}",
                "Error when compiling.\n".bright_red(),
                // TODO don't materialize a vec here
                err.iter()
                    .map(|e| e.to_string())
                    .collect::<Vec<_>>()
                    .join("\n"),
                format!(
                    "Compilation took {}.",
                    pretty_duration(&result.elapsed_time, None)
                )
                .bright_red()
            );
            ().wrap_err()
        }
    }
}

fn print_stats(elapsed_time: Duration, stats: CompilationStats) {
    let s_if_plural = |count: usize| {
        if count == 1 { "" } else { "s" }
    };

    info!(
        "Success! Compiled {} client field{}, {} client pointer{} and {} \
        entrypoint{}, and wrote or modified {} file{}, in {}.",
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
#[tracing::instrument(skip_all)]
pub fn compile<TNetworkProtocol: NetworkProtocol>(
    state: &mut CompilerState<TNetworkProtocol>,
) -> DiagnosticVecResult<CompilationStats> {
    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.
    let db = &state.db;
    let config = db.get_isograph_config();
    let (artifacts, stats) = get_artifact_path_and_content(db)?;

    let file_system_operations = get_file_system_operations(
        &artifacts,
        &config.artifact_directory.absolute_path,
        &mut state.file_system_state,
    );

    let total_artifacts_written = apply_file_system_operations(&file_system_operations, &artifacts)
        .map_err(Diagnostic::from)?;

    CompilationStats {
        client_field_count: stats.client_field_count,
        client_pointer_count: stats.client_pointer_count,
        entrypoint_count: stats.entrypoint_count,
        total_artifacts_written,
    }
    .wrap_ok()
}
