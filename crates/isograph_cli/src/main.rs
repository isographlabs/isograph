mod opt;

use colored::Colorize;
use isograph_compiler::compile_and_print;
use isograph_compiler::handle_watch_command;
use isograph_config::create_config;
use opt::CliOptions;
use structopt::StructOpt;

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
    } else if compile_and_print(&config).is_err() {
        std::process::exit(1);
    }
}
