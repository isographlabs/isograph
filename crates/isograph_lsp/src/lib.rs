use common_lang_types::{CurrentWorkingDirectory, Diagnostic, DiagnosticVecResult};
use isograph_config::CompilerConfig;
use isograph_schema::NetworkProtocol;
use lsp_server::Connection;

mod code_action;
mod commands;
mod completion;
mod diagnostic_notification;
mod document_highlight;
mod format;
mod goto_definition;
mod hover;
mod location_utils;
mod lsp_command_dispatch;
pub mod lsp_notification_dispatch;
mod lsp_request_dispatch;
pub mod lsp_runtime_error;
mod lsp_state;
mod semantic_tokens;
pub mod server;
pub mod text_document;
mod uri_file_path_ext;

pub async fn start_language_server<TNetworkProtocol: NetworkProtocol>(
    config: CompilerConfig,
    current_working_directory: CurrentWorkingDirectory,
) -> DiagnosticVecResult<()> {
    eprintln!("Starting language server");
    let (connection, io_handles) = Connection::stdio();
    let params = server::initialize(&connection)?;
    server::run::<TNetworkProtocol>(connection, config, params, current_working_directory).await?;
    io_handles
        .join()
        .map_err(|e| Diagnostic::from_error(e, None))?;
    Ok(())
}
