mod opt;

use colored::Colorize;
use isograph_compiler::compile_and_print;
use isograph_compiler::handle_watch_command;
use isograph_config::create_config;
use isograph_lsp::lsp_process_error::LSPProcessError;
use isograph_lsp::server;
use opt::CliOptions;
use opt::CompilerCLIOptions;
use opt::LSPCLIOptions;
use structopt::StructOpt;

#[tokio::main]
async fn main() {
    let opt = CliOptions::from_args();
    match opt {
        CliOptions::Compile(compiler_options) => {
            start_compiler(compiler_options).await;
        }
        CliOptions::LSP(lsp_options) => {
            start_language_server(lsp_options).await.unwrap();
        }
    }
}

async fn start_compiler(opt: CompilerCLIOptions) {
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
async fn start_language_server(lsp_options: LSPCLIOptions) -> Result<(), LSPProcessError> {
    let config = create_config(
        lsp_options
            .config
            .unwrap_or("./isograph.config.json".into()),
    );
    eprintln!("Starting language server");
    isograph_lsp::start_language_server(config).await?;
    Ok(())
}
