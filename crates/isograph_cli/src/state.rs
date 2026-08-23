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
use crate::event::{DiskChanged, IsographEvent, Presence};

pub use isograph_compiler::IsographState;

pub fn handle<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    event: IsographEvent,
) -> Vec<IsographEffect> {
    match event {
        IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
        IsographEvent::DiskChanged(change) => {
            handle_disk_changed(state, change);
            Vec::new()
        }
        IsographEvent::LspRequest(request) => method_not_found(request).wrap_vec(),
    }
}

fn method_not_found(request: crate::event::LspRequest) -> IsographEffect {
    let id = request.request.id.clone();
    IsographEffect::SendLspResponse(
        crate::effect::SendLspResponse {
            reply: request.reply,
            response: lsp_server::Response {
                id,
                result: None,
                error: lsp_server::ResponseError {
                    code: lsp_server::ErrorCode::MethodNotFound as i32,
                    data: None,
                    message: format!(
                        "No handler registered for method '{}'",
                        request.request.method
                    ),
                }
                .wrap_some(),
            },
        }
        .boxed(),
    )
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
) {
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

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use intern::string_key::Intern;
    use pico::Database;
    use prelude::Postfix;

    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{IsographState, handle, intern_config_directory};
    use crate::effect::IsographEffect;
    use crate::event::{DiskChanged, IsographEvent, Presence};

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
        let effects = handle(&mut state, IsographEvent::HelloWorld);
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, IsographEvent::Quit);
        assert!(matches!(effects.as_slice(), [IsographEffect::Kill]));
    }

    #[test]
    fn disk_changed_present_interns_a_path_relative_to_the_config() {
        let mut state = with_config();
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
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
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/other/a.ts"),
                presence: Presence::Present("outside".to_owned()),
            }),
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
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            }),
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
            crate::event::LspRequest {
                request: lsp_server::Request {
                    id: id.clone(),
                    method: "textDocument/hover".to_owned(),
                    params: serde_json::json!({}),
                },
                reply,
            }
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
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
    }
}
