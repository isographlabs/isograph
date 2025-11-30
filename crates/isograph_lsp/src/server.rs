#![allow(clippy::print_stderr)]

use crate::{
    completion::on_completion,
    format::on_format,
    goto_definition::on_goto_definition,
    hover::on_hover,
    lsp_notification_dispatch::LSPNotificationDispatch,
    lsp_request_dispatch::LSPRequestDispatch,
    lsp_runtime_error::LSPRuntimeError,
    references::on_references,
    semantic_tokens::on_semantic_token_full_request,
    text_document::{
        on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
    },
};
use colored::Colorize;
use common_lang_types::{
    CurrentWorkingDirectory, Diagnostic, DiagnosticResult, DiagnosticVecResult,
};
use isograph_compiler::{
    CompilerState, WithDuration, update_sources,
    watch::{create_debounced_file_watcher, has_config_changes},
};
use isograph_lang_types::semantic_token_legend::semantic_token_legend;
use isograph_schema::NetworkProtocol;
use log::{info, warn};
use lsp_server::{Connection, ErrorCode, Response, ResponseError};
use lsp_types::{
    CompletionOptions, HoverProviderCapability,
    request::{Completion, HoverRequest, References, SemanticTokensFullRequest},
};
use lsp_types::{
    InitializeParams, OneOf, SemanticTokensFullOptions, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    request::{Formatting, GotoDefinition},
};
use prelude::Postfix;
use std::{ops::ControlFlow, path::PathBuf};

/// Initializes an LSP connection, handling the `initialize` message and `initialized` notification
/// handshake.
pub fn initialize(connection: &Connection) -> DiagnosticResult<InitializeParams> {
    let server_capabilities = ServerCapabilities {
        // Enable text document syncing so we can know when files are opened/changed/saved/closed
        text_document_sync: TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)
            .wrap_some(),
        semantic_tokens_provider: SemanticTokensServerCapabilities::SemanticTokensOptions(
            SemanticTokensOptions {
                work_done_progress_options: WorkDoneProgressOptions::default(),
                legend: semantic_token_legend(),
                range: None,
                full: SemanticTokensFullOptions::Bool(true).wrap_some(),
            },
        )
        .wrap_some(),
        hover_provider: HoverProviderCapability::Simple(true).wrap_some(),
        document_formatting_provider: OneOf::Left(true).wrap_some(),
        definition_provider: OneOf::Left(true).wrap_some(),
        completion_provider: Some(CompletionOptions {
            trigger_characters: Some(vec!["\n".to_string()]),
            ..Default::default()
        }),
        references_provider: OneOf::Left(true).wrap_some(),
        ..Default::default()
    };
    let server_capabilities =
        serde_json::to_value(server_capabilities).map_err(|e| Diagnostic::from_error(e, None))?;
    let params = connection
        .initialize(server_capabilities)
        .map_err(|e| Diagnostic::from_error(e, None))?;

    serde_json::from_value::<InitializeParams>(params)
        .map_err(|e| Diagnostic::from_error(e, None))?
        .wrap_ok()
}

