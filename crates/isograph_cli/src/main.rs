mod artifact_file_contents;
mod batch_compile;
mod generate_artifacts;
mod isograph_literals;
mod opt;
mod schema;
mod watch;
mod write_artifacts;

use batch_compile::compile_and_print;
use colored::Colorize;
use isograph_config::create_config;
use opt::CliOptions;
use structopt::StructOpt;
use watch::handle_watch_command;

#[tokio::main]
async fn main() {
    let opt = CliOptions::from_args();
    let config = create_config(opt.config.unwrap_or("./isograph.config.json".into()));

    if opt.watch {
        match handle_watch_command(config).await {
            Ok(res) => match res {
                Ok(_) => {
                    eprintln!("{}", "Successfully watched. Exiting.\n".bright_green())
                }
                Err(err) => {
                    eprintln!(
                        "{}\n{:?}",
                        "Error in watch process of some sort.\n".bright_red(),
                        err
                    );
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!(
                    "{}\n{}",
                    "Error in watch process of some sort.\n".bright_red(),
                    err
                );
                std::process::exit(1);
            }
        };
    } else {
        if let Err(_) = compile_and_print(&config) {
            std::process::exit(1);
        }
    }
}
