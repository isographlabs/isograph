use std::path::PathBuf;

use crate::{
    compiler_state::{compile, CompilerState},
    source_files::initialize_sources,
    with_duration::WithDuration,
};
use colored::Colorize;
use common_lang_types::{CurrentWorkingDirectory, WithLocation};
use isograph_schema::NetworkProtocol;
use pretty_duration::pretty_duration;
use thiserror::Error;
use tracing::{error, info};

pub struct CompilationStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print<TNetworkProtocol: NetworkProtocol + 'static>(
    config_location: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(), Box<dyn std::error::Error>> {
    info!("{}", "Starting to compile.".cyan());
    print_result(WithDuration::new(|| {
        let mut state = CompilerState::new(config_location, current_working_directory);
        initialize_sources(&mut state.db)?;
        compile::<TNetworkProtocol>(&state.db)
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
    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{x}"));
            output
        })
    )]
    MultipleErrorsWithLocations {
        messages: Vec<WithLocation<Box<dyn std::error::Error>>>,
    },
}
