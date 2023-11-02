mod artifact_file_contents;
mod batch_compile;
mod generate_artifacts;
mod isograph_literals;
mod schema;
mod write_artifacts;

use batch_compile::{handle_compile_command, BatchCompileCliOptions};
use colored::Colorize;
use structopt::StructOpt;

fn main() {
    let opt = BatchCompileCliOptions::from_args();
    let result = handle_compile_command(opt);

    match result {
        Ok(_) => eprintln!("{}", "Successfully compiled.\n".bright_green()),
        Err(err) => {
            eprintln!("{}\n{}", "Error when compiling.\n".bright_red(), err);
            std::process::exit(1);
        }
    }
}
