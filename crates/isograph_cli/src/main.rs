mod opt;

use clap::Parser;
use colored::Colorize;
use common_lang_types::CurrentWorkingDirectory;
use graphql_network_protocol::GraphQLNetworkProtocol;
use intern::string_key::Intern;
use isograph_compiler::{compile_and_print, handle_watch_command};
use isograph_config::create_config;
use opentelemetry::{KeyValue, sdk::Resource};
use opentelemetry_otlp::WithExportConfig;
use opt::{Command, CompileCommand, LspCommand, Opt};
use std::{io, path::PathBuf};
use tracing::{error, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, prelude::*};

#[tokio::main]
async fn main() {
    let opt = Opt::parse();
    let command = opt.command.unwrap_or(Command::Compile(opt.compile));

    match command {
        Command::Compile(compile_command) => {
            start_compiler(compile_command, current_working_directory()).await;
        }
        Command::Lsp(lsp_command) => {
            start_language_server(lsp_command, current_working_directory()).await;
        }
    }
}

async fn start_compiler(
    compile_command: CompileCommand,
    current_working_directory: CurrentWorkingDirectory,
) {
    eprintln!("about to run {:?}", &compile_command);
    let config_location = compile_command
        .config
        .unwrap_or("./isograph.config.json".into());
    configure_logger(
        compile_command.log_level,
        &config_location,
        current_working_directory,
    );
    if compile_command.watch {
        match handle_watch_command::<GraphQLNetworkProtocol>(
            &config_location,
            current_working_directory,
        )
        .await
        {
            Ok(_) => {
                info!("{}", "Successfully watched. Exiting.\n")
            }
            Err(err) => {
                error!("{}\n{:?}", "Error in watch process of some sort.\n", err);
                std::process::exit(1);
            }
        };
    } else if let Err(e) =
        compile_and_print::<GraphQLNetworkProtocol>(&config_location, current_working_directory)
    {
        eprintln!("exit 1");
        eprintln!("{:?}", e);
        std::process::exit(1);
    }
}

async fn start_language_server(
    lsp_command: LspCommand,
    current_working_directory: CurrentWorkingDirectory,
) {
    let config_location = lsp_command
        .config
        .unwrap_or("./isograph.config.json".into());
    configure_logger(
        lsp_command.log_level,
        &config_location,
        current_working_directory,
    );
    info!("Starting language server");
    if let Err(_e) = isograph_lsp::start_language_server::<GraphQLNetworkProtocol>(
        &config_location,
        current_working_directory,
    )
    .await
    {
        error!(
            "{}",
            "Error encountered when running language server.".bright_red(),
            // TODO derive Error and print e
        );
        std::process::exit(1);
    }
}

fn configure_logger(
    log_level: LevelFilter,
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) {
    let config = create_config(config_location, current_working_directory);

    let mut fmt_layer = tracing_subscriber::fmt::layer()
        .pretty()
        .without_time()
        .with_writer(io::stderr);

    if !matches!(log_level, LevelFilter::DEBUG | LevelFilter::TRACE) {
        fmt_layer = fmt_layer
            .with_file(false)
            .with_line_number(false)
            .with_target(false);
    }

    let fmt_layer =
        fmt_layer.with_filter(EnvFilter::from_default_env().add_directive(log_level.into()));

    if let Some(options) = config.options.open_telemetry
        && options.enable_tracing
    {
        let tracer =
            opentelemetry_otlp::new_pipeline()
                .tracing()
                .with_exporter(
                    opentelemetry_otlp::new_exporter()
                        .tonic()
                        .with_endpoint(options.collector_endpoint),
                )
                .with_trace_config(opentelemetry::sdk::trace::config().with_resource(
                    Resource::new(vec![KeyValue::new("service.name", options.service_name)]),
                ))
                .install_batch(opentelemetry::runtime::Tokio)
                .expect("Failed to install OTLP tracer");

        let otlp_layer = tracing_opentelemetry::layer().with_tracer(tracer);

        tracing_subscriber::registry()
            .with(fmt_layer)
            .with(otlp_layer)
            .init();
    } else {
        tracing_subscriber::registry().with(fmt_layer).init();
    }
}

fn current_working_directory() -> CurrentWorkingDirectory {
    let mut current_dir = std::env::current_dir().expect("Expected current working to exist");

    if cfg!(target_os = "windows") {
        current_dir = std::fs::canonicalize(current_dir)
            .expect("Expected current working directory to be able to be canonicalized");
    }

    current_dir
        .to_str()
        .expect("Expected current working directory to be able to be stringified.")
        .intern()
        .into()
}
