use std::collections::HashMap;
use std::marker::PhantomData;

use common_lang_types::RelativePathToSourceFile;
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};
use prelude::Postfix;

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
pub struct DiskFileMap(pub HashMap<RelativePathToSourceFile, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: RelativePathToSourceFile,
    pub contents: String,
}

impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: RelativePathToSourceFile, contents: String) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| key.as_ref().starts_with(path.as_ref()))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
    }

    pub fn disk_file(&self, path: RelativePathToSourceFile) -> Option<&DiskFile> {
        let source_id = self.get_disk_file_map().untracked().0.get(&path).copied()?;
        self.get(source_id).wrap_some()
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use prelude::Postfix;
    use thiserror::Error;

    use super::IsographState;
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
            _path: RelativePathToSourceFile,
        ) -> &Option<Vec<IsoLiteralExtraction<Self>>> {
            const NONE: Option<Vec<IsoLiteralExtraction<TestHostLanguage>>> = None;
            &NONE
        }
    }

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    #[test]
    fn present_inserts_a_disk_file() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "export const a = 1;\n".to_owned());
        assert_eq!(
            state
                .disk_file(path)
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }

    #[test]
    fn a_second_present_replaces_contents() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "first".to_owned());
        state.insert_disk_file(path, "second".to_owned());
        assert_eq!(
            state
                .disk_file(path)
                .expect("the test inserted this path")
                .contents,
            "second"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 1);
    }

    #[test]
    fn an_empty_string_is_stored() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, String::new());
        assert_eq!(
            state
                .disk_file(path)
                .expect("the test inserted this path")
                .contents,
            ""
        );
    }

    #[test]
    fn two_paths_are_two_entries() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        assert_eq!(
            state
                .disk_file(a)
                .expect("the test inserted this path")
                .contents,
            "a"
        );
        assert_eq!(
            state
                .disk_file(b)
                .expect("the test inserted this path")
                .contents,
            "b"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn absent_removes_a_disk_file() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "export const a = 1;\n".to_owned());
        state.remove_disk_file(path);
        assert!(state.disk_file(path).is_none());
    }

    #[test]
    fn absent_of_a_never_present_path_is_a_noop() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.remove_disk_file(path);
        assert!(state.disk_file(path).is_none());
        assert!(state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn absent_then_present_on_different_paths_is_a_move() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let from = intern_path("src/a.ts");
        let to = intern_path("src/b.ts");
        state.insert_disk_file(from, "export const a = 1;\n".to_owned());
        state.remove_disk_file(from);
        state.insert_disk_file(to, "export const a = 1;\n".to_owned());
        assert!(state.disk_file(from).is_none());
        assert_eq!(
            state
                .disk_file(to)
                .expect("the test inserted this path")
                .contents,
            "export const a = 1;\n"
        );
    }

    #[test]
    fn an_empty_relative_path_is_a_map_key() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("");
        state.insert_disk_file(path, "empty".to_owned());
        assert_eq!(
            state
                .disk_file(path)
                .expect("the test inserted this path")
                .contents,
            "empty"
        );
        state.remove_disk_file(path);
        assert!(state.disk_file(path).is_none());
    }

    #[test]
    fn unnormalized_relative_paths_are_two_entries() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/./a.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        assert_eq!(
            state
                .disk_file(a)
                .expect("the test inserted this path")
                .contents,
            "a"
        );
        assert_eq!(
            state
                .disk_file(b)
                .expect("the test inserted this path")
                .contents,
            "b"
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn remove_disk_files_from_path_removes_descendants_not_a_sibling_prefix() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        let c = intern_path("src2/c.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        state.insert_disk_file(c, "c".to_owned());
        state.remove_disk_files_from_path(intern_path("src"));
        assert!(state.disk_file(a).is_none());
        assert!(state.disk_file(b).is_none());
        assert_eq!(
            state
                .disk_file(c)
                .expect("src2 is not a path prefix of src")
                .contents,
            "c"
        );
    }

    #[test]
    fn remove_disk_files_from_path_does_not_treat_a_filename_prefix_as_a_directory() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "a".to_owned());
        state.remove_disk_files_from_path(intern_path("src/a.ts.bak"));
        assert_eq!(
            state
                .disk_file(path)
                .expect("src/a.ts.bak is not a path prefix of src/a.ts")
                .contents,
            "a"
        );
    }

    #[test]
    fn remove_disk_files_from_path_of_the_empty_relative_path_removes_every_file() {
        let mut state = IsographState::<TestHostLanguage>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "a".to_owned());
        state.remove_disk_files_from_path(intern_path(""));
        assert!(state.disk_file(path).is_none());
    }
}
