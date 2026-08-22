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
    }
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
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(&mut state, IsographEvent::Quit);
        assert_eq!(effects, IsographEffect::Kill.wrap_vec());
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
        assert_eq!(effects, Vec::new());
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
        assert_eq!(effects, Vec::new());
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
        assert_eq!(effects, Vec::new());
        assert!(contents(state.reference(), interned("src/a.ts")).is_none());
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
