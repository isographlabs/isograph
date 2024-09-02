use std::ops::ControlFlow;

use crate::lsp_request_dispatch::LSPRequestDispatch;
use crate::lsp_runtime_error::LSPRuntimeError;
use crate::lsp_state::LSPState;
use crate::semantic_tokens::on_semantic_token_full_request;
use crate::semantic_tokens::semantic_token_legend::semantic_token_legend;
use crate::text_document::{
    on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
};
use crate::{
    lsp_notification_dispatch::LSPNotificationDispatch, lsp_process_error::LSPProcessResult,
};
use isograph_config::CompilerConfig as Config;
use lsp_server::{Connection, ErrorCode, Response, ResponseError};
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::{
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    InitializeParams, SemanticTokensFullOptions, SemanticTokensOptions,
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
                legend: semantic_token_legend(),
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
    let mut state = LSPState::new(connection.sender.clone());
    while let Ok(message) = connection.receiver.recv() {
        match message {
            lsp_server::Message::Request(request) => {
                eprintln!("Received request: {:?}", request);
                let response = dispatch_request(request, &mut state);
                eprintln!("Sending response: {:?}", response);
                state.send_message(response.into());
            }
            lsp_server::Message::Notification(notification) => {
                dispatch_notification(notification, &mut state);
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
fn dispatch_request(request: lsp_server::Request, lsp_state: &mut LSPState) -> Response {
    // Returns ControlFlow::Break(ServerResponse) if the request
    // was handled, ControlFlow::Continue(Request) otherwise.
    let get_response = || {
        let request = LSPRequestDispatch::new(request, lsp_state)
            .on_request_sync::<SemanticTokensFullRequest>(on_semantic_token_full_request)?
            .request();

        // If we have gotten here, we have not handled the request
        ControlFlow::Continue(request)
    };

    match get_response() {
        ControlFlow::Break(response) => response,
        ControlFlow::Continue(request) => Response {
            id: request.id,
            result: None,
            error: Some(ResponseError {
                code: ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!("No handler registered for method '{}'", request.method),
            }),
        },
    }
}
