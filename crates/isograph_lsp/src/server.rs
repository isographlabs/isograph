#![allow(clippy::print_stderr)]

use crate::{
    code_action::on_code_action,
    commands::{all_commands, on_command},
    completion::on_completion,
    diagnostic_notification::publish_new_diagnostics_and_clear_old_diagnostics,
    document_highlight::on_document_highlight,
    format::on_format,
    goto_definition::on_goto_definition,
    hover::on_hover,
    lsp_notification_dispatch::LSPNotificationDispatch,
    lsp_request_dispatch::LSPRequestDispatch,
    lsp_runtime_error::LSPRuntimeError,
    lsp_state::LspState,
    semantic_tokens::on_semantic_token_full_request,
    text_document::{
        on_did_change_text_document, on_did_close_text_document, on_did_open_text_document,
    },
};
use common_lang_types::{
    CurrentWorkingDirectory, LocationFreeDiagnostic, LocationFreeDiagnosticResult,
    LocationFreeDiagnosticVecResult,
};
use isograph_compiler::{
    CompilerState, WithDuration, update_sources,
    watch::{create_debounced_file_watcher, has_config_changes},
};
use isograph_config::{CompilerConfig, create_config};
use isograph_lang_types::semantic_token_legend::semantic_token_legend;
use isograph_schema::{CompilationProfile, validate_entire_schema};
use lsp_server::{Connection, ErrorCode, Response, ResponseError};
use lsp_types::{
    CodeActionProviderCapability, CompletionOptions, ExecuteCommandOptions,
    HoverProviderCapability,
    request::{
        CodeActionRequest, Completion, DocumentHighlightRequest, ExecuteCommand, HoverRequest,
        Request, SemanticTokensFullRequest,
    },
};
use lsp_types::{
    InitializeParams, OneOf, SemanticTokensFullOptions, SemanticTokensOptions,
    SemanticTokensServerCapabilities, ServerCapabilities, TextDocumentSyncCapability,
    TextDocumentSyncKind, WorkDoneProgressOptions,
    notification::{DidChangeTextDocument, DidCloseTextDocument, DidOpenTextDocument},
    request::{Formatting, GotoDefinition},
};
use prelude::{ErrClone, Postfix};
use std::{
    collections::BTreeSet,
    ops::{ControlFlow, Not},
    time::Duration,
};
use tokio::time::{Instant, sleep};

/// Initializes an LSP connection, handling the `initialize` message and `initialized` notification
/// handshake.
pub fn initialize(connection: &Connection) -> LocationFreeDiagnosticResult<InitializeParams> {
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
        completion_provider: CompletionOptions {
            trigger_characters: vec!["\n".to_string()].wrap_some(),
            ..Default::default()
        }
        .wrap_some(),
        document_highlight_provider: Some(OneOf::Left(true)),
        code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
        execute_command_provider: ExecuteCommandOptions {
            commands: all_commands(),
            ..Default::default()
        }
        .wrap_some(),
        ..Default::default()
    };
    let server_capabilities =
        serde_json::to_value(server_capabilities).map_err(LocationFreeDiagnostic::from_error)?;
    let params = connection
        .initialize(server_capabilities)
        .map_err(LocationFreeDiagnostic::from_error)?;

    serde_json::from_value::<InitializeParams>(params)
        .map_err(LocationFreeDiagnostic::from_error)?
        .wrap_ok()
}

const SHORT_DEBOUNCE_TIME: Duration = Duration::from_millis(100);
const LONG_DEBOUNCE_TIME: Duration = Duration::from_hours(24 * 365);

