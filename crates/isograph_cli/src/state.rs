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
    use std::path::PathBuf;

    use prelude::Postfix;

    use isograph_extract_typescript::TypeScriptHostLanguage;

    use super::{IsographState, handle};
    use crate::effect::IsographEffect;
    use crate::event::{DiskChanged, IsographEvent, Presence};

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
    fn disk_changed_present_returns_no_effects() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Present("export const a = 1;\n".to_owned()),
            }),
        );
        assert_eq!(effects, Vec::new());
    }

    #[test]
    fn disk_changed_absent_returns_no_effects() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let effects = handle(
            &mut state,
            IsographEvent::DiskChanged(DiskChanged {
                path: PathBuf::from("/tmp/proj/src/a.ts"),
                presence: Presence::Absent,
            }),
        );
        assert_eq!(effects, Vec::new());
    }
}
