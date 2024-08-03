use crate::lsp_process_error::LSPProcessResult;
use isograph_config::CompilerConfig as Config;
use lsp_server::Connection;
use lsp_types::{
    InitializeParams, SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
};

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
    _config: Config,
    _params: InitializeParams,
) -> LSPProcessResult<()> {
    eprintln!("Running server loop");
    while let Ok(message) = connection.receiver.recv() {
        eprintln!("Received message: {:?}", message);
    }

    panic!("Client exited without proper shutdown sequence.")
}
