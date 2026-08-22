use isograph_compiler::HostLanguage;
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
    }
}

fn handle_disk_changed<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    change: DiskChanged,
) {
    match change.presence {
        Presence::Present(contents) => {
            state.insert_disk_file(change.path, contents);
        }
        Presence::Absent => {
            state.remove_disk_file(&change.path);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pico::Database;
    use prelude::Postfix;

    use isograph_compiler::{DiskFile, HostLanguage};
    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{IsographState, handle};
    use crate::effect::IsographEffect;
    use crate::event::{DiskChanged, IsographEvent, Presence};

    fn disk_file<'a, THostLanguage: HostLanguage>(
        state: &'a IsographState<THostLanguage>,
        path: &Path,
    ) -> Option<&'a DiskFile> {
        state
            .get_disk_file_map()
            .untracked()
            .0
            .get(path)
            .map(|id| state.get(*id))
    }

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, IsographEvent::HelloWorld);
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, IsographEvent::Quit);
        assert_eq!(effects, IsographEffect::Kill.wrap_vec());
    }

    #[test]
    fn present_inserts_a_disk_file() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        assert_eq!(effects, Vec::new());
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }

    #[test]
    fn a_second_present_replaces_contents() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Present("first".to_owned()),
            }),
        );
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Present("second".to_owned()),
            }),
        );
        assert_eq!(effects, Vec::new());
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            "second"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 1);
    }

    #[test]
    fn an_empty_string_is_stored() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Present(String::new()),
            }),
        );
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            ""
        );
    }

    #[test]
    fn two_paths_are_two_entries() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let a = PathBuf::from("/tmp/proj/src/a.ts");
        let b = PathBuf::from("/tmp/proj/src/b.ts");
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: a.clone(),
                presence: Presence::Present("a".to_owned()),
            }),
        );
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: b.clone(),
                presence: Presence::Present("b".to_owned()),
            }),
        );
        assert_eq!(
            disk_file(state.reference(), a.reference())
                .expect("the test inserted this path")
                .contents,
            "a"
        );
        assert_eq!(
            disk_file(state.reference(), b.reference())
                .expect("the test inserted this path")
                .contents,
            "b"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn absent_removes_a_disk_file() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Absent,
            }),
        );
        assert_eq!(effects, Vec::new());
        assert!(disk_file(state.reference(), path.reference()).is_none());
    }

    #[test]
    fn absent_of_a_never_present_path_is_a_noop() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: path.clone(),
                presence: Presence::Absent,
            }),
        );
        assert_eq!(effects, Vec::new());
        assert!(disk_file(state.reference(), path.reference()).is_none());
        assert!(state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn absent_then_present_on_different_paths_is_a_move() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let from = PathBuf::from("/tmp/proj/src/a.ts");
        let to = PathBuf::from("/tmp/proj/src/b.ts");
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: from.clone(),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: from.clone(),
                presence: Presence::Absent,
            }),
        );
        handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: to.clone(),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        assert!(disk_file(state.reference(), from.reference()).is_none());
        assert_eq!(
            disk_file(state.reference(), to.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }
}
