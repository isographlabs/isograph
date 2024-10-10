mod opt;

use clap::Parser;
use colored::Colorize;
use isograph_compiler::{compile_and_print, handle_watch_command};
use isograph_config::create_config;
use opt::{Command, CompileCommand, LspCommand, Opt};

#[tokio::main]
async fn main() {
    let opt = Opt::parse();
    let command = opt.command.unwrap_or(Command::Compile(opt.compile));
    match command {
        Command::Compile(compile_command) => {
            start_compiler(compile_command).await;
        }
        Command::Lsp(lsp_command) => {
            start_language_server(lsp_command).await;
        }
    }
}

async fn start_compiler(compile_command: CompileCommand) {
    let config = create_config(
        compile_command
            .config
            .unwrap_or("./isograph.config.json".into()),
    );

    if compile_command.watch {
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

async fn start_language_server(lsp_command: LspCommand) {
    let config = create_config(
        lsp_command
            .config
            .unwrap_or("./isograph.config.json".into()),
    );
    eprintln!("Starting language server");
    if let Err(_e) = isograph_lsp::start_language_server(config).await {
        eprintln!(
            "{}",
            "Error encountered when running language server.".bright_red(),
            // TODO derive Error and print e
        );
        std::process::exit(1);
    }
}
