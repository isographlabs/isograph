use std::ops::ControlFlow;

use crate::lsp_runtime_error::LSPRuntimeError;
use crate::lsp_state::LSPState;
use crate::text_document::{
    on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
};
use crate::{
    lsp_notification_dispatch::LSPNotificationDispatch, lsp_process_error::LSPProcessResult,
};
use isograph_config::CompilerConfig as Config;
use lsp_server::Connection;
use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
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
    let mut state = LSPState::new();
    while let Ok(message) = connection.receiver.recv() {
        match message {
            lsp_server::Message::Request(request) => {
                eprintln!("Received request: {:?}", request);
            }
            lsp_server::Message::Notification(notification) => {
                dispatch_notification(notification, &mut state);
                eprintln!("State after notification: {:?}", state);
            }
            lsp_server::Message::Response(response) => {
                eprintln!("Received response: {:?}", response);
            }
        }
    }

    panic!("Client exited without proper shutdown sequence.")
}

fn dispatch_notification(
    notification: lsp_server::Notification,
    lsp_state: &mut LSPState,
) -> ControlFlow<Option<LSPRuntimeError>, ()> {
    LSPNotificationDispatch::new(notification, lsp_state)
        .on_notification_sync::<DidOpenTextDocument>(on_did_open_text_document)?
        .on_notification_sync::<DidCloseTextDocument>(on_did_close_text_document)?
        .on_notification_sync::<DidChangeTextDocument>(on_did_change_text_document)?
        .notification();

    ControlFlow::Continue(())
}
