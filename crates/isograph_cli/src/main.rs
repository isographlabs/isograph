mod opt;

use clap::Parser;
use common_lang_types::CurrentWorkingDirectory;
use graphql_network_protocol::GraphQLAndJavascriptProfile;
use intern::string_key::Intern;
use isograph_compiler::{compile_and_print, handle_watch_command};
use isograph_config::{CompilerConfig, Kind, create_config};
use opentelemetry::{KeyValue, sdk::Resource};
use opentelemetry_otlp::WithExportConfig;
use opt::{Command, CompileCommand, LspCommand, Opt};
use sql_network_protocol::SQLAndJavascriptProfile;
use std::io;
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
    let config_location = compile_command
        .config
        .unwrap_or("./isograph.config.json".into());

    let config = create_config(&config_location, current_working_directory);

    configure_logger(compile_command.log_level, &config);
    if compile_command.watch {
        let result = match config.kind {
            Kind::GraphQL => {
                handle_watch_command::<GraphQLAndJavascriptProfile>(
                    config,
                    current_working_directory,
                )
                .await
            }
            Kind::SQL => {
                handle_watch_command::<SQLAndJavascriptProfile>(config, current_working_directory)
                    .await
            }
        };
        match result {
            Ok(_) => {
                info!("{}", "Successfully watched. Exiting.\n")
            }
            Err(err) => {
                error!("Error in watch process of some sort");
                for diagnostic in err {
                    error!("\n{}", diagnostic);
                }
                std::process::exit(1);
            }
        };
    } else {
        let result = match config.kind {
            Kind::GraphQL => {
                compile_and_print::<GraphQLAndJavascriptProfile>(config, current_working_directory)
            }
            Kind::SQL => {
                compile_and_print::<SQLAndJavascriptProfile>(config, current_working_directory)
            }
        };
        if result.is_err() {
            std::process::exit(1);
        }
    }
}

async fn start_language_server(
    lsp_command: LspCommand,
    current_working_directory: CurrentWorkingDirectory,
) {
    let config_location = lsp_command
        .config
        .unwrap_or("./isograph.config.json".into());

    let config = create_config(&config_location, current_working_directory);

    configure_logger(lsp_command.log_level, &config);
    let result = match config.kind {
        Kind::GraphQL => {
            isograph_lsp::start_language_server::<GraphQLAndJavascriptProfile>(
                config,
                current_working_directory,
            )
            .await
        }
        Kind::SQL => {
            isograph_lsp::start_language_server::<SQLAndJavascriptProfile>(
                config,
                current_working_directory,
            )
            .await
        }
    };
    if let Err(e) = result {
        // TODO use eprintln once we figure out how to make clippy not complain
        error!("Error(s) encountered when running language server.");
        for err in e {
            error!("\n{}", err);
        }

        std::process::exit(1);
    }
}

fn configure_logger(log_level: LevelFilter, config: &CompilerConfig) {
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

    if let Some(options) = &config.options.open_telemetry
        && options.enable_tracing
    {
        let tracer = opentelemetry_otlp::new_pipeline()
            .tracing()
            .with_exporter(
                opentelemetry_otlp::new_exporter()
                    .tonic()
                    .with_endpoint(options.collector_endpoint.clone()),
            )
            .with_trace_config(
                opentelemetry::sdk::trace::config().with_resource(Resource::new(vec![
                    KeyValue::new("service.name", options.service_name.clone()),
                ])),
            )
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
