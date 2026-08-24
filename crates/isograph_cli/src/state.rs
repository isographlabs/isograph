use std::ops::ControlFlow;
use std::path::Path;

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Intern;
use isograph_compiler::HostLanguage;
use pico::Database;
use prelude::Postfix;

use crate::effect::{IsographEffect, no_effects};
use crate::event::{
    ClientConnectionEvent, DiskChanged, InternalEvent, IsographEvent, LiveSessions, Presence,
};

pub use isograph_compiler::IsographState;

pub struct State<THostLanguage: HostLanguage> {
    pub db: IsographState<THostLanguage>,
    pub live_sessions: LiveSessions,
}

pub fn handle<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
}

fn handle_client_connection<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
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
            no_effects()
        }
    }
}

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    lsp: crate::event::LspEvent,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::LspEvent::Request(incoming) => {
            let client = incoming.client;
            let id = incoming.request.id.clone();
            {
                let Some(session) = state.live_sessions.0.get_mut(&client) else {
                    return no_effects();
                };
                session.outstanding.insert(id.clone());
            }
            dispatch_lsp_request(state, incoming, id)
        }
        crate::event::LspEvent::Notification(notification) => {
            dispatch_lsp_notification(&mut state.db, notification)
        }
        crate::event::LspEvent::Response(_response) => no_effects(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    incoming: crate::event::LspRequest,
    id: lsp_server::RequestId,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { client, request } = incoming;
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, &state.db)
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
        ControlFlow::Break(response) => send_lsp_response(state, client, id, response),
        ControlFlow::Continue(request) => method_not_found(state, client, id, request),
    }
}

fn send_lsp_response<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    response: lsp_server::Response,
) -> Vec<IsographEffect> {
    let Some(send) = state.live_sessions.take_reply(client, id) else {
        return no_effects();
    };
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponseEffect {
            send: (move |token: &crate::effect::SideEffectToken| {
                send(token, lsp_server::Message::Response(response))
            })
            .boxed(),
        }
        .boxed(),
    )
    .wrap_vec()
}

fn method_not_found<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    client: crate::event::ClientId,
    id: lsp_server::RequestId,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    send_lsp_response(
        state,
        client,
        id,
        lsp_server::Response {
            id: request.id,
            result: None,
            error: lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!("No handler registered for method '{}'", request.method),
            }
            .wrap_some(),
        },
    )
}

fn handle_internal<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(state, change),
    }
}

fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                on_isograph_event::<THostLanguage>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}

fn on_isograph_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    handle_internal(state, internal)
}

pub(crate) fn intern_config_directory(
    state: &mut IsographState<impl HostLanguage>,
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

fn relative_path_to_source_file(
    state: &IsographState<impl HostLanguage>,
    absolute: &Path,
) -> RelativePathToSourceFile {
    let cwd = *state
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("CurrentWorkingDirectory is interned from the config path before DiskChanged");
    relative_path_from_absolute_and_working_directory(cwd, absolute)
}

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    match change {
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path);
            match change.presence {
                Presence::Present(contents) => {
                    state.insert_disk_file(path, contents);
                }
                Presence::Absent => {
                    state.remove_disk_file(path);
                }
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path);
            state.remove_disk_files_from_path(path);
        }
    }
    no_effects()
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

    use super::{IsographState, State, handle, intern_config_directory};
    use crate::effect::IsographEffect;
    use crate::event::{
        ClientConnectionEvent, ClientId, ClientStartedEvent, DiskChanged, DiskFileChanged,
        FolderRemoved, InternalEvent, IsographEvent, LiveSession, LiveSessions, LspEvent, Presence,
    };

    fn interned(s: &str) -> common_lang_types::RelativePathToSourceFile {
        s.intern().to()
    }

    fn empty() -> State<TypeScriptHostLanguage> {
        State {
            db: IsographState::<TypeScriptHostLanguage>::default(),
            live_sessions: LiveSessions::default(),
        }
    }

    fn with_config() -> State<TypeScriptHostLanguage> {
        let mut state = empty();
        intern_config_directory(&mut state.db, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn contents(
        state: &State<TypeScriptHostLanguage>,
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
    fn hello_world_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
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
        state: &mut State<TypeScriptHostLanguage>,
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
        let _ = crate::daemon::perform(&crate::effect::SideEffectToken::mint(), effect);
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
    fn lsp_formatting_of_a_live_client_returns_an_empty_array() {
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
        let _ = crate::daemon::perform(&crate::effect::SideEffectToken::mint(), effect);
        let lsp_server::Message::Response(response) = rx.recv().expect("perform sends on reply")
        else {
            panic!("perform sends Message::Response");
        };
        assert_eq!(response.id, id);
        assert_eq!(response.result, serde_json::json!([]).wrap_some());
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

    fn event_notification(params: serde_json::Value) -> IsographEvent {
        LspEvent::Notification(lsp_server::Notification {
            method: crate::lsp_socket::IsographEventNotification::METHOD.to_owned(),
            params,
        })
        .to()
    }

    #[test]
    fn lsp_isograph_event_hello_world_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            event_notification(
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
        let effects = handle(
            &mut state,
            event_notification(serde_json::to_value(InternalEvent::Quit).expect("serializes")),
        );
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn lsp_isograph_event_disk_changed_interns() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            event_notification(
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
        let effects = handle(
            &mut state,
            LspEvent::Notification(lsp_server::Notification {
                method: "window/logMessage".to_owned(),
                params: serde_json::json!({}),
            })
            .to(),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_response_returns_no_effects() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            LspEvent::Response(lsp_server::Response {
                id: lsp_server::RequestId::from(1),
                result: None,
                error: None,
            })
            .to(),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_isograph_event_bad_params_returns_no_effects() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            event_notification(serde_json::json!({"kind":"Nope"})),
        );
        assert!(effects.is_empty());
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

    #[test]
    fn take_reply_of_an_id_that_is_not_outstanding_is_none() {
        let mut sessions = LiveSessions::default();
        let (tx, _rx) = crossbeam::channel::unbounded();
        let client = ClientId(1);
        sessions.0.insert(
            client,
            dummy_session(tx, lsp_types::ClientCapabilities::default()),
        );
        assert!(
            sessions
                .take_reply(client, lsp_server::RequestId::from(1))
                .is_none()
        );
    }

    #[test]
    fn take_reply_of_an_outstanding_id_is_some_once() {
        let mut sessions = LiveSessions::default();
        let (tx, _rx) = crossbeam::channel::unbounded();
        let client = ClientId(1);
        let id = lsp_server::RequestId::from(1);
        sessions.0.insert(
            client,
            dummy_session(tx, lsp_types::ClientCapabilities::default()),
        );
        sessions
            .0
            .get_mut(&client)
            .expect("the test inserted the session")
            .outstanding
            .insert(id.clone());
        assert!(sessions.take_reply(client, id.clone()).is_some());
        assert!(sessions.take_reply(client, id).is_none());
    }
}
