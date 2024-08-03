mod opt;

use colored::Colorize;
use isograph_compiler::compile_and_print;
use isograph_compiler::handle_watch_command;
use isograph_config::create_config;
use isograph_lsp::lsp_process_error::LSPProcessError;
use isograph_lsp::server;
use opt::Command;
use opt::CompileCommand;
use opt::LSPCommand;
use opt::Opt;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();
    let command = opt.command.unwrap_or_else(|| {
        Command::Compile(
            opt.compile
                .expect("Command not provided and compile options not set"),
        )
    });
    match command {
        Command::Compile(compile_command) => {
            start_compiler(compile_command).await;
        }
        Command::LSP(lsp_command) => {
            start_language_server(lsp_command).await.unwrap();
        }
    }
}

async fn start_compiler(compile_command: CompileCommand) {
    let config = create_config(compile_command.config.unwrap_or("./isograph.config.json".into()));

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
async fn start_language_server(lsp_command: LSPCommand) -> Result<(), LSPProcessError> {
    let config = create_config(
        lsp_command
            .config
            .unwrap_or("./isograph.config.json".into()),
    );
    eprintln!("Starting language server");
    isograph_lsp::start_language_server(config).await?;
    Ok(())
}
