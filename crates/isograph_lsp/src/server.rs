use crate::lsp_process_error::LSPProcessResult;
use crossbeam::channel::Receiver;
use crossbeam::select;
use isograph_config::CompilerConfig as Config;
use log::debug;
use lsp_server::Connection;
use lsp_server::Message;
use lsp_types::request::Request;
use lsp_types::CodeActionProviderCapability;
use lsp_types::CompletionOptions;
use lsp_types::HoverProviderCapability;
use lsp_types::InitializeParams;
use lsp_types::SemanticTokensFullOptions;
use lsp_types::SemanticTokensLegend;
use lsp_types::SemanticTokensOptions;
use lsp_types::SemanticTokensServerCapabilities;
use lsp_types::ServerCapabilities;
use lsp_types::TextDocumentSyncCapability;
use lsp_types::TextDocumentSyncKind;
use lsp_types::WorkDoneProgressOptions;
use std::sync::Arc;

/// Initializes an LSP connection, handling the `initialize` message and `initialized` notification
/// handshake.
pub fn initialize(connection: &Connection) -> LSPProcessResult<InitializeParams> {
    let server_capabilities = ServerCapabilities {
        // Enable text document syncing so we can know when files are opened/changed/saved/closed
        text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: SemanticTokensLegend::default(),
                range: None,
                full: Some(SemanticTokensFullOptions::Bool(true)),
            },
        )),
        ..Default::default()
    };
    let server_capabilities = serde_json::to_value(server_capabilities)?;
    let params = connection.initialize(server_capabilities)?;
    let params: InitializeParams = serde_json::from_value(params)?;
    Ok(params)
}

/// Run the main server loop
pub async fn run(
    connection: Connection,
    mut config: Config,
    _params: InitializeParams,
) -> LSPProcessResult<()> {
    eprintln!("Running server loop");
    while let Some(message) = connection.receiver.recv().ok() {
        eprintln!("Received message: {:?}", message);
    }

    panic!("Client exited without proper shutdown sequence.")
}