/// Run the main server loop
pub async fn run<TNetworkProtocol: NetworkProtocol>(
    connection: Connection,
    config_location: &PathBuf,
    _params: InitializeParams,
    current_working_directory: CurrentWorkingDirectory,
) -> DiagnosticVecResult<()> {
    let mut compiler_state: CompilerState<TNetworkProtocol> =
        CompilerState::new(config_location, current_working_directory)?;

    eprintln!("Running server loop");

    let (tokio_sender, mut lsp_message_receiver) = tokio::sync::mpsc::channel(100);
    bridge_crossbeam_to_tokio(connection.receiver, tokio_sender);

    let config = compiler_state.db.get_isograph_config().clone();

    let (mut file_system_receiver, mut file_system_watcher) =
        create_debounced_file_watcher(&config);

    'all_messages: loop {
        tokio::select! {
            message = lsp_message_receiver.recv() => {
                if let Some(lsp_message) = message {
                    let duration = WithDuration::new(|| {
                        match lsp_message {
                            lsp_server::Message::Request(request) => {
                                eprintln!("\nReceived request: {}", request.method);
                                let response = dispatch_request(request, &compiler_state);
                                eprintln!("Sending response: {response:?}");
                                connection.sender.send(response.into()).unwrap();
                            }
                            lsp_server::Message::Notification(notification) => {
                                eprintln!("\nReceived notification: {}", notification.method);
                                let _ = dispatch_notification(notification, &mut compiler_state);
                            }
                            lsp_server::Message::Response(response) => {
                                eprintln!("\nReceived response: {response:?}");
                            }
                        }
                    });
                    eprintln!("Processing took {}ms.", duration.elapsed_time.as_millis());
                } else {
                    // If any connection breaks, we can just end
                    break 'all_messages;
                }
            }
            message = file_system_receiver.recv() => {
                if let Some(Ok(changes)) = message {
                    if has_config_changes(&changes) {
                        info!(
                            "{}",
                            "Config change detected.".cyan()
                        );
                        compiler_state = CompilerState::new(config_location, current_working_directory)?;
                        file_system_watcher.stop();
                        // TODO is this a bug? Will we continue to watch the old folders? I think so.
                        (file_system_receiver, file_system_watcher) =
                            create_debounced_file_watcher(&config);

                        // TODO this is a temporary expedient. We need a good way to copy the old DB state to the
                        // new DB. Namely, there's an open files hash map that needs to be transferred over.
                        //
                        // That will probably be a bit more easily solved when we have a db macro.
                        warn!("Shutting down language server. This is not currently supported");
                        // Wrapping this in if true, because otherwise, cargo complains that the above code
                        // is useless! And that's true. But this is a temporary expedient, because we
                        // don't actually want to break here.
                        if true {
                            break 'all_messages;
                        }
                    } else {
                        info!("{}", "File changes detected. Starting to compile.".cyan());
                        update_sources(&mut compiler_state.db, &changes)?;
                        compiler_state.run_garbage_collection();
                    };
                } else {
                    // If any connection breaks or we have some file system errors, we can just end here.
                    break 'all_messages;
                }
            }
        };
    }

    Ok(())
}

fn dispatch_notification<TNetworkProtocol: NetworkProtocol>(
    notification: lsp_server::Notification,
    compiler_state: &mut CompilerState<TNetworkProtocol>,
) -> ControlFlow<Option<LSPRuntimeError>, ()> {
    LSPNotificationDispatch::new(notification, compiler_state)
        .on_notification_sync::<DidOpenTextDocument>(on_did_open_text_document)?
        .on_notification_sync::<DidCloseTextDocument>(on_did_close_text_document)?
        .on_notification_sync::<DidChangeTextDocument>(on_did_change_text_document)?
        .notification();

    ControlFlow::Continue(())
}
fn dispatch_request<TNetworkProtocol: NetworkProtocol>(
    request: lsp_server::Request,
    compiler_state: &CompilerState<TNetworkProtocol>,
) -> Response {
    // Returns ControlFlow::Break(ServerResponse) if the request
    // was handled, ControlFlow::Continue(Request) otherwise.
    let get_response = || {
        let request = LSPRequestDispatch::new(request, compiler_state)
            .on_request_sync::<SemanticTokensFullRequest>(on_semantic_token_full_request)?
            .on_request_sync::<HoverRequest>(on_hover)?
            .on_request_sync::<Formatting>(on_format)?
            .on_request_sync::<GotoDefinition>(on_goto_definition)?
            .on_request_sync::<Completion>(on_completion)?
            .on_request_sync::<References>(on_references)?
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

fn bridge_crossbeam_to_tokio<T: Send + 'static>(
    crossbeam_receiver: crossbeam::channel::Receiver<T>,
    tokio_sender: tokio::sync::mpsc::Sender<T>,
) {
    std::thread::spawn(move || {
        while let Ok(msg) = crossbeam_receiver.recv() {
            // Use blocking_send since we're in a std::thread, not tokio task
            if tokio_sender.blocking_send(msg).is_err() {
                break;
            }
        }
    });
}
