use std::ops::ControlFlow;
use std::path::{Path, PathBuf};

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use intern::string_key::Intern;
use isograph_compiler::HostLanguage;
use pico::Database;
use prelude::Postfix;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, Internal, IsographEvent, Presence};

pub use isograph_compiler::IsographState;

pub fn handle<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
    }
}

fn dispatch_lsp<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    lsp: crate::event::Lsp,
) -> Vec<IsographEffect> {
    match lsp {
        crate::event::Lsp::Request(incoming) => dispatch_lsp_request(state, incoming),
        crate::event::Lsp::Notification(notification) => {
            dispatch_lsp_notification(state, notification)
        }
        crate::event::Lsp::Response(_response) => Vec::new(),
    }
}

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    incoming: crate::event::LspRequest,
) -> Vec<IsographEffect> {
    let crate::event::LspRequest { request, reply } = incoming;
    let get_response = || {
        let request =
            isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, state).request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => crate::effect::IsographEffect::SendLspResponse(
            crate::effect::SendLspResponse { reply, response }.boxed(),
        )
        .wrap_vec(),
        ControlFlow::Continue(request) => {
            method_not_found(crate::event::LspRequest { request, reply })
        }
    }
}

fn method_not_found(incoming: crate::event::LspRequest) -> Vec<IsographEffect> {
    // Immediate SendLspResponse this slice. Async later: a timer plus an event; handle of
    // that event is an immediate SendLspResponse.
    crate::effect::IsographEffect::SendLspResponse(
        crate::effect::SendLspResponse {
            reply: incoming.reply,
            response: lsp_server::Response {
                id: incoming.request.id,
                result: None,
                error: lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::MethodNotFound as i32,
                    data: None,
                    message: format!(
                        "No handler registered for method '{}'",
                        incoming.request.method
                    ),
                }
                .wrap_some(),
            },
        }
        .boxed(),
    )
    .wrap_vec()
}

fn handle_internal<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: Internal,
) -> Vec<IsographEffect> {
    match internal {
        Internal::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        Internal::Quit => IsographEffect::Kill.wrap_vec(),
        Internal::DiskChanged(change) => handle_disk_changed(state, change),
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
        ControlFlow::Continue(()) => Vec::new(),
    }
}

fn on_isograph_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: Internal,
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
    absolute: &PathBuf,
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
    Vec::new()
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use intern::string_key::Intern;
    use lsp_types::notification::Notification;
    use pico::Database;
    use prelude::Postfix;

    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{IsographState, handle, intern_config_directory};
    use crate::effect::IsographEffect;
    use crate::event::{
        DiskChanged, DiskFileChanged, FolderRemoved, Internal, IsographEvent, Lsp, Presence,
    };

    fn interned(s: &str) -> common_lang_types::RelativePathToSourceFile {
        s.intern().to()
    }

    fn with_config() -> IsographState<TypeScriptHostLanguage> {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        intern_config_directory(&mut state, Path::new("/tmp/proj/isograph.config.json"));
        state
    }

    fn contents(
        state: &IsographState<TypeScriptHostLanguage>,
        path: common_lang_types::RelativePathToSourceFile,
    ) -> Option<&str> {
        state
            .get_disk_file_map()
            .untracked()
            .0
            .get(&path)
            .map(|id| state.get(*id).contents.as_str())
    }

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, Internal::HelloWorld.to());
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, Internal::Quit.to());
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn disk_changed_present_interns_a_path_relative_to_the_config() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            }))
            .to(),
        );
        assert!(effects.is_empty());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
    }

    #[test]
    fn lsp_request_returns_method_not_found() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let (reply, rx) = crossbeam::channel::unbounded();
        let id = lsp_server::RequestId::from(1);
        let effects = handle(
            &mut state,
            Lsp::Request(crate::event::LspRequest {
                request: lsp_server::Request {
                    id: id.clone(),
                    method: "textDocument/hover".to_owned(),
                    params: serde_json::json!({}),
                },
                reply,
            })
            .to(),
        );
        assert_eq!(effects.len(), 1);
        let effect = effects
            .into_iter()
            .next()
            .expect("handle returned one effect");
        let crate::effect::IsographEffect::SendLspResponse(send) = effect else {
            panic!("handle of LspRequest returns SendLspResponse");
        };
        assert_eq!(send.response.id, id);
        assert!(send.response.result.is_none());
        let error = send
            .response
            .error
            .as_ref()
            .expect("MethodNotFound is an error");
        assert_eq!(error.code, lsp_server::ErrorCode::MethodNotFound as i32);
        assert_eq!(
            error.message,
            "No handler registered for method 'textDocument/hover'"
        );
        let _ = crate::daemon::perform(crate::effect::IsographEffect::SendLspResponse(send));
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
    }

    #[test]
    #[should_panic(
        expected = "CurrentWorkingDirectory is interned from the config path before DiskChanged"
    )]
    fn disk_changed_without_a_config_directory_panics() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/b.ts"),
                presence: Presence::Present("b".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src2/c.ts"),
                presence: Presence::Present("c".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src2/c.ts"),
                presence: Presence::Present("c".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::FolderRemoved(FolderRemoved {
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
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("a".to_owned()),
            }))
            .to(),
        );
        let effects = handle(
            &mut state,
            Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
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
        Lsp::Notification(lsp_server::Notification {
            method: crate::lsp_socket::IsographEventNotification::METHOD.to_owned(),
            params,
        })
        .to()
    }

    #[test]
    fn lsp_isograph_event_hello_world_returns_log_hello_world() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            event_notification(serde_json::to_value(Internal::HelloWorld).expect("serializes")),
        );
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }

    #[test]
    fn lsp_isograph_event_quit_returns_kill() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            event_notification(serde_json::to_value(Internal::Quit).expect("serializes")),
        );
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn lsp_isograph_event_disk_changed_interns() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            event_notification(
                serde_json::to_value(Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
                    path: PathBuf::from("/tmp/proj/src/a.ts"),
                    presence: Presence::Present("export const a = 1;\n".to_owned()),
                })))
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
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            Lsp::Notification(lsp_server::Notification {
                method: "window/logMessage".to_owned(),
                params: serde_json::json!({}),
            })
            .to(),
        );
        assert!(effects.is_empty());
    }

    #[test]
    fn lsp_response_returns_no_effects() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            Lsp::Response(lsp_server::Response {
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
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            event_notification(serde_json::json!({"kind":"Nope"})),
        );
        assert!(effects.is_empty());
    }
}
