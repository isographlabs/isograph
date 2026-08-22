use std::process::ExitCode;

use crate::ConfigFlag;
use crate::discover;

#[expect(clippy::print_stdout, clippy::print_stderr)]
pub fn run(id: &ConfigFlag) -> ExitCode {
    match discover::config_path(id.config.as_deref()) {
        Ok(path) => {
            println!("{}", path.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}
