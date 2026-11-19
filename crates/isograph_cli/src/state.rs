use std::ops::ControlFlow;
use std::path::Path;
use std::time::Duration;

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathError, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Intern;
use isograph_compiler::{ClientId, HostLanguage};
use pico::Database;
use prelude::Postfix;

use crate::effect::{IsographEffect, no_effects};
use crate::event::{
    ClientConnectionEvent, DiskChanged, InternalEvent, IsographEvent, LiveSessions, Presence,
};
use crate::timer::TimerPayload;

pub use isograph_compiler::IsographDb;

pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub live_sessions: LiveSessions,
    pub(crate) hello_world_debounce: Option<freddie_timer::TimerGuard>,
}

pub fn handle<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal_event(state, internal),
        IsographEvent::Timer(fired) => handle_timer_fired(state, fired),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
}

fn handle_client_connection<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
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

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
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

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
    before: crate::event::BeforeCompute,
) -> Vec<IsographEffect> {
    let Some(after) = state.live_sessions.before_compute(before) else {
        return no_effects();
    };
    let get_response = || {
        let request = crate::lsp_request_dispatch::LSPRequestDispatch::new(incoming, &state.db)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                crate::adapter::semantic_tokens_response::<THostLanguage>,
            )?
            .on_request_sync::<lsp_types::request::Formatting>(
                crate::adapter::formatting_response::<THostLanguage>,
            )?
            .on_request_sync::<lsp_types::request::WillSaveWaitUntil>(
                crate::adapter::will_save_wait_until_response::<THostLanguage>,
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

fn handle_internal_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => handle_hello_world(state),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(&mut state.db, change),
    }
}

fn handle_hello_world<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) =
        freddie_timer::timer_effect_and_guard(Duration::ZERO, TimerPayload::HelloWorldDebounced);
    state.hello_world_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

fn handle_timer_fired<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.hello_world_debounce.as_ref()) {
        Some(&TimerPayload::HelloWorldDebounced) => {
            state.hello_world_debounce = None;
            IsographEffect::LogHelloWorld.wrap_vec()
        }
        None => no_effects(),
    }
}

fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: crate::event::LspNotification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event_notification::<THostLanguage>,
            )?
            .on_notification_sync::<lsp_types::notification::DidOpenTextDocument>(
                crate::adapter::on_did_open::<THostLanguage>,
            )?
            .on_notification_sync::<lsp_types::notification::DidChangeTextDocument>(
                crate::adapter::on_did_change::<THostLanguage>,
            )?
            .on_notification_sync::<lsp_types::notification::DidCloseTextDocument>(
                crate::adapter::on_did_close::<THostLanguage>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}

fn on_isograph_event_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    _client: ClientId,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    handle_internal_event(state, internal)
}

pub(crate) fn intern_config_directory(
    state: &mut IsographDb<impl HostLanguage>,
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
    state: &IsographDb<impl HostLanguage>,
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

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(state, change)
        .unwrap_or_else(|error| tracing::warn!(%error, "DiskChanged path"));
    no_effects()
}

