mod artifact_file_contents;
mod batch_compile;
mod config;
mod generate_artifacts;
mod isograph_literals;
mod opt;
mod schema;
mod watch;
mod write_artifacts;

use batch_compile::handle_compile_command;
use colored::Colorize;
use opt::CliOptions;
use structopt::StructOpt;
use watch::handle_watch_command;

fn main() {
    let opt = CliOptions::from_args();

    if opt.watch {
        handle_watch_command(opt.compile_options);
    } else {
        match handle_compile_command(&opt.compile_options) {
            Ok(_) => eprintln!("{}", "Successfully compiled.\n".bright_green()),
            Err(err) => {
                eprintln!("{}\n{}", "Error when compiling.\n".bright_red(), err);
                std::process::exit(1);
            }
        }
    }
}