/// Run the main server loop
pub async fn run<TCompilationProfile: CompilationProfile>(
    connection: Connection,
    config: CompilerConfig,
    _params: InitializeParams,
    current_working_directory: CurrentWorkingDirectory,
) -> LocationFreeDiagnosticVecResult<()> {
    let config_location = config.config_location.clone();

    let (mut file_system_receiver, mut file_system_watcher) =
        create_debounced_file_watcher(&config);

    let compiler_state: CompilerState<TCompilationProfile> =
        CompilerState::new(config, current_working_directory).map_err(|e| e.wrap_vec())?;
    let mut lsp_state = LspState::new(compiler_state, &connection.sender);

    #[allow(clippy::mutable_key_type)]
    let mut uris_with_diagnostics = BTreeSet::new().note_todo(
        "When we panic, we should clear all diagnostics. \
        Add a panic_unwind handler for that.",
    );

    eprintln!("Running server loop");

    let (tokio_sender, mut lsp_message_receiver) = tokio::sync::mpsc::channel(100);
    bridge_crossbeam_to_tokio(connection.receiver, tokio_sender);

    // After 100ms of inactivity, we compile the codebase and emit diagnostics.
    // Note that in response to events, we delay the debounce timer.
    let debounce_timer = sleep(SHORT_DEBOUNCE_TIME);
    tokio::pin!(debounce_timer);

    'all_messages: loop {
        tokio::select! {
            message = lsp_message_receiver.recv() => {
                if let Some(lsp_message) = message {
                    let duration = WithDuration::new(|| {
                        match lsp_message {
                            lsp_server::Message::Request(request) => {
                                eprintln!("\nReceived request: {}", request.method);
                                let print_full_response = should_print_full_response(&request);
                                let response = dispatch_request(request, &lsp_state);
                                if print_full_response {
                                    eprintln!("Succeeded; sending response: {response:?}");
                                } else {
                                    eprintln!("Succeeded; sending response.");
                                }
                                connection.sender.send(response.into()).unwrap();
                            }
                            lsp_server::Message::Notification(notification) => {
                                eprintln!("\nReceived notification: {}", notification.method);
                                let _ = dispatch_notification(notification, &mut lsp_state);

                                // NOTE: we attempt to be judicious, i.e. only trigger the debounce timer
                                // if we receive a notification (which may have changed state).
                                debounce_timer.as_mut().reset(Instant::now() + SHORT_DEBOUNCE_TIME);
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
                        eprintln!("Config change detected.");
                        let config = create_config(&config_location, current_working_directory);
                        file_system_watcher.stop();
                        // TODO is this a bug? Will we continue to watch the old folders? I think so.
                        (file_system_receiver, file_system_watcher) =
                            create_debounced_file_watcher(&config);
                        let compiler_state = CompilerState::new(config, current_working_directory).map_err(|e| e.wrap_vec())?;
                        lsp_state = LspState::new(compiler_state, &connection.sender);

                        // TODO this is a temporary expedient. We need a good way to copy the old DB state to the
                        // new DB. Namely, there's an open files hash map that needs to be transferred over.
                        //
                        // That will probably be a bit more easily solved when we have a db macro.
                        eprintln!("Shutting down language server. This is not currently supported");
                        // Wrapping this in if true, because otherwise, cargo complains that the above code
                        // is useless! And that's true. But this is a temporary expedient, because we
                        // don't actually want to break here.
                        if true {
                            break 'all_messages;
                        }
                    } else {
                        eprintln!("File changes detected. Starting to compile.");
                        update_sources(&mut lsp_state.compiler_state.db, &changes)?;

                        lsp_state.compiler_state.run_garbage_collection();
                    };

                    debounce_timer.as_mut().reset(Instant::now() + SHORT_DEBOUNCE_TIME);
                } else {
                    // If any connection breaks or we have some file system errors, we can just end here.
                    break 'all_messages;
                }
            }
            _ = &mut debounce_timer => {
                let diagnostics = validate_entire_schema(&lsp_state.compiler_state.db)
                    .clone_err()
                    .err()
                    .unwrap_or_default();

                eprintln!("Publishing diagnostics {:?}", diagnostics);

                uris_with_diagnostics = publish_new_diagnostics_and_clear_old_diagnostics(
                    &lsp_state.compiler_state.db,
                    &diagnostics,
                    &connection.sender,
                    uris_with_diagnostics
                );

                debounce_timer.as_mut().reset(Instant::now() + LONG_DEBOUNCE_TIME);
            }
        };
    }

    ().wrap_ok()
}

fn dispatch_notification<TCompilationProfile: CompilationProfile>(
    notification: lsp_server::Notification,
    lsp_state: &mut LspState<TCompilationProfile>,
) -> ControlFlow<Option<LSPRuntimeError>, ()> {
    LSPNotificationDispatch::new(notification, lsp_state)
        .on_notification_sync::<DidOpenTextDocument>(on_did_open_text_document)?
        .on_notification_sync::<DidCloseTextDocument>(on_did_close_text_document)?
        .on_notification_sync::<DidChangeTextDocument>(on_did_change_text_document)?
        .notification();

    ControlFlow::Continue(())
}
fn dispatch_request<TCompilationProfile: CompilationProfile>(
    request: lsp_server::Request,
    lsp_state: &LspState<TCompilationProfile>,
) -> Response {
    // Returns ControlFlow::Break(ServerResponse) if the request
    // was handled, ControlFlow::Continue(Request) otherwise.
    let get_response = || {
        let request = LSPRequestDispatch::new(request, lsp_state)
            .on_request_sync::<SemanticTokensFullRequest>(on_semantic_token_full_request)?
            .on_request_sync::<HoverRequest>(on_hover)?
            .on_request_sync::<Formatting>(on_format)?
            .on_request_sync::<GotoDefinition>(on_goto_definition)?
            .on_request_sync::<Completion>(on_completion)?
            .on_request_sync::<DocumentHighlightRequest>(on_document_highlight)?
            .on_request_sync::<CodeActionRequest>(on_code_action)?
            .on_request_sync::<ExecuteCommand>(on_command)?
            .request();

        // If we have gotten here, we have not handled the request
        ControlFlow::Continue(request)
    };

    match get_response() {
        ControlFlow::Break(response) => response,
        ControlFlow::Continue(request) => Response {
            id: request.id,
            result: None,
            error: ResponseError {
                code: ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!("No handler registered for method '{}'", request.method),
            }
            .wrap_some(),
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

/// Certain methods have really large responses, and they're not useful! We don't want to print
/// the full response for those.
static NON_PRINTABLE_METHODS: &'static [&'static str] = &[SemanticTokensFullRequest::METHOD];

fn should_print_full_response(request: &lsp_server::Request) -> bool {
    NON_PRINTABLE_METHODS
        .contains(&request.method.as_str())
        .not()
}