fn apply_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    change: DiskChanged,
) -> Result<(), DiskChangedPathError> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path)?;
            match change.presence {
                Presence::Present(contents) => state.insert_disk_file(path, contents),
                Presence::Absent => state.remove_disk_file(path),
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path)?;
            state.remove_disk_files_from_path(path);
        }
    }
    ().wrap_ok()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use intern::string_key::Intern;
    use lsp_types::notification::Notification;
    use lsp_types::request::Request;
    use pico::Database;
    use prelude::Postfix;

    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{IsographDb, IsographState, handle, intern_config_directory};
    use crate::effect::IsographEffect;
    use crate::event::{
        ClientConnectionEvent, ClientStartedEvent, DiskChanged, DiskFileChanged, FolderRemoved,
        InternalEvent, IsographEvent, LiveSession, LiveSessions, LspEvent, LspNotification,
        LspResponse, Presence,
    };
    use isograph_compiler::ClientId;

    fn interned(s: &str) -> common_lang_types::RelativePathToSourceFile {
        s.intern().to()
    }

    fn empty() -> IsographState<TypeScriptHostLanguage> {
        IsographState {
            db: IsographDb::<TypeScriptHostLanguage>::default(),
            live_sessions: LiveSessions::default(),
            hello_world_debounce: None,
        }
    }

    fn with_config() -> IsographState<TypeScriptHostLanguage> {
        let mut state = empty();
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn contents(
        state: &IsographState<TypeScriptHostLanguage>,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<&str> {
        state
            .db
            .get_disk_file_map()
            .untracked()
            .0
            .get(&path)
            .map(|id| state.db.get(*id).contents.as_str())
    }

    #[test]
    fn hello_world_returns_a_timer() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        assert!(matches!(effects.as_slice(), [IsographEffect::Timer(_)]));
        assert!(state.hello_world_debounce.is_some());
    }

    #[test]
    fn matching_hello_world_debounced_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
        assert!(state.hello_world_debounce.is_none());
    }

    #[test]
    fn stale_hello_world_debounced_returns_no_effects() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        state.hello_world_debounce = None;
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
        assert!(effects.is_empty());
    }

    #[test]
    fn a_second_hello_world_drops_the_first_guard() {
        let mut state = empty();
        let first = handle(&mut state, InternalEvent::HelloWorld.to());
        let second = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(first_timer) = first
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(first_timer.event));
        assert!(effects.is_empty());
        let IsographEffect::Timer(second_timer) = second
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(second_timer.event));
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
        assert!(state.hello_world_debounce.is_none());
    }

    #[test]
    fn timer_fired_on_a_never_armed_state_returns_no_effects() {
        let mut armed = empty();
        let effects = handle(&mut armed, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let mut state = empty();
        let effects = handle(&mut state, timer.event.to());
        assert!(effects.is_empty());
    }

    #[test]
    fn a_matching_hello_world_debounced_fed_twice_logs_once() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let first = handle(&mut state, timer.event.to());
        assert!(matches!(first.as_slice(), [IsographEffect::LogHelloWorld]));
        let second = handle(&mut state, timer.event.to());
        assert!(second.is_empty());
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
        state: &mut IsographState<TypeScriptHostLanguage>,
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
                "textDocument/hover",
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
            "No handler registered for method 'textDocument/hover'"
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

    fn with_disk_x() -> IsographState<TypeScriptHostLanguage> {
        let mut state = with_config();
        state
            .db
            .insert_disk_file(interned("src/x.ts"), "disk".to_owned());
        state
    }

    fn started(state: &mut IsographState<TypeScriptHostLanguage>, client: ClientId) {
        let (tx, _rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
    }

    fn read(
        state: &IsographState<TypeScriptHostLanguage>,
        client: ClientId,
        path: &str,
    ) -> Option<String> {
        isograph_compiler::file_contents(state.db.reference(), client, interned(path)).clone()
    }

    #[test]
    fn did_open_stores_the_buffer_for_that_client_only() {
        let mut state = with_disk_x();
        started(&mut state, ClientId(1));
        let effects = handle(&mut state, did_open(ClientId(1), X_URI, "open"));
        assert!(effects.is_empty());
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
        assert!(effects.is_empty());
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
        assert!(effects.is_empty());
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

    fn disk_changed(state: &mut IsographState<TypeScriptHostLanguage>, path: &str, text: &str) {
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

    fn live_client(
        state: &mut IsographState<TypeScriptHostLanguage>,
        client: ClientId,
    ) -> TestClient {
        let (tx, rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
        TestClient { client, rx }
    }

    fn request_result(
        state: &mut IsographState<TypeScriptHostLanguage>,
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
        state: &mut IsographState<TypeScriptHostLanguage>,
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
        state: &mut IsographState<TypeScriptHostLanguage>,
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
        state: &mut IsographState<TypeScriptHostLanguage>,
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

    fn with_disk_home() -> IsographState<TypeScriptHostLanguage> {
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
            &mut IsographState<TypeScriptHostLanguage>,
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

    fn start_a_client(state: &mut IsographState<TypeScriptHostLanguage>) -> ClientId {
        let client = ClientId(1);
        let (tx, _rx) = crossbeam::channel::unbounded();
        start_client(state, client, tx);
        client
    }

    #[test]
    fn lsp_isograph_event_hello_world_returns_a_timer() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        assert!(matches!(effects.as_slice(), [IsographEffect::Timer(_)]));
        assert!(state.hello_world_debounce.is_some());
    }

    #[test]
    fn lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world() {
        let mut state = empty();
        let client = start_a_client(&mut state);
        let effects = handle(
            &mut state,
            event_notification(
                client,
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("LSP HelloWorld returns one Timer")
        else {
            panic!("LSP HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
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
        assert!(state.hello_world_debounce.is_none());
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
        assert!(state.hello_world_debounce.is_none());
    }

    fn dummy_session(
        sender: crossbeam::channel::Sender<lsp_server::Message>,
        capabilities: lsp_types::ClientCapabilities,
    ) -> LiveSession {
        LiveSession {
            sender,
            capabilities,
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
}
