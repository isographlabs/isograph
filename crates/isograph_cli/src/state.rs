use std::collections::HashMap;
use std::path::PathBuf;

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};
use prelude::Postfix;

use crate::effect::IsographEffect;
use crate::event::{DiskChanged, IsographEvent, Presence};

#[derive(Default, Debug, Db)]
pub struct IsographState {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: PathBuf,
    pub contents: String,
}

impl IsographState {
    pub fn handle(&mut self, event: IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
            IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
            IsographEvent::DiskChanged(change) => {
                self.handle_disk_changed(change);
                Vec::new()
            }
        }
    }

    fn handle_disk_changed(&mut self, change: DiskChanged) {
        match change.presence {
            Presence::Present(contents) => {
                let source_id = self.set(DiskFile {
                    path: change.path.clone(),
                    contents,
                });
                self.get_disk_file_map_mut()
                    .tracked()
                    .0
                    .insert(change.path, source_id);
            }
            Presence::Absent => {
                if let Some(source_id) = self
                    .get_disk_file_map_mut()
                    .tracked()
                    .0
                    .remove(&change.path)
                {
                    self.remove(source_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pico::Database;
    use prelude::Postfix;

    use super::{DiskFile, IsographState};
    use crate::effect::IsographEffect;
    use crate::event::{DiskChanged, IsographEvent, Presence};

    fn disk_file<'a>(state: &'a IsographState, path: &Path) -> Option<&'a DiskFile> {
        state
            .get_disk_file_map()
            .untracked()
            .0
            .get(path)
            .map(|id| state.get(*id))
    }

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = IsographState::default();
        let effects = state.handle(IsographEvent::HelloWorld);
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState::default();
        let effects = state.handle(IsographEvent::Quit);
        assert_eq!(effects, IsographEffect::Kill.wrap_vec());
    }

    #[test]
    fn present_inserts_a_disk_file() {
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let effects = state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present("export const a = 1;\n".to_owned()),
        }));
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
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present("first".to_owned()),
        }));
        let effects = state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present("second".to_owned()),
        }));
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
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present(String::new()),
        }));
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            ""
        );
    }

    #[test]
    fn two_paths_are_two_entries() {
        let mut state = IsographState::default();
        let a = PathBuf::from("/tmp/proj/src/a.ts");
        let b = PathBuf::from("/tmp/proj/src/b.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: a.clone(),
            presence: Presence::Present("a".to_owned()),
        }));
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: b.clone(),
            presence: Presence::Present("b".to_owned()),
        }));
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
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present("export const a = 1;\n".to_owned()),
        }));
        let effects = state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Absent,
        }));
        assert_eq!(effects, Vec::new());
        assert!(disk_file(state.reference(), path.reference()).is_none());
    }

    #[test]
    fn absent_of_a_never_present_path_is_a_noop() {
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        let effects = state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Absent,
        }));
        assert_eq!(effects, Vec::new());
        assert!(disk_file(state.reference(), path.reference()).is_none());
        assert!(state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn present_of_an_empty_string_is_present_not_absent() {
        let mut state = IsographState::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: path.clone(),
            presence: Presence::Present(String::new()),
        }));
        assert!(disk_file(state.reference(), path.reference()).is_some());
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            ""
        );
    }

    #[test]
    fn absent_then_present_on_different_paths_is_a_move() {
        let mut state = IsographState::default();
        let from = PathBuf::from("/tmp/proj/src/a.ts");
        let to = PathBuf::from("/tmp/proj/src/b.ts");
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: from.clone(),
            presence: Presence::Present("export const a = 1;\n".to_owned()),
        }));
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: from.clone(),
            presence: Presence::Absent,
        }));
        state.handle(IsographEvent::DiskChanged(DiskChanged {
            path: to.clone(),
            presence: Presence::Present("export const a = 1;\n".to_owned()),
        }));
        assert!(disk_file(state.reference(), from.reference()).is_none());
        assert_eq!(
            disk_file(state.reference(), to.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }
}
