mod opt;

use clap::Parser;
use colored::Colorize;
use isograph_compiler::{compile_and_print, handle_watch_command};
use isograph_config::create_config;
use opt::{Command, CompileCommand, LspCommand, Opt};
use std::io;
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::fmt::format::FmtSpan;

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
    configure_logger(compile_command.log_level);
    let config_location = compile_command
        .config
        .unwrap_or("./isograph.config.json".into());

    if compile_command.watch {
        match handle_watch_command(config_location).await {
            Ok(res) => match res {
                Ok(_) => {
                    info!("{}", "Successfully watched. Exiting.\n")
                }
                Err(err) => {
                    error!("{}\n{:?}", "Error in watch process of some sort.\n", err);
                    std::process::exit(1);
                }
            },
            Err(err) => {
                error!("{}\n{}", "Error in watch process of some sort.\n", err);
                std::process::exit(1);
            }
        };
    } else if compile_and_print(config_location).is_err() {
        std::process::exit(1);
    }
}

async fn start_language_server(lsp_command: LspCommand) {
    let config = create_config(
        lsp_command
            .config
            .unwrap_or("./isograph.config.json".into()),
    );
    info!("Starting language server");
    if let Err(_e) = isograph_lsp::start_language_server(config).await {
        error!(
            "{}",
            "Error encountered when running language server.".bright_red(),
            // TODO derive Error and print e
        );
        std::process::exit(1);
    }
}

fn configure_logger(log_level: LevelFilter) {
    let mut collector = tracing_subscriber::fmt()
        .pretty()
        .without_time()
        .with_max_level(log_level)
        .with_writer(io::stderr);
    match log_level {
        LevelFilter::DEBUG | LevelFilter::TRACE => {
            collector = collector.with_span_events(FmtSpan::FULL);
        }
        _ => {
            collector = collector
                .with_file(false)
                .with_line_number(false)
                .with_target(false);
        }
    }
    collector.init();
}
