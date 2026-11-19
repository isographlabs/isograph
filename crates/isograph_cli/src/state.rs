use std::ops::ControlFlow;
use std::path::Path;

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathError, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use intern::Lookup;
use intern::string_key::Intern;
use isograph_compiler::{ClientId, CompilationProfile};
use pico::Database;
use prelude::Postfix;

use crate::effect::{IsographEffect, no_effects};
use crate::event::{
    ClientConnectionEvent, DiskChanged, InternalEvent, IsographEvent, LiveSessions, Presence,
};
use crate::timer::TimerPayload;
use crate::watch::ProjectGlobs;

pub use isograph_compiler::IsographDb;

pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}

pub fn handle<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal_event(state, internal),
        IsographEvent::Timer(fired) => handle_timer_fired(state, fired),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
}

fn handle_client_connection<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    event: ClientConnectionEvent,
) -> Vec<IsographEffect> {
    match event {
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            no_effects()
        }
        ClientConnectionEvent::Gone(client) => {
            state.live_sessions.0.remove(&client);
            state.db.remove_open_files_of_client(client);
            no_effects()
        }
    }
}

fn dispatch_lsp<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    lsp: crate::event::LspEvent,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            let Some(before_compute) = state.live_sessions.reply(client, id) else {
                return no_effects();
            };
            dispatch_lsp_request(state, incoming, before_compute)
        }
        crate::event::LspEvent::Notification(notification) => {
            if !state.live_sessions.0.contains_key(&notification.client) {
                return no_effects();
            }
            dispatch_lsp_notification(state, notification)
        }
        crate::event::LspEvent::Response(crate::event::LspResponse {
            client,
            response: _response,
        }) => {
            if !state.live_sessions.0.contains_key(&client) {
                return no_effects();
            }
            no_effects()
        }
    }
}

fn dispatch_lsp_request<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
    let get_response = || {
        let request = crate::lsp_request_dispatch::LSPRequestDispatch::new(incoming, &state.db)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                crate::adapter::semantic_tokens_response::<TProfile>,
            )?
            .on_request_sync::<lsp_types::request::Formatting>(
                crate::adapter::formatting_response::<TProfile>,
            )?
            .on_request_sync::<lsp_types::request::WillSaveWaitUntil>(
                crate::adapter::will_save_wait_until_response::<TProfile>,
            )?
            .on_request_sync::<lsp_types::request::HoverRequest>(
                crate::adapter::hover_response::<TProfile>,
            )?
            .on_request_sync::<lsp_types::request::GotoDefinition>(
                crate::adapter::definition_response::<TProfile>,
            )?
            .on_request_sync::<lsp_types::request::DocumentHighlightRequest>(
                crate::adapter::document_highlight_response::<TProfile>,
            )?
            .request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => {
            send_lsp_response(&mut state.live_sessions, after, response_body(response))
        }
        ControlFlow::Continue(request) => {
            method_not_found(&mut state.live_sessions, after, request)
        }
    }
}

fn send_lsp_response(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    body: Result<serde_json::Value, lsp_server::ResponseError>,
) -> Vec<IsographEffect> {
    let Some(send) = sessions.after_compute(after, body) else {
        return no_effects();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect { send }.boxed(),
    )
    .wrap_vec()
}

fn method_not_found(
    sessions: &mut crate::event::LiveSessions,
    after: crate::event::AfterCompute,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        sessions,
        after,
        lsp_server::ResponseError {
            code: lsp_server::ErrorCode::MethodNotFound as i32,
            data: None,
            message: format!("No handler registered for method '{}'", request.method),
        }
        .wrap_err(),
    )
}

fn response_body(
    response: lsp_server::Response,
) -> Result<serde_json::Value, lsp_server::ResponseError> {
    match (response.result, response.error) {
        (Some(value), None) => value.wrap_ok(),
        (_, Some(error)) => error.wrap_err(),
        (None, None) => lsp_server::ResponseError {
            code: lsp_server::ErrorCode::UnknownErrorCode as i32,
            data: None,
            message: "LSP response has neither result nor error".to_owned(),
        }
        .wrap_err(),
    }
}

fn handle_internal_event<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => {
            handle_disk_changed(&mut state.db, state.globs.reference(), change)
        }
    }
}

fn handle_timer_fired<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.diagnostics_debounce.as_ref()) {
        Some(&TimerPayload::DiagnosticsDebounce) => {
            state.diagnostics_debounce = None;
            crate::diagnostics::publish_diagnostics(state)
        }
        None => no_effects(),
    }
}

fn dispatch_lsp_notification<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    notification: crate::event::LspNotification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event_notification::<TProfile>,
            )?
            .on_notification_sync::<lsp_types::notification::DidOpenTextDocument>(
                crate::adapter::on_did_open::<TProfile>,
            )?
            .on_notification_sync::<lsp_types::notification::DidChangeTextDocument>(
                crate::adapter::on_did_change::<TProfile>,
            )?
            .on_notification_sync::<lsp_types::notification::DidCloseTextDocument>(
                crate::adapter::on_did_close::<TProfile>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}

fn on_isograph_event_notification<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    _client: ClientId,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    handle_internal_event(state, internal)
}

