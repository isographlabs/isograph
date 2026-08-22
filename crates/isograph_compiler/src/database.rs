use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};

use crate::HostLanguage;

#[derive(Debug, Db)]
pub struct IsographState<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

impl<THostLanguage: HostLanguage> Default for IsographState<THostLanguage> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: PathBuf,
    pub contents: String,
}

impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: PathBuf, contents: String) {
        let source_id = self.set(DiskFile {
            path: path.clone(),
            contents,
        });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: &Path) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(path) {
            self.remove(source_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::{Path, PathBuf};

    use pico::Database;
    use prelude::Postfix;
    use thiserror::Error;

    use super::{DiskFile, IsographState};
    use crate::host_language::{HostLanguage, IsoLiteralExtraction};

    #[derive(Clone, Debug, PartialEq, Eq, Error)]
    #[error("test host")]
    struct TestHostError;

    struct TestHostLanguage;

    impl HostLanguage for TestHostLanguage {
        type Error = TestHostError;
        type LiteralContext = ();

        fn extract_iso_literals(
            _db: &IsographState<Self>,
            _path: PathBuf,
        ) -> &Option<Vec<IsoLiteralExtraction<Self>>> {
            const NONE: Option<Vec<IsoLiteralExtraction<TestHostLanguage>>> = None;
            &NONE
        }
    }

    fn disk_file<'a>(
        state: &'a IsographState<TestHostLanguage>,
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
    fn present_inserts_a_disk_file() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.insert_disk_file(path.clone(), "export const a = 1;\n".to_owned());
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }

    #[test]
    fn a_second_present_replaces_contents() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.insert_disk_file(path.clone(), "first".to_owned());
        state.insert_disk_file(path.clone(), "second".to_owned());
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
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.insert_disk_file(path.clone(), String::new());
        assert_eq!(
            disk_file(state.reference(), path.reference())
                .expect("the test inserted this path")
                .contents,
            ""
        );
    }

    #[test]
    fn two_paths_are_two_entries() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let a = PathBuf::from("/tmp/proj/src/a.ts");
        let b = PathBuf::from("/tmp/proj/src/b.ts");
        state.insert_disk_file(a.clone(), "a".to_owned());
        state.insert_disk_file(b.clone(), "b".to_owned());
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
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.insert_disk_file(path.clone(), "export const a = 1;\n".to_owned());
        state.remove_disk_file(path.reference());
        assert!(disk_file(state.reference(), path.reference()).is_none());
    }

    #[test]
    fn absent_of_a_never_present_path_is_a_noop() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = PathBuf::from("/tmp/proj/src/a.ts");
        state.remove_disk_file(path.reference());
        assert!(disk_file(state.reference(), path.reference()).is_none());
        assert!(state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn absent_then_present_on_different_paths_is_a_move() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let from = PathBuf::from("/tmp/proj/src/a.ts");
        let to = PathBuf::from("/tmp/proj/src/b.ts");
        state.insert_disk_file(from.clone(), "export const a = 1;\n".to_owned());
        state.remove_disk_file(from.reference());
        state.insert_disk_file(to.clone(), "export const a = 1;\n".to_owned());
        assert!(disk_file(state.reference(), from.reference()).is_none());
        assert_eq!(
            disk_file(state.reference(), to.reference())
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }
}
