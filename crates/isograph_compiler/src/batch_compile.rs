use std::path::PathBuf;

use crate::{
    compiler_state::{compile, BatchCompileError, CompilerState},
    with_duration::WithDuration,
};
use colored::Colorize;
use common_lang_types::CurrentWorkingDirectory;
use isograph_schema::NetworkProtocol;
use pretty_duration::pretty_duration;
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