pub(crate) fn intern_config_directory(
    state: &mut IsographDb<impl CompilationProfile>,
    config_path: &Path,
) {
    let directory = config_path
        .parent()
        .expect("a config file path has a parent directory");
    let interned: CurrentWorkingDirectory = directory
        .to_str()
        .expect("the config directory is UTF-8")
        .intern()
        .to();
    state.set(interned);
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[error("{}: {error}", .path.display())]
struct DiskChangedPathError {
    path: std::path::PathBuf,
    error: RelativePathError,
}

fn relative_path_to_source_file(
    state: &IsographDb<impl CompilationProfile>,
    absolute: &Path,
) -> Result<RelativePathToSourceFile, DiskChangedPathError> {
    let cwd = *state
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("CurrentWorkingDirectory is interned from the config path before DiskChanged");
    relative_path_from_absolute_and_working_directory(cwd, absolute).map_err(|error| {
        DiskChangedPathError {
            path: absolute.to_owned(),
            error,
        }
    })
}

fn handle_disk_changed<TProfile: CompilationProfile>(
    db: &mut IsographDb<TProfile>,
    globs: &ProjectGlobs,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(db, globs, change)
        .unwrap_or_else(|error| tracing::warn!(%error, "DiskChanged path"));
    no_effects()
}

fn apply_disk_changed<TProfile: CompilationProfile>(
    db: &mut IsographDb<TProfile>,
    globs: &ProjectGlobs,
    change: DiskChanged,
) -> Result<(), DiskChangedPathError> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(db, &change.path)?;
            match change.presence {
                Presence::Present(contents) => db.insert_disk_file(
                    path,
                    globs.membership::<TProfile>(Path::new(path.lookup())),
                    contents,
                ),
                Presence::Absent => db.remove_disk_file(path),
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(db, &folder.path)?;
            db.remove_disk_files_from_path(path);
        }
    }
    ().wrap_ok()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use intern::Lookup;
    use intern::string_key::Intern;
    use lsp_types::notification::Notification;
    use lsp_types::request::Request;
    use prelude::Postfix;

    use crate::test_profile::TestProfile;

    use super::{IsographDb, IsographState, handle, intern_config_directory};
    use crate::diagnostics::PublishedDiagnostics;
    use crate::effect::IsographEffect;
    use crate::event::{
        ClientConnectionEvent, ClientStartedEvent, DiskChanged, DiskFileChanged, FolderRemoved,
        InternalEvent, IsographEvent, LiveSession, LiveSessions, LspEvent, LspNotification,
        LspResponse, Presence,
    };
    use crate::timer::TimerPayload;
    use isograph_compiler::ClientId;
    use std::collections::HashMap;

    const SOURCE_FILE: isograph_compiler::FileMembership = isograph_compiler::FileMembership {
        source_file: isograph_compiler::IncludeFile::Include,
        schema_file: isograph_compiler::IncludeFile::Skip,
    };

    fn interned(s: &str) -> common_lang_types::RelativePathToSourceFile {
        s.intern().to()
    }

    fn empty() -> IsographState<TestProfile> {
        IsographState {
            db: IsographDb::<TestProfile>::default(),
            live_sessions: LiveSessions::default(),
            globs: crate::watch::ProjectGlobs::parse(&["src/**".to_owned()], &[])
                .expect("the test glob is valid"),
            diagnostics_debounce: None,
        }
    }

    fn with_config() -> IsographState<TestProfile> {
        let mut state = empty();
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn contents(
        state: &IsographState<TestProfile>,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<&str> {
        isograph_compiler::file_contents(state.db.reference(), ClientId(1), path).as_deref()
    }

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }

    #[test]
    fn timer_fired_on_a_never_armed_state_returns_no_effects() {
        let mut armed = with_config();
        started(&mut armed, ClientId(1));
        let timer = only_timer(handle(&mut armed, did_open(ClientId(1), X_URI, "open")));
        let mut state = empty();
        let effects = handle(&mut state, timer.event.to());
        assert!(effects.is_empty());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::Quit.to());
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn disk_changed_present_interns_a_path_relative_to_the_config() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert_eq!(
            contents(state.reference(), interned("src/a.ts")),
            "export const a = 1;\n".wrap_some()
        );
    }

    #[test]
    fn disk_changed_present_outside_the_config_dir_is_a_parent_relative_path() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/other/a.ts"),
                presence: Presence::Present("outside".to_owned()),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert_eq!(
            contents(state.reference(), interned("../other/a.ts")),
            "outside".wrap_some()
        );
    }

    #[test]
    fn disk_changed_absent_removes_the_relative_path() {
        let mut state = with_config();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
    }

    fn start_client(
        state: &mut IsographState<TestProfile>,
        client: ClientId,
        sender: crossbeam::channel::Sender<lsp_server::Message>,
    ) {
        handle(
            state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(sender, lsp_types::ClientCapabilities::default()),
            })
            .to(),
        );
    }

    fn lsp_request(
        client: ClientId,
        id: lsp_server::RequestId,
        method: &str,
        params: serde_json::Value,
    ) -> IsographEvent {
        LspEvent::Request(crate::event::LspRequest {
            client,
            request: lsp_server::Request {
                id,
                method: method.to_owned(),
                params,
            },
        })
        .to()
    }

    fn formatting_params() -> serde_json::Value {
        serde_json::json!({
            "textDocument": { "uri": "file:///tmp/proj/src/x.ts" },
            "options": { "tabSize": 2, "insertSpaces": true }
        })
    }

    #[test]
    fn lsp_request_returns_method_not_found() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        let id = lsp_server::RequestId::from(1);
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                id.clone(),
                "textDocument/references",
                serde_json::json!({}),
            ),
        );
        assert_eq!(effects.len(), 1);
        let effect = effects
            .into_iter()
            .next()
            .expect("handle returned one effect");
        assert!(
            matches!(effect, crate::effect::IsographEffect::SendLspResponse(_)),
            "handle of LspRequest returns SendLspResponseEffect"
        );
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = crate::daemon::perform(
            &crate::effect::SideEffectToken::mint(),
            effect,
            event_tx.reference(),
        );
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        assert_eq!(response.id, id);
        assert!(response.result.is_none());
        let error = response.error.as_ref().expect("MethodNotFound is an error");
        assert_eq!(error.code, lsp_server::ErrorCode::MethodNotFound as i32);
        assert_eq!(
            error.message,
            "No handler registered for method 'textDocument/references'"
        );
        assert!(
            state
                .live_sessions
                .0
                .get(&client)
                .expect("the client is live")
                .outstanding
                .is_empty()
        );
    }

    #[test]
    fn lsp_formatting_of_a_live_client_without_a_file_is_null() {
        let mut state = with_config();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        let id = lsp_server::RequestId::from(1);
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                id.clone(),
                lsp_types::request::Formatting::METHOD,
                formatting_params(),
            ),
        );
        assert_eq!(effects.len(), 1);
        let effect = effects
            .into_iter()
            .next()
            .expect("handle returned one effect");
        assert!(
            matches!(effect, crate::effect::IsographEffect::SendLspResponse(_)),
            "handle of Formatting returns SendLspResponseEffect"
        );
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = crate::daemon::perform(
            &crate::effect::SideEffectToken::mint(),
            effect,
            event_tx.reference(),
        );
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        assert_eq!(response.id, id);
        assert_eq!(response.result, serde_json::Value::Null.wrap_some());
        assert!(
            state
                .live_sessions
                .0
                .get(&client)
                .expect("the client is live")
                .outstanding
                .is_empty()
        );
    }

    #[test]
    fn lsp_formatting_of_a_live_client_with_empty_params_returns_an_error() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        let id = lsp_server::RequestId::from(1);
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                id.clone(),
                lsp_types::request::Formatting::METHOD,
                serde_json::json!({}),
            ),
        );
        assert_eq!(effects.len(), 1);
        let effect = effects
            .into_iter()
            .next()
            .expect("handle returned one effect");
        assert!(
            matches!(effect, crate::effect::IsographEffect::SendLspResponse(_)),
            "handle of Formatting with empty params returns SendLspResponseEffect"
        );
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = crate::daemon::perform(
            &crate::effect::SideEffectToken::mint(),
            effect,
            event_tx.reference(),
        );
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        assert_eq!(response.id, id);
        assert!(response.result.is_none());
        assert!(response.error.is_some());
        assert!(
            state
                .live_sessions
                .0
                .get(&client)
                .expect("the client is live")
                .outstanding
                .is_empty()
        );
    }

    #[test]
    fn lsp_request_without_a_live_session_returns_no_effects() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            lsp_request(
                ClientId(1),
                lsp_server::RequestId::from(1),
                "textDocument/hover",
                serde_json::json!({}),
            ),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_request_from_a_sibling_client_returns_no_effects() {
        let mut state = empty();
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, ClientId(1), tx);
        let effects = handle(
            &mut state,
            lsp_request(
                ClientId(2),
                lsp_server::RequestId::from(1),
                "textDocument/hover",
                serde_json::json!({}),
            ),
        );
        assert!(effects.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lsp_request_after_gone_returns_no_effects() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        handle(&mut state, ClientConnectionEvent::Gone(client).to());
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                lsp_server::RequestId::from(1),
                "textDocument/hover",
                serde_json::json!({}),
            ),
        );
        assert!(effects.is_empty());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn lsp_formatting_from_a_client_that_is_not_live_returns_no_effects() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            lsp_request(
                ClientId(1),
                lsp_server::RequestId::from(1),
                lsp_types::request::Formatting::METHOD,
                formatting_params(),
            ),
        );
        assert!(effects.is_empty());
    }

    #[test]
    #[should_panic(
        expected = "CurrentWorkingDirectory is interned from the config path before DiskChanged"
    )]
    fn disk_changed_without_a_config_directory_panics() {
        let mut state = empty();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }))
            .to(),
        );
    }

    #[test]
    fn folder_removed_of_src_removes_files_under_src_not_src2() {
        let mut state = with_config();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/b.ts"),
                presence: Presence::Present("b".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src2/c.ts"),
                presence: Presence::Present("c".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
                path: PathBuf::from("/tmp/proj/src"),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
        assert!(contents(state.reference(), interned("src/b.ts")).is_none());
        assert_eq!(
            contents(state.reference(), interned("src2/c.ts")),
            "c".wrap_some()
        );
    }

    #[test]
    fn folder_removed_of_the_config_directory_removes_every_interned_file() {
        let mut state = with_config();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src2/c.ts"),
                presence: Presence::Present("c".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
                path: PathBuf::from("/tmp/proj"),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
        assert!(contents(state.reference(), interned("src2/c.ts")).is_none());
    }

    #[test]
    fn folder_removed_of_a_never_interned_path_is_a_noop() {
        let mut state = with_config();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
                path: PathBuf::from("/tmp/proj/never"),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert_eq!(
            contents(state.reference(), interned("src/a.ts")),
            "a".wrap_some()
        );
    }

    fn only_response(
        effects: Vec<IsographEffect>,
        rx: &crossbeam::channel::Receiver<lsp_server::Message>,
    ) -> lsp_server::Response {
        let [effect] = <[IsographEffect; 1]>::try_from(effects)
            .unwrap_or_else(|effects| panic!("expected one effect, got {}", effects.len()));
        perform_send(effect);
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        response
    }

    #[test]
    fn disk_changed_of_a_relative_path_returns_no_effects_and_interns_nothing() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(state.db.get_disk_file_map().untracked().0.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn disk_changed_of_a_non_utf8_path_returns_no_effects_and_interns_nothing() {
        use std::ffi::OsStr;
        use std::os::unix::ffi::OsStrExt;
        let mut state = with_config();
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from(OsStr::from_bytes(b"/tmp/proj/\xFF.ts")),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(state.db.get_disk_file_map().untracked().0.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn semantic_tokens_of_a_non_utf8_file_uri_is_an_error_response() {
        let mut state = with_config();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                lsp_server::RequestId::from(1),
                lsp_types::request::SemanticTokensFullRequest::METHOD,
                serde_json::json!({ "textDocument": { "uri": "file:///tmp/proj/%FF.ts" } }),
            ),
        );
        let response = only_response(effects, rx.reference());
        let error = response.error.expect("a non-UTF-8 path is an error");
        assert_eq!(error.code, lsp_server::ErrorCode::UnknownErrorCode as i32);
        assert_eq!(error.message, "the relative path is not UTF-8");
    }

    #[test]
    fn formatting_without_a_working_directory_is_an_error_response() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(&mut state, client, tx);
        let effects = handle(
            &mut state,
            lsp_request(
                client,
                lsp_server::RequestId::from(1),
                lsp_types::request::Formatting::METHOD,
                formatting_params(),
            ),
        );
        let response = only_response(effects, rx.reference());
        let error = response
            .error
            .expect("a missing working directory is an error");
        assert_eq!(error.message, "CurrentWorkingDirectory is not interned");
    }

    const X_URI: &str = "file:///tmp/proj/src/x.ts";

    fn document_notification(
        client: ClientId,
        method: &str,
        params: serde_json::Value,
    ) -> IsographEvent {
        LspEvent::Notification(LspNotification {
            client,
            notification: lsp_server::Notification {
                method: method.to_owned(),
                params,
            },
        })
        .to()
    }

    fn did_open(client: ClientId, uri: &str, text: &str) -> IsographEvent {
        document_notification(
            client,
            lsp_types::notification::DidOpenTextDocument::METHOD,
            serde_json::json!({
                "textDocument": { "uri": uri, "languageId": "typescript", "version": 1, "text": text }
            }),
        )
    }

    fn did_change(client: ClientId, uri: &str, changes: serde_json::Value) -> IsographEvent {
        document_notification(
            client,
            lsp_types::notification::DidChangeTextDocument::METHOD,
            serde_json::json!({
                "textDocument": { "uri": uri, "version": 2 },
                "contentChanges": changes
            }),
        )
    }

    fn did_close(client: ClientId, uri: &str) -> IsographEvent {
        document_notification(
            client,
            lsp_types::notification::DidCloseTextDocument::METHOD,
            serde_json::json!({ "textDocument": { "uri": uri } }),
        )
    }

    fn with_disk_x() -> IsographState<TestProfile> {
        let mut state = with_config();
        state
            .db
            .insert_disk_file(interned("src/x.ts"), SOURCE_FILE, "disk".to_owned());
        state
    }

    fn started(state: &mut IsographState<TestProfile>, client: ClientId) {
        let (tx, _rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
    }

    fn read(state: &IsographState<TestProfile>, client: ClientId, path: &str) -> Option<String> {
        isograph_compiler::file_contents(state.db.reference(), client, interned(path)).clone()
    }

    #[test]
    fn did_open_stores_the_buffer_for_that_client_only() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        let effects = handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        only_timer(effects);
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "open".to_owned().wrap_some()
        );
        assert_eq!(
            read(state.reference(), ClientId(2), "src/x.ts"),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn did_change_with_one_full_change_replaces_the_buffer() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        let effects = handle(
            &mut state,
            did_change(
                ClientId(1),
                X_URI,
                serde_json::json!([{ "text": "changed" }]),
            ),
        );
        only_timer(effects);
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "changed".to_owned().wrap_some()
        );
    }

    #[test]
    fn did_change_that_is_not_one_full_change_leaves_the_buffer() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        let ranged = serde_json::json!([{
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 1 }
            },
            "text": "ranged"
        }]);
        for changes in [
            serde_json::json!([]),
            serde_json::json!([{ "text": "first" }, { "text": "second" }]),
            ranged,
        ] {
            let effects = handle(&mut state, did_change(ClientId(1), X_URI, changes.clone()));
            assert!(effects.is_empty(), "{changes}");
            assert_eq!(
                read(state.reference(), ClientId(1), "src/x.ts"),
                "open".to_owned().wrap_some(),
                "{changes}"
            );
        }
    }

    #[test]
    fn did_close_falls_back_to_disk() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "open".to_owned().wrap_some()
        );
        let effects = handle(&mut state, did_close(ClientId(1), X_URI));
        only_timer(effects);
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn two_clients_that_open_one_uri_each_read_their_own() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        started(&mut state, ClientId(2));
        handle(&mut state, did_open(ClientId(1), X_URI, "one"));
        handle(&mut state, did_open(ClientId(2), X_URI, "two"));
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "one".to_owned().wrap_some()
        );
        assert_eq!(
            read(state.reference(), ClientId(2), "src/x.ts"),
            "two".to_owned().wrap_some()
        );
    }

    #[test]
    fn gone_removes_only_that_clients_buffers() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        started(&mut state, ClientId(2));
        handle(&mut state, did_open(ClientId(1), X_URI, "one"));
        handle(&mut state, did_open(ClientId(2), X_URI, "two"));
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "one".to_owned().wrap_some()
        );
        let effects = handle(&mut state, ClientConnectionEvent::Gone(ClientId(1)).to());
        assert!(effects.is_empty());
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "disk".to_owned().wrap_some()
        );
        assert_eq!(
            read(state.reference(), ClientId(2), "src/x.ts"),
            "two".to_owned().wrap_some()
        );
    }

    #[test]
    fn gone_of_a_client_that_never_started_leaves_other_buffers() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(2));
        handle(&mut state, did_open(ClientId(2), X_URI, "two"));
        let effects = handle(&mut state, ClientConnectionEvent::Gone(ClientId(7)).to());
        assert!(effects.is_empty());
        assert_eq!(
            read(state.reference(), ClientId(2), "src/x.ts"),
            "two".to_owned().wrap_some()
        );
    }

    fn with_schema_glob() -> IsographState<TestProfile> {
        let mut state = with_config();
        state.globs =
            crate::watch::ProjectGlobs::parse(&["src/**".to_owned()], &["schema/**".to_owned()])
                .expect("the test globs are valid");
        state
    }

    fn disk_file_changed(path: &str, presence: Presence) -> IsographEvent {
        InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
            path: PathBuf::from(path),
            presence,
        }))
        .to()
    }

    fn source_disk_paths(state: &IsographState<TestProfile>) -> Vec<&'static str> {
        state
            .db
            .get_source_path_map()
            .untracked()
            .disk
            .iter()
            .map(|path| path.lookup())
            .collect()
    }

    fn schema_disk_paths(state: &IsographState<TestProfile>) -> Vec<&'static str> {
        state
            .db
            .get_schema_path_map()
            .untracked()
            .disk
            .iter()
            .map(|path| path.lookup())
            .collect()
    }

    #[test]
    fn disk_changed_lists_source_and_schema_paths_by_the_globs() {
        let mut state = with_schema_glob();
        handle(
            &mut state,
            disk_file_changed("/tmp/proj/src/a.ts", Presence::Present("a".to_owned())),
        );
        handle(
            &mut state,
            disk_file_changed(
                "/tmp/proj/schema/a.graphql",
                Presence::Present("type Query".to_owned()),
            ),
        );
        handle(
            &mut state,
            disk_file_changed("/tmp/proj/other/a.ts", Presence::Present("b".to_owned())),
        );
        assert_eq!(source_disk_paths(state.reference()), "src/a.ts".wrap_vec());
        assert_eq!(
            schema_disk_paths(state.reference()),
            "schema/a.graphql".wrap_vec()
        );
        handle(
            &mut state,
            disk_file_changed("/tmp/proj/src/a.ts", Presence::Absent),
        );
        handle(
            &mut state,
            disk_file_changed("/tmp/proj/schema/a.graphql", Presence::Absent),
        );
        assert_eq!(source_disk_paths(state.reference()), Vec::<&str>::new());
        assert_eq!(schema_disk_paths(state.reference()), Vec::<&str>::new());
    }

    #[test]
    fn did_open_lists_only_a_source_file_and_did_close_and_gone_drop_it() {
        let mut state = with_config();
        started(&mut state, ClientId(1));
        handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        handle(
            &mut state,
            did_open(ClientId(1), "file:///tmp/proj/other/a.ts", "open"),
        );
        assert_eq!(
            state.db.open_source_paths_of_client(ClientId(1)),
            interned("src/x.ts").wrap_vec()
        );
        handle(&mut state, did_close(ClientId(1), X_URI));
        assert_eq!(
            state.db.open_source_paths_of_client(ClientId(1)),
            Vec::<common_lang_types::RelativePathToSourceFile>::new()
        );
        handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        handle(&mut state, ClientConnectionEvent::Gone(ClientId(1)).to());
        assert_eq!(
            state.db.open_source_paths_of_client(ClientId(1)),
            Vec::<common_lang_types::RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn did_open_from_a_client_that_never_started_stores_nothing() {
        let mut state = with_disk_x();
        let effects = handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        assert!(effects.is_empty());
        assert_eq!(
            read(state.reference(), ClientId(1), "src/x.ts"),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn did_open_of_a_non_file_uri_stores_nothing() {
        let mut state = with_config();
        started(&mut state, ClientId(1));
        let effects = handle(
            &mut state,
            did_open(ClientId(1), "https://example.net/a.ts", "open"),
        );
        assert!(effects.is_empty());
        assert!(state.db.get_open_file_map().untracked().0.is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn did_open_of_a_non_utf8_file_uri_stores_nothing() {
        let mut state = with_config();
        started(&mut state, ClientId(1));
        let effects = handle(
            &mut state,
            did_open(ClientId(1), "file:///tmp/proj/%FF.ts", "open"),
        );
        assert!(effects.is_empty());
        assert!(state.db.get_open_file_map().untracked().0.is_empty());
    }

    #[test]
    fn did_open_without_a_working_directory_stores_nothing() {
        let mut state = empty();
        started(&mut state, ClientId(1));
        let effects = handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        assert!(effects.is_empty());
        assert!(state.db.get_open_file_map().untracked().0.is_empty());
    }

    const HOME_URI: &str = "file:///tmp/proj/src/Home.ts";
    const HOME_DISK: &str = "export const Home = iso(`entrypoint Query.HomeRoute`)";
    const FIELD_X: &str = "iso(`field Query.x{id}`)";
    const ENTRYPOINT_A: &str = "iso(`entrypoint Query.A`)";
    const KEYWORD: u32 = 15;

    fn disk_changed(state: &mut IsographState<TestProfile>, path: &str, text: &str) {
        handle(
            state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from(path),
                presence: Presence::Present(text.to_owned()),
            }))
            .to(),
        );
    }

    struct TestClient {
        client: ClientId,
        rx: crossbeam::channel::Receiver<lsp_server::Message>,
    }

    fn live_client(state: &mut IsographState<TestProfile>, client: ClientId) -> TestClient {
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
        TestClient { client, rx }
    }

    fn request_result(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        method: &str,
        params: serde_json::Value,
    ) -> serde_json::Value {
        let effects = handle(
            state,
            lsp_request(
                client.client,
                lsp_server::RequestId::from(1),
                method,
                params,
            ),
        );
        only_response(effects, client.rx.reference())
            .result
            .expect("the request succeeds")
    }

    fn formatting_edits(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
    ) -> Option<Vec<lsp_types::TextEdit>> {
        let result = request_result(
            state,
            client,
            lsp_types::request::Formatting::METHOD,
            serde_json::json!({
                "textDocument": { "uri": uri },
                "options": { "tabSize": 2, "insertSpaces": true }
            }),
        );
        serde_json::from_value(result).expect("Formatting returns TextEdit[] | null")
    }

    fn will_save_edits(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
    ) -> Option<Vec<lsp_types::TextEdit>> {
        let result = request_result(
            state,
            client,
            lsp_types::request::WillSaveWaitUntil::METHOD,
            serde_json::json!({ "textDocument": { "uri": uri }, "reason": 1 }),
        );
        serde_json::from_value(result).expect("willSaveWaitUntil returns TextEdit[] | null")
    }

    fn semantic_tokens(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
    ) -> Option<Vec<lsp_types::SemanticToken>> {
        let result = request_result(
            state,
            client,
            lsp_types::request::SemanticTokensFullRequest::METHOD,
            serde_json::json!({ "textDocument": { "uri": uri } }),
        );
        serde_json::from_value::<Option<lsp_types::SemanticTokens>>(result)
            .expect("semanticTokens/full returns SemanticTokens | null")
            .map(|tokens| tokens.data)
    }

    fn with_disk_home() -> IsographState<TestProfile> {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/Home.ts", HOME_DISK);
        state
    }

    fn first_token(tokens: Option<Vec<lsp_types::SemanticToken>>) -> lsp_types::SemanticToken {
        *tokens
            .expect("the file has text")
            .first()
            .expect("the literal has a token")
    }

    #[test]
    fn formatting_reads_the_clients_buffer_over_disk() {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", "const x = 1;\n");
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, X_URI, FIELD_X));
        let edits = formatting_edits(&mut state, &one, X_URI).expect("the buffer has text");
        let [edit] = edits.as_slice() else {
            panic!("expected one edit, got {edits:?}");
        };
        assert_eq!(edit.new_text, "\n  field Query.x {\n    id,\n  }\n");
    }

    #[test]
    fn will_save_wait_until_returns_the_formatting_edits_of_the_buffer() {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", "const x = 1;\n");
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, X_URI, FIELD_X));
        let formatting = formatting_edits(&mut state, &one, X_URI);
        let will_save = will_save_edits(&mut state, &one, X_URI);
        assert_eq!(will_save, formatting);
        assert_eq!(will_save.map(|edits| edits.len()), 1.wrap_some());
    }

    #[test]
    fn formatting_after_did_close_reads_disk() {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", "const x = 1;\n");
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, X_URI, FIELD_X));
        assert_eq!(
            formatting_edits(&mut state, &one, X_URI).map(|edits| edits.len()),
            1.wrap_some()
        );
        handle(&mut state, did_close(one.client, X_URI));
        assert_eq!(
            formatting_edits(&mut state, &one, X_URI),
            Vec::new().wrap_some()
        );
    }

    /// Client 1 opens `buffer` over a disk file without literals; `request` edits are applied to `buffer`.
    fn buffer_after_edits(
        buffer: &str,
        request: fn(
            &mut IsographState<TestProfile>,
            &TestClient,
            &str,
        ) -> Option<Vec<lsp_types::TextEdit>>,
    ) -> String {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", "const x = 1;\n");
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, X_URI, buffer));
        let edits = request(&mut state, &one, X_URI).expect("the buffer has text");
        crate::adapter::tests::assert_edits_replace_disjoint_interiors(buffer, &edits);
        crate::adapter::tests::apply_text_edits(buffer, &edits)
    }

    #[test]
    fn formatting_two_literals_in_a_buffer_leaves_the_javascript_around_them() {
        use crate::adapter::tests::{TWO_LITERALS, TWO_LITERALS_PRETTY};
        assert_eq!(
            buffer_after_edits(TWO_LITERALS, formatting_edits),
            TWO_LITERALS_PRETTY
        );
        assert_eq!(
            buffer_after_edits(TWO_LITERALS_PRETTY, formatting_edits),
            TWO_LITERALS_PRETTY
        );
    }

    #[test]
    fn formatting_three_literals_in_a_buffer_leaves_the_javascript_around_them() {
        use crate::adapter::tests::{THREE_LITERALS, THREE_LITERALS_PRETTY};
        assert_eq!(
            buffer_after_edits(THREE_LITERALS, formatting_edits),
            THREE_LITERALS_PRETTY
        );
        assert_eq!(
            buffer_after_edits(THREE_LITERALS_PRETTY, formatting_edits),
            THREE_LITERALS_PRETTY
        );
    }

    #[test]
    fn will_save_wait_until_of_two_literals_in_a_buffer_formats_both() {
        use crate::adapter::tests::{TWO_LITERALS, TWO_LITERALS_PRETTY};
        assert_eq!(
            buffer_after_edits(TWO_LITERALS, will_save_edits),
            TWO_LITERALS_PRETTY
        );
    }

    #[test]
    fn will_save_wait_until_of_three_literals_in_a_buffer_formats_all_three() {
        use crate::adapter::tests::{THREE_LITERALS, THREE_LITERALS_PRETTY};
        assert_eq!(
            buffer_after_edits(THREE_LITERALS, will_save_edits),
            THREE_LITERALS_PRETTY
        );
    }

    #[test]
    fn semantic_tokens_follow_did_open_did_change_and_did_close() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let two = live_client(&mut state, ClientId(2));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        let opened = first_token(semantic_tokens(&mut state, &one, HOME_URI));
        assert_eq!(opened.token_type, KEYWORD);
        assert_eq!(opened.length, 5);
        handle(
            &mut state,
            did_change(
                one.client,
                HOME_URI,
                serde_json::json!([{ "text": ENTRYPOINT_A }]),
            ),
        );
        let changed = first_token(semantic_tokens(&mut state, &one, HOME_URI));
        assert_eq!(changed.token_type, KEYWORD);
        assert_eq!(changed.length, 10);
        handle(&mut state, did_close(one.client, HOME_URI));
        assert_eq!(
            semantic_tokens(&mut state, &one, HOME_URI),
            semantic_tokens(&mut state, &two, HOME_URI)
        );
    }

    #[test]
    fn two_clients_with_one_uri_open_get_tokens_of_their_own_buffers() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let two = live_client(&mut state, ClientId(2));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        handle(&mut state, did_open(two.client, HOME_URI, ENTRYPOINT_A));
        assert_eq!(
            first_token(semantic_tokens(&mut state, &one, HOME_URI)).length,
            5
        );
        assert_eq!(
            first_token(semantic_tokens(&mut state, &two, HOME_URI)).length,
            10
        );
    }

    #[test]
    fn a_client_that_never_opened_the_uri_gets_disk_tokens() {
        let mut disk_only = with_disk_home();
        let reader = live_client(&mut disk_only, ClientId(2));
        let disk_tokens = semantic_tokens(&mut disk_only, &reader, HOME_URI);
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let two = live_client(&mut state, ClientId(2));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        assert_eq!(
            first_token(semantic_tokens(&mut state, &one, HOME_URI)).length,
            5
        );
        assert_eq!(semantic_tokens(&mut state, &two, HOME_URI), disk_tokens);
        assert_eq!(first_token(disk_tokens).length, 10);
    }

    #[test]
    fn semantic_tokens_of_an_open_file_with_no_disk_file_are_null_after_did_close() {
        const NEW_URI: &str = "file:///tmp/proj/src/new.ts";
        let mut state = with_config();
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, NEW_URI, FIELD_X));
        assert!(semantic_tokens(&mut state, &one, NEW_URI).is_some());
        handle(&mut state, did_close(one.client, NEW_URI));
        assert_eq!(semantic_tokens(&mut state, &one, NEW_URI), None);
    }

    #[test]
    fn an_empty_buffer_over_a_disk_literal_has_no_tokens() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, HOME_URI, ""));
        assert_eq!(
            semantic_tokens(&mut state, &one, HOME_URI),
            Vec::new().wrap_some()
        );
    }

    fn hover_params(uri: &str, line: u32, character: u32) -> serde_json::Value {
        serde_json::json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        })
    }

    /// The hover at `line:character`, or `None` for a `null` result.
    fn hover(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<lsp_types::Hover> {
        let result = request_result(
            state,
            client,
            lsp_types::request::HoverRequest::METHOD,
            hover_params(uri, line, character),
        );
        serde_json::from_value::<Option<lsp_types::Hover>>(result)
            .expect("hover returns Hover | null")
    }

    fn markup_value(hover: lsp_types::Hover) -> String {
        match hover.contents {
            lsp_types::HoverContents::Markup(markup) => markup.value,
            contents => panic!("expected markup contents, got {contents:?}"),
        }
    }

    /// The Markdown text of the hover at `line:character`, or `None` for a `null` result.
    fn hover_text(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<String> {
        hover(state, client, uri, line, character).map(markup_value)
    }

    /// The UTF-16 column of the first `needle` in the one-line `line`.
    fn column(line: &str, needle: &str) -> u32 {
        let byte = line.find(needle).expect("the fixture contains the needle");
        line[..byte].encode_utf16().count() as u32
    }

    /// Client 1's hover at `line:character` of `src/x.ts` on disk as `text`.
    fn disk_hover(text: &str, line: u32, character: u32) -> Option<lsp_types::Hover> {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", text);
        let one = live_client(&mut state, ClientId(1));
        hover(&mut state, &one, X_URI, line, character)
    }

    fn hover_on_disk(text: &str, line: u32, character: u32) -> Option<String> {
        disk_hover(text, line, character).map(markup_value)
    }

    /// The range of client 1's hover at `line:character` of `src/x.ts` on disk as `text`.
    fn hover_range_on_disk(
        text: &str,
        line: u32,
        character: u32,
    ) -> Option<Option<lsp_types::Range>> {
        disk_hover(text, line, character).map(|hover| hover.range)
    }

    /// The range on `line` from UTF-16 column `start` to `end`, as a hover range.
    fn hover_char_range(line: u32, start: u32, end: u32) -> Option<Option<lsp_types::Range>> {
        lsp_types::Range {
            start: lsp_types::Position {
                line,
                character: start,
            },
            end: lsp_types::Position {
                line,
                character: end,
            },
        }
        .wrap_some()
        .wrap_some()
    }

    fn some(text: &str) -> Option<String> {
        text.to_owned().wrap_some()
    }

    #[test]
    fn hover_names_the_leaf_under_each_char_of_an_entrypoint() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let mut hover =
            |needle: &str| hover_text(&mut state, &one, HOME_URI, 0, column(HOME_DISK, needle));
        assert_eq!(hover("Query"), None);
        assert_eq!(hover("HomeRoute"), None);
        assert_eq!(hover("entrypoint"), some("EntrypointDeclaration"));
        assert_eq!(hover("."), some("EntrypointDeclaration"));
    }

    #[test]
    fn hover_outside_the_interior_is_null() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let opening = column(HOME_DISK, "`");
        let closing = HOME_DISK
            .rfind('`')
            .expect("the fixture has a closing backtick") as u32;
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, 1), None);
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, column(HOME_DISK, "iso(")),
            None
        );
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, opening), None);
        assert!(hover_text(&mut state, &one, HOME_URI, 0, opening + 1).is_some());
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, closing), None);
    }

    #[test]
    fn hover_contents_are_markdown() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let result = request_result(
            &mut state,
            &one,
            lsp_types::request::HoverRequest::METHOD,
            hover_params(HOME_URI, 0, column(HOME_DISK, "entrypoint")),
        );
        assert_eq!(result["contents"]["kind"], serde_json::json!("markdown"));
        assert_eq!(
            result["contents"]["value"],
            serde_json::json!("EntrypointDeclaration")
        );
    }

    #[test]
    fn hover_reads_the_clients_buffer_and_disk_after_did_close() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let two = live_client(&mut state, ClientId(2));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        let brace = column(FIELD_X, "{");
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, brace),
            some("SelectionSet")
        );
        assert_eq!(hover_text(&mut state, &two, HOME_URI, 0, brace), None);
        let disk_keyword = column(HOME_DISK, "entrypoint");
        assert_eq!(
            hover_text(&mut state, &two, HOME_URI, 0, disk_keyword),
            some("EntrypointDeclaration")
        );
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, disk_keyword),
            None
        );
        handle(&mut state, did_close(one.client, HOME_URI));
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, brace), None);
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, disk_keyword),
            some("EntrypointDeclaration")
        );
    }

    #[test]
    fn hover_after_did_change_reads_the_changed_buffer() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        let character = column(FIELD_X, "field");
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, character),
            some("SelectableDeclaration")
        );
        handle(
            &mut state,
            did_change(
                one.client,
                HOME_URI,
                serde_json::json!([{ "text": ENTRYPOINT_A }]),
            ),
        );
        assert_eq!(
            hover_text(&mut state, &one, HOME_URI, 0, character),
            some("EntrypointDeclaration")
        );
    }

    #[test]
    fn hover_in_two_literals_on_two_lines_resolves_in_each() {
        let text = format!("{ENTRYPOINT_A}\n{FIELD_X}");
        assert_eq!(
            hover_on_disk(&text, 0, column(ENTRYPOINT_A, "entrypoint")),
            some("EntrypointDeclaration")
        );
        assert_eq!(
            hover_on_disk(&text, 1, column(FIELD_X, "field")),
            some("SelectableDeclaration")
        );
    }

    #[test]
    fn hover_in_two_literals_on_one_line_resolves_in_each() {
        let text = format!("{ENTRYPOINT_A} {FIELD_X}");
        assert_eq!(
            hover_on_disk(&text, 0, column(&text, "entrypoint")),
            some("EntrypointDeclaration")
        );
        assert_eq!(
            hover_on_disk(&text, 0, column(&text, "field")),
            some("SelectableDeclaration")
        );
        assert_eq!(hover_on_disk(&text, 0, column(&text, ") iso")), None);
    }

    const DECLARED: &str = "iso(`field Pet.x($id: ID!) { id }`)";
    const ENTRYPOINT_QUERY_X: &str = "iso(`entrypoint Query.X`)";
    const NESTED: &str = "iso(`field Pet.fullName { owner { name } }`)";
    const ALIASED: &str = "iso(`field Pet.fullName { b: bar }`)";

    /// Each one-line fixture with the UTF-16 column of every entity, selectable, and selection
    /// name in it that no declaration in the fixture defines.
    fn undeclared_schema_name_positions() -> Vec<(&'static str, u32)> {
        vec![
            (DECLARED, column(DECLARED, "Pet")),
            (DECLARED, column(DECLARED, "ID")),
            (DECLARED, column(DECLARED, "{ id") + 2),
            (ENTRYPOINT_QUERY_X, column(ENTRYPOINT_QUERY_X, "Query")),
            (ENTRYPOINT_QUERY_X, column(ENTRYPOINT_QUERY_X, "X")),
            (NESTED, column(NESTED, "owner")),
            (NESTED, column(NESTED, "name")),
            (ALIASED, column(ALIASED, "b:")),
            (ALIASED, column(ALIASED, "bar")),
        ]
    }

    #[test]
    fn hover_on_an_undeclared_schema_name_is_null() {
        for (text, character) in undeclared_schema_name_positions() {
            assert_eq!(
                hover_on_disk(text, 0, character),
                None,
                "{text:?} at {character}"
            );
        }
        assert_eq!(
            hover_on_disk(DECLARED, 0, column(DECLARED, "field")),
            some("SelectableDeclaration")
        );
        assert_eq!(
            hover_on_disk(
                ENTRYPOINT_QUERY_X,
                0,
                column(ENTRYPOINT_QUERY_X, "entrypoint")
            ),
            some("EntrypointDeclaration")
        );
    }

    /// Client 1's `textDocument/definition` result at `line:character` of `src/x.ts` on disk
    /// as `text`.
    fn definition_on_disk(text: &str, line: u32, character: u32) -> serde_json::Value {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", text);
        let one = live_client(&mut state, ClientId(1));
        request_result(
            &mut state,
            &one,
            lsp_types::request::GotoDefinition::METHOD,
            hover_params(X_URI, line, character),
        )
    }

    #[test]
    fn definition_of_an_undeclared_schema_name_is_null() {
        let others = [
            (DECLARED, column(DECLARED, "field")),
            (DECLARED, column(DECLARED, "{")),
            (ENTRYPOINT_QUERY_X, column(ENTRYPOINT_QUERY_X, "entrypoint")),
        ];
        for (text, character) in undeclared_schema_name_positions().into_iter().chain(others) {
            assert_eq!(
                definition_on_disk(text, 0, character),
                serde_json::Value::Null,
                "{text:?} at {character}"
            );
        }
    }

    const PET_FILE: &str = "iso(`entity Pet`) iso(`field Pet.owner to Pet { id }`)";
    const FULL_NAME_FILE: &str = "iso(`field Pet.fullName { id }`)";
    const SELECTING_FILE: &str = "iso(`field Pet.x { owner { fullName } }`)";

    /// A one-line range of `src/<file>` in a `textDocument/definition` result.
    fn definition_json(file: &str, text: &str, needle: &str) -> serde_json::Value {
        let start = column(text, needle);
        serde_json::json!({
            "uri": format!("file:///tmp/proj/src/{file}"),
            "range": {
                "start": { "line": 0, "character": start },
                "end": { "line": 0, "character": start + needle.encode_utf16().count() as u32 },
            },
        })
    }

    #[test]
    fn hover_and_definition_read_declarations_across_source_files() {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/Pet.ts", PET_FILE);
        disk_changed(&mut state, "/tmp/proj/src/fullName.ts", FULL_NAME_FILE);
        disk_changed(&mut state, "/tmp/proj/src/x.ts", SELECTING_FILE);
        let one = live_client(&mut state, ClientId(1));
        let at = |needle: &str| hover_params(X_URI, 0, column(SELECTING_FILE, needle));
        let definition = |state: &mut IsographState<TestProfile>, needle: &str| {
            request_result(
                state,
                &one,
                lsp_types::request::GotoDefinition::METHOD,
                at(needle),
            )
        };
        assert_eq!(
            hover_text(
                &mut state,
                &one,
                X_URI,
                0,
                column(SELECTING_FILE, "fullName")
            ),
            some("field **Pet.fullName**")
        );
        assert_eq!(
            definition(&mut state, "fullName"),
            definition_json("fullName.ts", FULL_NAME_FILE, "fullName")
        );
        assert_eq!(
            hover_text(&mut state, &one, X_URI, 0, column(SELECTING_FILE, "owner")),
            some("field **Pet.owner**: **Pet**")
        );
        assert_eq!(
            definition(&mut state, "owner"),
            definition_json("Pet.ts", PET_FILE, "owner")
        );
        assert_eq!(
            hover_text(&mut state, &one, X_URI, 0, column(SELECTING_FILE, "Pet")),
            some("entity **Pet**")
        );
        assert_eq!(
            definition(&mut state, "Pet"),
            definition_json("Pet.ts", PET_FILE, "Pet")
        );
    }

    #[test]
    fn a_declarations_own_name_resolves_to_that_declaration() {
        let character = column(DECLARED, "x(");
        assert_eq!(
            hover_on_disk(DECLARED, 0, character),
            some("field **Pet.x**")
        );
        assert_eq!(
            definition_on_disk(DECLARED, 0, character),
            serde_json::json!({
                "uri": X_URI,
                "range": {
                    "start": { "line": 0, "character": character },
                    "end": { "line": 0, "character": character + 1 },
                },
            })
        );
    }

    #[test]
    fn hover_in_an_empty_or_whitespace_only_literal_is_null() {
        assert_eq!(hover_on_disk("iso(``)", 0, 4), None);
        assert_eq!(hover_on_disk("iso(``)", 0, 5), None);
        assert_eq!(hover_on_disk("iso(`  `)", 0, 5), None);
        assert_eq!(hover_on_disk("iso(`  `)", 0, 6), None);
    }

    #[test]
    fn hover_after_wide_chars_uses_utf16_columns() {
        for prefix in ["const s = \"😀\"; ", "const s = \"é\"; "] {
            let text = format!("{prefix}{ENTRYPOINT_A}");
            assert_eq!(
                hover_on_disk(&text, 0, column(&text, ".")),
                some("EntrypointDeclaration"),
                "after {prefix:?}"
            );
        }
    }

    #[test]
    fn hover_after_crlf_and_cr_is_on_the_next_line() {
        for line_break in ["\r\n", "\r"] {
            let text = format!("const x = 1;{line_break}{ENTRYPOINT_A}");
            assert_eq!(
                hover_on_disk(&text, 1, column(ENTRYPOINT_A, ".")),
                some("EntrypointDeclaration"),
                "after {line_break:?}"
            );
        }
    }

    const PET_MULTILINE: &str = "const iso: any = null;\n\nexport const fullName = iso(`\n    field Pet.fullName @foo(bar: 123, baz: $qux)\n    {\n        id\n    }\n`);\n";
    const PET_ONE_LINE: &str = "export const fullName = iso(`field Pet.fullName @foo(bar: 123, baz: $qux) { id, name }`);\n";

    /// The line and UTF-16 column of the `nth` (0-based) `needle` after `after` in `text`.
    fn position_of(text: &str, after: &str, needle: &str, nth: usize) -> (u32, u32) {
        let from = text.find(after).expect("the fixture contains `after`");
        let (byte, _) = text[from..]
            .match_indices(needle)
            .nth(nth)
            .expect("the fixture contains the needle");
        let byte = from + byte;
        let line_start = text[..byte].rfind('\n').map_or(0, |newline| newline + 1);
        (
            text[..byte].matches('\n').count() as u32,
            text[line_start..byte].encode_utf16().count() as u32,
        )
    }

    /// Client 1's hover at the `nth` `needle` after `after`, through a didOpen buffer of `text`.
    fn buffer_hover(text: &str, after: &str, needle: &str, nth: usize) -> Option<lsp_types::Hover> {
        let mut state = with_config();
        let one = live_client(&mut state, ClientId(1));
        handle(&mut state, did_open(one.client, X_URI, text));
        let (line, character) = position_of(text, after, needle, nth);
        hover(&mut state, &one, X_URI, line, character)
    }

    fn hover_in_buffer(text: &str, after: &str, needle: &str, nth: usize) -> Option<String> {
        buffer_hover(text, after, needle, nth).map(markup_value)
    }

    #[test]
    fn hover_on_argument_colons_and_commas_of_the_pet_literal() {
        for text in [PET_MULTILINE, PET_ONE_LINE] {
            for nth in [0, 1] {
                let (line, character) = position_of(text, "iso(`", ":", nth);
                assert_eq!(
                    hover_on_disk(text, line, character),
                    some("Argument"),
                    "colon {nth} of {text:?}"
                );
                assert_eq!(
                    hover_in_buffer(text, "iso(`", ":", nth),
                    some("Argument"),
                    "colon {nth} of the buffer {text:?}"
                );
            }
            let (line, character) = position_of(text, "iso(`", ",", 0);
            assert_eq!(
                hover_on_disk(text, line, character),
                some("ArgumentList"),
                "argument comma of {text:?}"
            );
            assert_eq!(
                hover_in_buffer(text, "iso(`", ",", 0),
                some("ArgumentList"),
                "argument comma of the buffer {text:?}"
            );
        }
    }

    #[test]
    fn hover_range_on_argument_colons_and_commas_is_that_char() {
        for text in [PET_MULTILINE, PET_ONE_LINE] {
            for (needle, nth) in [(":", 0), (":", 1), (",", 0)] {
                let (line, character) = position_of(text, "iso(`", needle, nth);
                let expected = hover_char_range(line, character, character + 1);
                assert_eq!(
                    hover_range_on_disk(text, line, character),
                    expected,
                    "{needle:?} {nth} of {text:?}"
                );
                assert_eq!(
                    buffer_hover(text, "iso(`", needle, nth).map(|hover| hover.range),
                    expected,
                    "{needle:?} {nth} of the buffer {text:?}"
                );
            }
        }
    }

    #[test]
    fn hover_range_is_the_first_and_the_last_interior_char() {
        let first = column(FIELD_X, "field");
        let last = column(FIELD_X, "`)") - 1;
        assert_eq!(
            hover_range_on_disk(FIELD_X, 0, first),
            hover_char_range(0, first, first + 1)
        );
        assert_eq!(
            hover_range_on_disk(FIELD_X, 0, last),
            hover_char_range(0, last, last + 1)
        );
    }

    #[test]
    fn hover_range_after_wide_chars_is_in_utf16_columns() {
        for prefix in ["const s = \"😀\"; ", "const s = \"é\"; "] {
            let text = format!("{prefix}{ENTRYPOINT_A}");
            let dot = column(&text, ".");
            assert_eq!(
                hover_range_on_disk(&text, 0, dot),
                hover_char_range(0, dot, dot + 1),
                "after {prefix:?}"
            );
        }
    }

    #[test]
    fn hover_range_on_a_multi_byte_char_in_a_string_literal_spans_its_utf16_units() {
        let text = "iso(`field Pet.fullName @foo(bar: \"é😀\") { id }`)";
        let e_acute = column(text, "é");
        let emoji = column(text, "😀");
        assert_eq!(
            hover_range_on_disk(text, 0, e_acute),
            hover_char_range(0, e_acute, e_acute + 1)
        );
        assert_eq!(
            hover_range_on_disk(text, 0, emoji),
            hover_char_range(0, emoji, emoji + 2)
        );
    }

    #[test]
    fn hover_range_after_crlf_and_cr_is_on_the_next_line() {
        for line_break in ["\r\n", "\r"] {
            let text = format!("const x = 1;{line_break}{ENTRYPOINT_A}");
            let dot = column(ENTRYPOINT_A, ".");
            assert_eq!(
                hover_range_on_disk(&text, 1, dot),
                hover_char_range(1, dot, dot + 1),
                "after {line_break:?}"
            );
        }
    }

    #[test]
    fn hover_on_a_comma_between_selections_is_the_selection_set() {
        let (line, character) = position_of(PET_ONE_LINE, "{ id", ",", 0);
        assert_eq!(
            hover_on_disk(PET_ONE_LINE, line, character),
            some("SelectionSet")
        );
        assert_eq!(
            hover_in_buffer(PET_ONE_LINE, "{ id", ",", 0),
            some("SelectionSet")
        );
    }

    #[test]
    fn hover_on_an_unknown_keyword_is_the_slot() {
        let text = "iso(`fieldd Query.foo`)";
        assert_eq!(
            hover_on_disk(text, 0, column(text, "fieldd")),
            some("IsoLiteralSlot")
        );
    }

    #[test]
    fn hover_past_the_line_end_or_the_last_line_is_null() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let end = HOME_DISK.len() as u32;
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, end), None);
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, end + 1), None);
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 1, 0), None);
    }

    #[test]
    fn hover_of_a_path_without_text_is_null() {
        let mut state = with_config();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(hover_text(&mut state, &one, HOME_URI, 0, 0), None);
    }

    #[test]
    fn hover_of_a_non_file_uri_is_null() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(
            hover_text(&mut state, &one, "untitled:Untitled-1", 0, 0),
            None
        );
    }

    fn highlights(
        state: &mut IsographState<TestProfile>,
        client: &TestClient,
        uri: &str,
        line: u32,
        character: u32,
    ) -> Option<Vec<lsp_types::DocumentHighlight>> {
        let result = request_result(
            state,
            client,
            lsp_types::request::DocumentHighlightRequest::METHOD,
            hover_params(uri, line, character),
        );
        serde_json::from_value(result)
            .expect("documentHighlight returns DocumentHighlight[] | null")
    }

    /// The one text highlight over `line:start..line:end`.
    fn text_highlight(
        line: u32,
        start: u32,
        end: u32,
    ) -> Option<Vec<lsp_types::DocumentHighlight>> {
        lsp_types::DocumentHighlight {
            range: lsp_types::Range {
                start: lsp_types::Position {
                    line,
                    character: start,
                },
                end: lsp_types::Position {
                    line,
                    character: end,
                },
            },
            kind: lsp_types::DocumentHighlightKind::TEXT.wrap_some(),
        }
        .wrap_vec()
        .wrap_some()
    }

    /// The highlight of the token `needle` on the one-line `line_text`.
    fn highlight_of(
        line: u32,
        line_text: &str,
        needle: &str,
    ) -> Option<Vec<lsp_types::DocumentHighlight>> {
        let start = column(line_text, needle);
        text_highlight(line, start, start + needle.encode_utf16().count() as u32)
    }

    /// Client 1's highlights at `line:character` of `src/x.ts` on disk as `text`.
    fn highlights_on_disk(
        text: &str,
        line: u32,
        character: u32,
    ) -> Option<Vec<lsp_types::DocumentHighlight>> {
        let mut state = with_config();
        disk_changed(&mut state, "/tmp/proj/src/x.ts", text);
        let one = live_client(&mut state, ClientId(1));
        highlights(&mut state, &one, X_URI, line, character)
    }

    #[test]
    fn highlight_on_a_name_is_exactly_that_token() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let expected = highlight_of(0, HOME_DISK, "HomeRoute");
        let first = column(HOME_DISK, "HomeRoute");
        let last = first + "HomeRoute".len() as u32 - 1;
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, first), expected);
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, last), expected);
        assert_eq!(
            highlights(&mut state, &one, HOME_URI, 0, column(HOME_DISK, "Query")),
            highlight_of(0, HOME_DISK, "Query")
        );
    }

    #[test]
    fn highlight_on_a_period_is_the_period() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(
            highlights(&mut state, &one, HOME_URI, 0, column(HOME_DISK, ".")),
            highlight_of(0, HOME_DISK, ".")
        );
    }

    #[test]
    fn highlight_between_tokens_inside_a_literal_is_empty() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(
            highlights(&mut state, &one, HOME_URI, 0, column(HOME_DISK, " Query")),
            Vec::new().wrap_some()
        );
        let trailing = "iso(`entrypoint Query.A  `)";
        assert_eq!(
            highlights_on_disk(trailing, 0, column(trailing, "  `") + 1),
            Vec::new().wrap_some()
        );
        let comma = "iso(`field Query.x { a, b }`)";
        assert_eq!(
            highlights_on_disk(comma, 0, column(comma, ",")),
            Vec::new().wrap_some()
        );
    }

    #[test]
    fn highlight_outside_every_literal_is_null() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let opening = column(HOME_DISK, "`");
        let closing = HOME_DISK
            .rfind('`')
            .expect("the fixture has a closing backtick") as u32;
        let end = HOME_DISK.len() as u32;
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, 1), None);
        assert_eq!(
            highlights(&mut state, &one, HOME_URI, 0, column(HOME_DISK, "iso(")),
            None
        );
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, opening), None);
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, closing), None);
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, end), None);
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, end + 1), None);
        assert_eq!(highlights(&mut state, &one, HOME_URI, 1, 0), None);
    }

    #[test]
    fn highlight_of_a_path_without_text_is_null() {
        let mut state = with_config();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(highlights(&mut state, &one, HOME_URI, 0, 0), None);
    }

    #[test]
    fn highlight_in_two_literals_is_each_literals_token() {
        let text = format!("{ENTRYPOINT_A}\n{FIELD_X}");
        assert_eq!(
            highlights_on_disk(&text, 0, column(ENTRYPOINT_A, "Query")),
            highlight_of(0, ENTRYPOINT_A, "Query")
        );
        assert_eq!(
            highlights_on_disk(&text, 1, column(FIELD_X, "id")),
            highlight_of(1, FIELD_X, "id")
        );
        let one_line = format!("{ENTRYPOINT_A} {FIELD_X}");
        assert_eq!(
            highlights_on_disk(&one_line, 0, column(&one_line, "id")),
            highlight_of(0, &one_line, "id")
        );
    }

    #[test]
    fn highlight_after_a_wide_char_is_in_utf16_columns() {
        let text = format!("const s = \"😀\"; {ENTRYPOINT_A}");
        let start = column(&text, "Query");
        assert_eq!(
            start,
            text.find("Query").expect("the fixture has Query") as u32 - 2
        );
        assert_eq!(
            highlights_on_disk(&text, 0, start),
            text_highlight(0, start, start + 5)
        );
    }

    #[test]
    fn highlight_reads_the_clients_buffer_and_another_client_reads_disk() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        let two = live_client(&mut state, ClientId(2));
        handle(&mut state, did_open(one.client, HOME_URI, FIELD_X));
        let id = column(FIELD_X, "id");
        assert_eq!(
            highlights(&mut state, &one, HOME_URI, 0, id),
            highlight_of(0, FIELD_X, "id")
        );
        assert_eq!(highlights(&mut state, &two, HOME_URI, 0, id), None);
        assert_eq!(
            highlights(&mut state, &two, HOME_URI, 0, column(HOME_DISK, "Query")),
            highlight_of(0, HOME_DISK, "Query")
        );
    }

    #[test]
    fn highlight_of_a_non_file_uri_is_null() {
        let mut state = with_disk_home();
        let one = live_client(&mut state, ClientId(1));
        assert_eq!(
            highlights(&mut state, &one, "untitled:Untitled-1", 0, 0),
            None
        );
    }

    #[test]
    fn hover_from_a_client_that_never_started_returns_no_effects() {
        let mut state = with_disk_home();
        let effects = handle(
            &mut state,
            lsp_request(
                ClientId(1),
                lsp_server::RequestId::from(1),
                lsp_types::request::HoverRequest::METHOD,
                hover_params(HOME_URI, 0, column(HOME_DISK, "Query")),
            ),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn file_absent_of_a_directory_path_does_not_remove_files_under_it() {
        let mut state = with_config();
        handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src"),
                presence: Presence::Absent,
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert_eq!(
            contents(state.reference(), interned("src/a.ts")),
            "a".wrap_some()
        );
    }

    fn event_notification(client: ClientId, params: serde_json::Value) -> IsographEvent {
        LspEvent::Notification(LspNotification {
            client,
            notification: lsp_server::Notification {
                method: crate::lsp_socket::IsographEventNotification::METHOD.to_owned(),
                params,
            },
        })
        .to()
    }

    fn start_a_client(state: &mut IsographState<TestProfile>) -> ClientId {
        let client = ClientId(1);
        let (tx, _rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
        client
    }

    #[test]
    fn lsp_isograph_event_hello_world_returns_log_hello_world() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }

    #[test]
    fn lsp_isograph_event_quit_returns_kill() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::Quit).expect("serializes"),
            ),
        );
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn lsp_isograph_event_disk_changed_interns() {
        let mut state = with_config();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::DiskChanged(DiskChanged::File(
                    DiskFileChanged {
                        path: PathBuf::from("/tmp/proj/src/a.ts"),
                        presence: Presence::Present("export const a = 1;\n".to_owned()),
                    },
                )))
                .expect("serializes"),
            ),
        );
        assert!(effects.is_empty());
        assert_eq!(
            contents(state.reference(), interned("src/a.ts")),
            "export const a = 1;\n".wrap_some()
        );
    }

    #[test]
    fn lsp_unknown_notification_returns_no_effects() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            LspEvent::Notification(LspNotification {
                client,
                notification: lsp_server::Notification {
                    method: "window/logMessage".to_owned(),
                    params: serde_json::json!({}),
                },
            })
            .to(),
        );
        assert!(effects.is_empty());
    }

    fn lsp_response(client: ClientId) -> IsographEvent {
        LspEvent::Response(LspResponse {
            client,
            response: lsp_server::Response {
                id: lsp_server::RequestId::from(1),
                result: None,
                error: None,
            },
        })
        .to()
    }

    #[test]
    fn lsp_response_from_a_live_client_returns_no_effects() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(&mut state, lsp_response(client));
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_response_from_a_client_that_never_started_returns_no_effects() {
        let mut state = empty();
        let effects = handle(&mut state, lsp_response(ClientId(1)));
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_isograph_event_bad_params_returns_no_effects() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(client, serde_json::json!({"kind":"Nope"})),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_notification_from_a_client_that_never_started_returns_no_effects() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            event_notification(
                ClientId(1),
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        assert!(effects.is_empty());
        assert!(state.diagnostics_debounce.is_none());
    }

    #[test]
    fn lsp_notification_disk_changed_from_a_client_that_never_started_does_not_intern() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            event_notification(
                ClientId(1),
                serde_json::to_value(InternalEvent::DiskChanged(DiskChanged::File(
                    DiskFileChanged {
                        path: PathBuf::from("/tmp/proj/src/a.ts"),
                        presence: Presence::Present("export const a = 1;\n".to_owned()),
                    },
                )))
                .expect("serializes"),
            ),
        );
        assert!(effects.is_empty());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
    }

    #[test]
    fn lsp_notification_after_gone_returns_no_effects() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        handle(&mut state, ClientConnectionEvent::Gone(client).to());
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        assert!(effects.is_empty());
        assert!(state.diagnostics_debounce.is_none());
    }

    fn dummy_session(
        sender: crossbeam::channel::Sender<lsp_server::Message>,
        capabilities: lsp_types::ClientCapabilities,
    ) -> LiveSession {
        LiveSession {
            sender,
            capabilities,
            published_diagnostics: PublishedDiagnostics::default(),
            outstanding: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn client_started_stores_the_session() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, rx) = crossbeam::channel::unbounded();
        let capabilities = lsp_types::ClientCapabilities::default();
        let effects = handle(
            &mut state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(tx, capabilities.clone()),
            })
            .to(),
        );
        assert!(effects.is_empty());
        let session = state
            .live_sessions
            .0
            .get(&client)
            .expect("Started inserts a row");
        assert_eq!(session.capabilities, capabilities);
        assert_eq!(
            session.published_diagnostics,
            PublishedDiagnostics::default()
        );
        session
            .sender
            .send(lsp_server::Message::Notification(
                lsp_server::Notification {
                    method: "textDocument/publishDiagnostics".to_owned(),
                    params: serde_json::json!({}),
                },
            ))
            .expect("the stored sender is live");
        let lsp_server::Message::Notification(received) =
            rx.recv().expect("the test receives the notification")
        else {
            panic!("the stored sender carries Message::Notification");
        };
        assert_eq!(received.method, "textDocument/publishDiagnostics");
        assert_eq!(received.params, serde_json::json!({}));
    }

    #[test]
    fn client_gone_removes_the_session() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx, _rx) = crossbeam::channel::unbounded();
        handle(
            &mut state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(tx, lsp_types::ClientCapabilities::default()),
            })
            .to(),
        );
        let effects = handle(&mut state, ClientConnectionEvent::Gone(client).to());
        assert!(effects.is_empty());
        assert!(!state.live_sessions.0.contains_key(&client));
    }

    #[test]
    fn client_gone_of_an_unknown_id_is_a_noop() {
        let mut state = empty();
        let effects = handle(&mut state, ClientConnectionEvent::Gone(ClientId(1)).to());
        assert!(effects.is_empty());
        assert!(state.live_sessions.0.is_empty());
    }

    #[test]
    fn client_started_twice_for_the_same_id_keeps_the_second_session() {
        let mut state = empty();
        let client = ClientId(1);
        let (tx1, rx1) = crossbeam::channel::unbounded();
        let (tx2, rx2) = crossbeam::channel::unbounded();
        handle(
            &mut state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(tx1, lsp_types::ClientCapabilities::default()),
            })
            .to(),
        );
        let effects = handle(
            &mut state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(tx2, lsp_types::ClientCapabilities::default()),
            })
            .to(),
        );
        assert!(effects.is_empty());
        assert_eq!(state.live_sessions.0.len(), 1);
        let session = state
            .live_sessions
            .0
            .get(&client)
            .expect("Started twice keeps one row");
        let notification = lsp_server::Message::Notification(lsp_server::Notification {
            method: "textDocument/publishDiagnostics".to_owned(),
            params: serde_json::json!({}),
        });
        session
            .sender
            .send(notification)
            .expect("the second sender is stored");
        assert!(
            matches!(
                rx2.recv().expect("the second session receives"),
                lsp_server::Message::Notification(_)
            ),
            "the second sender receives the notification"
        );
        assert!(rx1.try_recv().is_err(), "the first session was replaced");
        assert!(session.outstanding.is_empty());
    }

    fn insert_live_session(
        sessions: &mut LiveSessions,
        client: ClientId,
    ) -> crossbeam::channel::Receiver<lsp_server::Message> {
        let (tx, rx) = crossbeam::channel::unbounded();
        sessions.0.insert(
            client,
            dummy_session(tx, lsp_types::ClientCapabilities::default()),
        );
        rx
    }

    fn perform_send(effect: crate::effect::IsographEffect) {
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = crate::daemon::perform(
            &crate::effect::SideEffectToken::mint(),
            effect,
            event_tx.reference(),
        );
    }

    #[test]
    fn reply_of_a_client_that_is_not_in_the_map_is_none() {
        let mut sessions = LiveSessions::default();
        assert!(
            sessions
                .reply(ClientId(1), lsp_server::RequestId::from(1))
                .is_none()
        );
    }

    #[test]
    fn reply_then_after_compute_of_a_live_client_sends_the_closed_over_id() {
        let mut sessions = LiveSessions::default();
        let client = ClientId(1);
        let id = lsp_server::RequestId::from(1);
        let rx = insert_live_session(&mut sessions, client);
        let before = sessions
            .reply(client, id.clone())
            .expect("reply of a live client is Some");
        assert!(
            sessions
                .0
                .get(&client)
                .expect("the client is live")
                .outstanding
                .contains(&id)
        );
        let after = sessions
            .before_compute(before)
            .expect("before_compute of an outstanding id is Some");
        let send = sessions
            .after_compute(after, serde_json::json!([]).wrap_ok())
            .expect("after_compute of an outstanding id is Some");
        assert!(
            sessions
                .0
                .get(&client)
                .expect("the client is live")
                .outstanding
                .is_empty()
        );
        perform_send(crate::effect::IsographEffect::SendLspResponse(
            crate::effect::SendLspResponseEffect { send }.boxed(),
        ));
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        assert_eq!(response.id, id);
        assert_eq!(response.result, serde_json::json!([]).wrap_some());
        assert!(response.error.is_none());
    }

    #[test]
    fn before_compute_after_removing_the_client_is_none() {
        let mut sessions = LiveSessions::default();
        let client = ClientId(1);
        let rx = insert_live_session(&mut sessions, client);
        let before = sessions
            .reply(client, lsp_server::RequestId::from(1))
            .expect("reply of a live client is Some");
        sessions.0.remove(&client);
        assert!(sessions.before_compute(before).is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn after_compute_after_removing_the_client_is_none() {
        let mut sessions = LiveSessions::default();
        let client = ClientId(1);
        let rx = insert_live_session(&mut sessions, client);
        let before = sessions
            .reply(client, lsp_server::RequestId::from(1))
            .expect("reply of a live client is Some");
        let after = sessions
            .before_compute(before)
            .expect("before_compute of an outstanding id is Some");
        sessions.0.remove(&client);
        assert!(
            sessions
                .after_compute(after, serde_json::json!([]).wrap_ok())
                .is_none()
        );
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn before_compute_after_removing_the_id_is_none() {
        let mut sessions = LiveSessions::default();
        let client = ClientId(1);
        let id = lsp_server::RequestId::from(1);
        let rx = insert_live_session(&mut sessions, client);
        let before = sessions
            .reply(client, id.clone())
            .expect("reply of a live client is Some");
        sessions
            .0
            .get_mut(&client)
            .expect("the client is live")
            .outstanding
            .remove(&id);
        assert!(sessions.before_compute(before).is_none());
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn after_compute_after_removing_the_id_is_none() {
        let mut sessions = LiveSessions::default();
        let client = ClientId(1);
        let id = lsp_server::RequestId::from(1);
        let rx = insert_live_session(&mut sessions, client);
        let before = sessions
            .reply(client, id.clone())
            .expect("reply of a live client is Some");
        let after = sessions
            .before_compute(before)
            .expect("before_compute of an outstanding id is Some");
        sessions
            .0
            .get_mut(&client)
            .expect("the client is live")
            .outstanding
            .remove(&id);
        assert!(
            sessions
                .after_compute(after, serde_json::json!([]).wrap_ok())
                .is_none()
        );
        assert!(rx.try_recv().is_err());
    }

    fn diagnostics_capabilities() -> lsp_types::ClientCapabilities {
        lsp_types::ClientCapabilities {
            text_document: lsp_types::TextDocumentClientCapabilities {
                publish_diagnostics: lsp_types::PublishDiagnosticsClientCapabilities::default()
                    .wrap_some(),
                ..Default::default()
            }
            .wrap_some(),
            ..Default::default()
        }
    }

    fn start_with_capabilities(
        state: &mut IsographState<TestProfile>,
        client: ClientId,
        capabilities: lsp_types::ClientCapabilities,
    ) -> crossbeam::channel::Receiver<lsp_server::Message> {
        let (tx, rx) = crossbeam::channel::unbounded();
        let effects = handle(
            state,
            ClientConnectionEvent::Started(ClientStartedEvent {
                client,
                session: dummy_session(tx, capabilities),
            })
            .to(),
        );
        assert!(effects.is_empty());
        rx
    }

    fn diagnostics_client(
        state: &mut IsographState<TestProfile>,
        client: ClientId,
    ) -> crossbeam::channel::Receiver<lsp_server::Message> {
        start_with_capabilities(state, client, diagnostics_capabilities())
    }

    fn only_timer(effects: Vec<IsographEffect>) -> freddie_timer::TimerEffect<TimerPayload> {
        let [IsographEffect::Timer(timer)] = <[IsographEffect; 1]>::try_from(effects)
            .expect("the arming event returns exactly one effect")
        else {
            panic!("the arming event returns a Timer");
        };
        timer
    }

    fn fire(
        state: &mut IsographState<TestProfile>,
        effects: Vec<IsographEffect>,
    ) -> Vec<IsographEffect> {
        handle(state, IsographEvent::Timer(only_timer(effects).event))
    }

    /// Performs every effect, then every notification `rx` received, keyed by URI.
    fn published(
        effects: Vec<IsographEffect>,
        rx: &crossbeam::channel::Receiver<lsp_server::Message>,
    ) -> HashMap<String, Vec<lsp_types::Diagnostic>> {
        perform_all(effects);
        drained(rx)
    }

    fn perform_all(effects: Vec<IsographEffect>) {
        effects.into_iter().for_each(perform_send);
    }

    /// Every notification `rx` received, keyed by URI.
    fn drained(
        rx: &crossbeam::channel::Receiver<lsp_server::Message>,
    ) -> HashMap<String, Vec<lsp_types::Diagnostic>> {
        rx.try_iter()
            .map(|message| {
                let lsp_server::Message::Notification(notification) = message else {
                    panic!("diagnostics arrive as notifications");
                };
                assert_eq!(
                    notification.method,
                    <lsp_types::notification::PublishDiagnostics as Notification>::METHOD
                );
                let params: lsp_types::PublishDiagnosticsParams =
                    serde_json::from_value(notification.params)
                        .expect("the notification carries PublishDiagnosticsParams");
                assert_eq!(params.version, None);
                (params.uri.as_str().to_owned(), params.diagnostics)
            })
            .collect()
    }

    fn change(client: ClientId, uri: &str, text: &str) -> IsographEvent {
        did_change(client, uri, serde_json::json!([{ "text": text }]))
    }

    const BROKEN: &str = "iso(`entrypoint`)";
    const EMPTY_LITERAL: &str = "iso(` `)";
    const CLEAN: &str = "iso(`entity Query`)";

    fn empty_literal_message() -> String {
        isograph_parser::ParseError::Ast(isograph_parser::AstError::EmptyLiteral).to_string()
    }

    #[test]
    fn did_open_of_a_broken_literal_publishes_its_errors_when_the_timer_fires() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        let effects = fire(&mut state, armed);
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::PublishDiagnostics(_)]
        ));
        let sent = published(effects, rx.reference());
        assert_eq!(sent.len(), 1);
        let diagnostics = sent.get(X_URI).expect("the open file is published");
        assert!(!diagnostics.is_empty());
        assert!(diagnostics.iter().all(|diagnostic| {
            diagnostic.severity == lsp_types::DiagnosticSeverity::ERROR.wrap_some()
                && diagnostic.source == "isograph".to_owned().wrap_some()
                && diagnostic.code.is_none()
        }));
    }

    #[test]
    fn diagnostic_columns_are_utf16_code_units() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(
            &mut state,
            did_open(
                ClientId(1),
                X_URI,
                "const é = 1;\nconst 😀 = iso(`entrypoint`)",
            ),
        );
        let effects = fire(&mut state, armed);
        let sent = published(effects, rx.reference());
        let parsed = isograph_parser::parse_iso_literal("entrypoint");
        let first = sent.get(X_URI).expect("the open file is published")[0]
            .range
            .start;
        assert_eq!(first.line, 1);
        assert_eq!(
            first.character,
            "const 😀 = iso(`".encode_utf16().count() as u32 + parsed.errors[0].location.start
        );
    }

    #[test]
    fn crlf_before_the_literal_puts_its_diagnostics_on_line_one() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(
            &mut state,
            did_open(ClientId(1), X_URI, "const a = 1;\r\niso(`entrypoint`)"),
        );
        let effects = fire(&mut state, armed);
        let sent = published(effects, rx.reference());
        let diagnostics = sent.get(X_URI).expect("the open file is published");
        assert!(!diagnostics.is_empty());
        assert!(
            diagnostics
                .iter()
                .all(|diagnostic| diagnostic.range.start.line == 1)
        );
    }

    #[test]
    fn a_file_with_no_literals_publishes_nothing() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, "const a = 1;"));
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn a_whitespace_only_literal_publishes_the_empty_literal_error() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, EMPTY_LITERAL));
        let effects = fire(&mut state, armed);
        let sent = published(effects, rx.reference());
        let diagnostics = sent.get(X_URI).expect("the open file is published");
        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            empty_literal_message().wrap_vec()
        );
    }

    #[test]
    fn an_open_file_selecting_an_undefined_field_publishes_that_validation_error() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let text = "iso(`entity Query`) iso(`field Query.x { nope }`)";
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, text));
        let effects = fire(&mut state, armed);
        let start = text.find("nope").expect("the fixture selects nope") as u32;
        assert_eq!(
            published(effects, rx.reference()),
            HashMap::from([(
                X_URI.to_owned(),
                lsp_types::Diagnostic {
                    range: lsp_types::Range::new(
                        lsp_types::Position::new(0, start),
                        lsp_types::Position::new(0, start + 4)
                    ),
                    severity: lsp_types::DiagnosticSeverity::ERROR.wrap_some(),
                    source: "isograph".to_owned().wrap_some(),
                    message: "Field `Query.nope` is not defined.".to_owned(),
                    ..Default::default()
                }
                .wrap_vec()
            )])
        );
    }

    #[test]
    fn a_literal_with_only_a_host_language_problem_publishes_nothing() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(
            &mut state,
            did_open(ClientId(1), X_URI, "iso`entity Query`"),
        );
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn fixing_a_published_file_publishes_an_empty_list_once() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        let effects = fire(&mut state, armed);
        assert!(!published(effects, rx.reference())[X_URI].is_empty());
        let armed = handle(&mut state, change(ClientId(1), X_URI, CLEAN));
        let effects = fire(&mut state, armed);
        assert_eq!(
            published(effects, rx.reference()),
            HashMap::from([(X_URI.to_owned(), Vec::new())])
        );
        let armed = handle(&mut state, change(ClientId(1), X_URI, CLEAN));
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn breaking_a_clean_file_publishes_its_errors() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, CLEAN));
        assert!(fire(&mut state, armed).is_empty());
        let armed = handle(&mut state, change(ClientId(1), X_URI, BROKEN));
        let effects = fire(&mut state, armed);
        assert!(!published(effects, rx.reference())[X_URI].is_empty());
    }

    #[test]
    fn an_identical_change_after_a_publish_publishes_nothing() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        let effects = fire(&mut state, armed);
        assert_eq!(published(effects, rx.reference()).len(), 1);
        let armed = handle(&mut state, change(ClientId(1), X_URI, BROKEN));
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn two_changes_before_a_fire_publish_the_second_text_once() {
        let mut state = with_config();
        let rx = diagnostics_client(&mut state, ClientId(1));
        let first = only_timer(handle(&mut state, did_open(ClientId(1), X_URI, BROKEN)));
        let second = handle(&mut state, change(ClientId(1), X_URI, EMPTY_LITERAL));
        assert!(handle(&mut state, IsographEvent::Timer(first.event)).is_empty());
        let effects = fire(&mut state, second);
        let sent = published(effects, rx.reference());
        assert_eq!(
            sent[X_URI]
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            empty_literal_message().wrap_vec()
        );
    }

    #[test]
    fn the_same_firing_delivered_twice_publishes_once() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        let timer = only_timer(handle(&mut state, did_open(ClientId(1), X_URI, BROKEN)));
        assert_eq!(
            handle(&mut state, IsographEvent::Timer(timer.event)).len(),
            1
        );
        assert!(handle(&mut state, IsographEvent::Timer(timer.event)).is_empty());
    }

    #[test]
    fn closing_a_published_file_publishes_an_empty_list() {
        for disk in [None, CLEAN.wrap_some(), BROKEN.wrap_some()] {
            let mut state = with_config();
            if let Some(disk) = disk {
                state
                    .db
                    .insert_disk_file(interned("src/x.ts"), SOURCE_FILE, disk.to_owned());
            }
            let rx = diagnostics_client(&mut state, ClientId(1));
            let armed = handle(&mut state, did_open(ClientId(1), X_URI, EMPTY_LITERAL));
            let effects = fire(&mut state, armed);
            assert_eq!(published(effects, rx.reference()).len(), 1, "{disk:?}");
            let armed = handle(&mut state, did_close(ClientId(1), X_URI));
            let effects = fire(&mut state, armed);
            assert_eq!(
                published(effects, rx.reference()),
                HashMap::from([(X_URI.to_owned(), Vec::new())]),
                "{disk:?}"
            );
        }
    }

    #[test]
    fn a_broken_disk_only_file_publishes_nothing() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        let effects = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/y.ts"),
                presence: Presence::Present(BROKEN.to_owned()),
            }))
            .to(),
        );
        assert!(effects.is_empty());
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, CLEAN));
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn disk_changed_does_not_arm_the_debounce() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        state.diagnostics_debounce = None;
        let present = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/x.ts"),
                presence: Presence::Present(CLEAN.to_owned()),
            }))
            .to(),
        );
        let absent = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/x.ts"),
                presence: Presence::Absent,
            }))
            .to(),
        );
        let folder_removed = handle(
            &mut state,
            InternalEvent::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
                path: PathBuf::from("/tmp/proj/src"),
            }))
            .to(),
        );
        assert!(present.is_empty());
        assert!(absent.is_empty());
        assert!(folder_removed.is_empty());
        assert!(state.diagnostics_debounce.is_none());
    }

    #[test]
    fn started_does_not_arm_the_debounce() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        assert!(state.diagnostics_debounce.is_none());
    }

    #[test]
    fn open_files_outside_the_globs_or_skipped_by_the_host_publish_nothing() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        handle(
            &mut state,
            did_open(ClientId(1), "file:///tmp/proj/other/x.ts", BROKEN),
        );
        let armed = handle(
            &mut state,
            did_open(ClientId(1), "file:///tmp/proj/src/README.md", BROKEN),
        );
        assert!(fire(&mut state, armed).is_empty());
    }

    #[test]
    fn two_clients_with_one_path_each_receive_diagnostics_of_their_own_buffer() {
        let mut state = with_config();
        let rx_1 = diagnostics_client(&mut state, ClientId(1));
        let rx_2 = diagnostics_client(&mut state, ClientId(2));
        handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        let armed = handle(&mut state, did_open(ClientId(2), X_URI, EMPTY_LITERAL));
        let effects = fire(&mut state, armed);
        assert_eq!(effects.len(), 2);
        let expected_1 = isograph_lsp::lsp_diagnostics_for_file(
            state.db.reference(),
            ClientId(1),
            interned("src/x.ts"),
        )
        .clone()
        .expect("client 1 has a buffer");
        perform_all(effects);
        let sent_1 = drained(rx_1.reference());
        let sent_2 = drained(rx_2.reference());
        assert_eq!(sent_1, HashMap::from([(X_URI.to_owned(), expected_1)]));
        assert!(
            sent_1[X_URI]
                .iter()
                .all(|diagnostic| diagnostic.message != empty_literal_message())
        );
        assert_eq!(
            sent_2[X_URI]
                .iter()
                .map(|diagnostic| diagnostic.message.clone())
                .collect::<Vec<_>>(),
            empty_literal_message().wrap_vec()
        );
        let armed = handle(&mut state, change(ClientId(2), X_URI, CLEAN));
        let effects = fire(&mut state, armed);
        assert_eq!(effects.len(), 1);
        perform_all(effects);
        assert_eq!(
            drained(rx_2.reference()),
            HashMap::from([(X_URI.to_owned(), Vec::new())])
        );
        assert!(matches!(
            rx_1.try_recv(),
            Err(crossbeam::channel::TryRecvError::Empty)
        ));
    }

    #[test]
    fn another_clients_open_file_is_not_published_to_a_client() {
        let mut state = with_config();
        let rx_1 = diagnostics_client(&mut state, ClientId(1));
        let rx_2 = diagnostics_client(&mut state, ClientId(2));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        let effects = fire(&mut state, armed);
        assert_eq!(effects.len(), 1);
        perform_all(effects);
        assert_eq!(drained(rx_1.reference()).len(), 1);
        assert!(matches!(
            rx_2.try_recv(),
            Err(crossbeam::channel::TryRecvError::Empty)
        ));
    }

    #[test]
    fn a_client_gone_before_the_fire_receives_nothing() {
        let mut state = with_config();
        let _rx_1 = diagnostics_client(&mut state, ClientId(1));
        let rx_2 = diagnostics_client(&mut state, ClientId(2));
        handle(&mut state, did_open(ClientId(2), X_URI, BROKEN));
        let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
        handle(&mut state, ClientConnectionEvent::Gone(ClientId(1)).to());
        let effects = fire(&mut state, armed);
        assert_eq!(effects.len(), 1);
        assert_eq!(published(effects, rx_2.reference()).len(), 1);
    }

    #[test]
    fn a_client_without_the_publish_diagnostics_capability_receives_nothing() {
        let without_text_document = lsp_types::ClientCapabilities::default();
        let without_publish_diagnostics = lsp_types::ClientCapabilities {
            text_document: lsp_types::TextDocumentClientCapabilities::default().wrap_some(),
            ..Default::default()
        };
        for capabilities in [without_text_document, without_publish_diagnostics] {
            let mut state = with_config();
            let rx = start_with_capabilities(&mut state, ClientId(1), capabilities);
            let armed = handle(&mut state, did_open(ClientId(1), X_URI, BROKEN));
            assert!(fire(&mut state, armed).is_empty());
            assert!(matches!(
                rx.try_recv(),
                Err(crossbeam::channel::TryRecvError::Empty)
            ));
        }
    }

    #[test]
    fn rejected_document_notifications_do_not_arm_the_debounce() {
        let mut state = with_config();
        let _rx = diagnostics_client(&mut state, ClientId(1));
        let ranged = serde_json::json!([{
            "range": {
                "start": { "line": 0, "character": 0 },
                "end": { "line": 0, "character": 1 }
            },
            "text": "ranged"
        }]);
        let effects = [
            handle(
                &mut state,
                did_open(ClientId(1), "https://example.net/a.ts", BROKEN),
            ),
            handle(&mut state, did_change(ClientId(1), X_URI, ranged)),
            handle(&mut state, did_open(ClientId(9), X_URI, BROKEN)),
        ];
        assert!(effects.iter().all(Vec::is_empty));
        assert!(state.diagnostics_debounce.is_none());
    }
}
