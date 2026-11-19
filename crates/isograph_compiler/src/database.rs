use std::collections::HashMap;
use std::marker::PhantomData;

use common_lang_types::RelativePathToSourceFile;
use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};
use prelude::Postfix;

use crate::CompilationProfile;

/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ClientId(pub u64);

#[derive(Debug, Db)]
pub struct IsographDb<TProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    open_file_map: OpenFileMap,
    phantom_data: PhantomData<TProfile>,
}

impl<TProfile: CompilationProfile> Default for IsographDb<TProfile> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            open_file_map: OpenFileMap::default(),
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

/// Every client's open buffers.
#[derive(Debug, Default)]
pub struct OpenFileMap(pub HashMap<OpenFileKey, SourceId<OpenFile>>);

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct OpenFileKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}

/// One client's buffer of one path.
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct OpenFile {
    #[key]
    pub key: OpenFileKey,
    pub contents: String,
}

/// The text of `path` as `client` reads it: that client's open buffer, otherwise disk.
#[memo]
pub fn file_contents<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    match db.open_file_source_id(OpenFileKey { client, path }) {
        Some(open_file_id) => db.get(open_file_id).contents.clone().wrap_some(),
        None => db
            .get(db.disk_file_source_id(path)?)
            .contents
            .clone()
            .wrap_some(),
    }
}

impl<TProfile: CompilationProfile> IsographDb<TProfile> {
    fn open_file_source_id(&self, key: OpenFileKey) -> Option<SourceId<OpenFile>> {
        match self.get_open_file_map().untracked().0.get(&key).copied() {
            Some(open_file_id) => open_file_id.wrap_some(),
            None => self.get_open_file_map().tracked().0.get(&key).copied(),
        }
    }

    fn disk_file_source_id(&self, path: RelativePathToSourceFile) -> Option<SourceId<DiskFile>> {
        match self.get_disk_file_map().untracked().0.get(&path).copied() {
            Some(disk_file_id) => disk_file_id.wrap_some(),
            None => self.get_disk_file_map().tracked().0.get(&path).copied(),
        }
    }

    pub fn set_open_file(
        &mut self,
        client: ClientId,
        path: RelativePathToSourceFile,
        contents: String,
    ) {
        let key = OpenFileKey { client, path };
        let open_file_id = self.set(OpenFile { key, contents });
        if self.get_open_file_map().untracked().0.contains_key(&key) {
            return;
        }
        self.get_open_file_map_mut()
            .tracked()
            .0
            .insert(key, open_file_id);
    }

    pub fn remove_open_file(&mut self, client: ClientId, path: RelativePathToSourceFile) {
        let key = OpenFileKey { client, path };
        let Some(open_file_id) = self.get_open_file_map().untracked().0.get(&key).copied() else {
            return;
        };
        self.get_open_file_map_mut().tracked().0.remove(&key);
        self.remove(open_file_id);
    }

    pub fn remove_open_files_of_client(&mut self, client: ClientId) {
        let is_client = |key: &OpenFileKey| key.client == client;
        if !self.get_open_file_map().untracked().0.keys().any(is_client) {
            return;
        }
        let open_file_ids: Vec<SourceId<OpenFile>> = self
            .get_open_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| is_client(key))
            .map(|(_, open_file_id)| open_file_id)
            .collect();
        for open_file_id in open_file_ids {
            self.remove(open_file_id);
        }
    }

    /// Every path `client` has open.
    pub fn open_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile> {
        self.get_open_file_map()
            .tracked()
            .0
            .keys()
            .filter(|key| key.client == client)
            .map(|key| key.path)
            .collect()
    }

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
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use prelude::Postfix;
    use thiserror::Error;

    use std::path::Path;

    use super::{ClientId, IsographDb, file_contents};
    use crate::host_language::{
        CompilationProfile, HostLanguage, IncludeFile, IsoLiteralExtraction, NetworkProtocol,
    };

    #[derive(Clone, Debug, PartialEq, Eq, Error)]
    #[error("test host")]
    struct TestHostError;

    struct TestHostLanguage;

    impl HostLanguage for TestHostLanguage {
        type Error = TestHostError;
        type LiteralContext = ();

        fn extract_iso_literals_from_text(_contents: &str) -> Vec<IsoLiteralExtraction<Self>> {
            Vec::new()
        }

        fn should_include_source_file(_relative_path: &Path) -> IncludeFile {
            IncludeFile::Include
        }
    }

    struct TestNetworkProtocol;

    impl NetworkProtocol for TestNetworkProtocol {}

    struct TestProfile;

    impl CompilationProfile for TestProfile {
        type HostLanguage = TestHostLanguage;
        type NetworkProtocol = TestNetworkProtocol;
    }

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    #[test]
    fn present_inserts_a_disk_file() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "export const a = 1;\n".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "export const a = 1;\n".to_owned().wrap_some()
        );
    }

    #[test]
    fn a_second_present_replaces_contents() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "first".to_owned());
        state.insert_disk_file(path, "second".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "second".to_owned().wrap_some()
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 1);
    }

    #[test]
    fn an_empty_string_is_stored() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, String::new());
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "".to_owned().wrap_some()
        );
    }

    #[test]
    fn two_paths_are_two_entries() {
        let mut state = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), a),
            "a".to_owned().wrap_some()
        );
        assert_eq!(
            read(state.reference(), ClientId(1), b),
            "b".to_owned().wrap_some()
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn absent_removes_a_disk_file() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "export const a = 1;\n".to_owned());
        state.remove_disk_file(path);
        assert!(read(state.reference(), ClientId(1), path).is_none());
    }

    #[test]
    fn absent_of_a_never_present_path_is_a_noop() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.remove_disk_file(path);
        assert!(read(state.reference(), ClientId(1), path).is_none());
        assert!(state.get_disk_file_map().untracked().0.is_empty());
    }

    #[test]
    fn absent_then_present_on_different_paths_is_a_move() {
        let mut state = IsographDb::<TestProfile>::default();
        let from = intern_path("src/a.ts");
        let to = intern_path("src/b.ts");
        state.insert_disk_file(from, "export const a = 1;\n".to_owned());
        state.remove_disk_file(from);
        state.insert_disk_file(to, "export const a = 1;\n".to_owned());
        assert!(read(state.reference(), ClientId(1), from).is_none());
        assert_eq!(
            read(state.reference(), ClientId(1), to),
            "export const a = 1;\n".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_empty_relative_path_is_a_map_key() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("");
        state.insert_disk_file(path, "empty".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "empty".to_owned().wrap_some()
        );
        state.remove_disk_file(path);
        assert!(read(state.reference(), ClientId(1), path).is_none());
    }

    #[test]
    fn unnormalized_relative_paths_are_two_entries() {
        let mut state = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/./a.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), a),
            "a".to_owned().wrap_some()
        );
        assert_eq!(
            read(state.reference(), ClientId(1), b),
            "b".to_owned().wrap_some()
        );
        assert_eq!(state.get_disk_file_map().untracked().0.len(), 2);
    }

    #[test]
    fn remove_disk_files_from_path_removes_descendants_not_a_sibling_prefix() {
        let mut state = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        let c = intern_path("src2/c.ts");
        state.insert_disk_file(a, "a".to_owned());
        state.insert_disk_file(b, "b".to_owned());
        state.insert_disk_file(c, "c".to_owned());
        state.remove_disk_files_from_path(intern_path("src"));
        assert!(read(state.reference(), ClientId(1), a).is_none());
        assert!(read(state.reference(), ClientId(1), b).is_none());
        assert_eq!(
            read(state.reference(), ClientId(1), c),
            "c".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_disk_files_from_path_does_not_treat_a_filename_prefix_as_a_directory() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "a".to_owned());
        state.remove_disk_files_from_path(intern_path("src/a.ts.bak"));
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "a".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_disk_files_from_path_of_the_empty_relative_path_removes_every_file() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, "a".to_owned());
        state.remove_disk_files_from_path(intern_path(""));
        assert!(read(state.reference(), ClientId(1), path).is_none());
    }

    fn read(
        db: &IsographDb<TestProfile>,
        client: ClientId,
        path: RelativePathToSourceFile,
    ) -> Option<String> {
        file_contents(db, client, path).clone()
    }

    #[test]
    fn file_contents_with_no_disk_file_and_no_buffer_is_none() {
        let db = IsographDb::<TestProfile>::default();
        assert_eq!(
            read(db.reference(), ClientId(1), intern_path("src/a.ts")),
            None
        );
    }

    #[test]
    fn file_contents_falls_back_to_disk() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_open_buffer_wins_over_disk_for_its_client_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_open_buffer_without_a_disk_file_is_read_by_its_client_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(read(db.reference(), ClientId(2), path), None);
    }

    #[test]
    fn a_second_set_open_file_of_the_same_key_replaces_the_text() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_open_file(ClientId(1), path, "first".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "first".to_owned().wrap_some()
        );
        db.set_open_file(ClientId(1), path, "second".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "second".to_owned().wrap_some()
        );
    }

    #[test]
    fn two_clients_with_one_path_open_each_read_their_own() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "one".to_owned());
        db.set_open_file(ClientId(2), path, "two".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "one".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "two".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_empty_buffer_is_read_and_not_disk() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, String::new());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            String::new().wrap_some()
        );
    }

    #[test]
    fn a_disk_change_while_open_does_not_change_the_buffer_read() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.insert_disk_file(path, "disk changed".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "disk changed".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_open_file_falls_back_to_the_current_disk_text() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.insert_disk_file(path, "disk changed".to_owned());
        db.remove_open_file(ClientId(1), path);
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk changed".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_open_file_of_a_key_never_opened_leaves_every_read_unchanged() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        let other = intern_path("src/b.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "open".to_owned());
        db.remove_open_file(ClientId(1), other);
        db.remove_open_file(ClientId(2), path);
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "disk".to_owned().wrap_some()
        );
        assert_eq!(read(db.reference(), ClientId(1), other), None);
    }

    #[test]
    fn remove_open_files_of_client_leaves_another_clients_buffers() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        db.insert_disk_file(a, "disk a".to_owned());
        db.insert_disk_file(b, "disk b".to_owned());
        db.set_open_file(ClientId(1), a, "one a".to_owned());
        db.set_open_file(ClientId(1), b, "one b".to_owned());
        db.set_open_file(ClientId(2), a, "two a".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), a),
            "one a".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(1), b),
            "one b".to_owned().wrap_some()
        );
        db.remove_open_files_of_client(ClientId(1));
        assert_eq!(
            read(db.reference(), ClientId(1), a),
            "disk a".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(1), b),
            "disk b".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), a),
            "two a".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_open_files_of_a_client_with_nothing_open_leaves_every_read_unchanged() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(2), path, "two".to_owned());
        db.remove_open_files_of_client(ClientId(1));
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "two".to_owned().wrap_some()
        );
        assert_eq!(db.get_open_file_map().untracked().0.len(), 1);
    }

    #[test]
    fn remove_disk_file_of_a_path_open_in_one_client() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "disk".to_owned().wrap_some()
        );
        db.remove_disk_file(path);
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(read(db.reference(), ClientId(2), path), None);
    }

    #[test]
    fn missing_then_present_on_disk_is_read() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        // pico records no dependency on a never-written tracked counter; see refactors/pending/pico-missing-key-dependency.md.
        db.insert_disk_file(intern_path("src/other.ts"), "other".to_owned());
        assert_eq!(read(db.reference(), ClientId(1), path), None);
        db.insert_disk_file(path, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn read_disk_then_open_then_close_then_remove_disk() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        // pico records no dependency on a never-written tracked counter; see refactors/pending/pico-missing-key-dependency.md.
        db.set_open_file(ClientId(2), intern_path("src/other.ts"), "other".to_owned());
        db.insert_disk_file(path, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_open_file(ClientId(1), path, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.set_open_file(ClientId(1), path, "open again".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open again".to_owned().wrap_some()
        );
        db.remove_open_file(ClientId(1), path);
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.remove_disk_file(path);
        assert_eq!(read(db.reference(), ClientId(1), path), None);
    }

    #[test]
    fn another_clients_open_does_not_change_a_disk_read() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_open_file(ClientId(2), path, "two".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn read_before_first_open_file_write_misses_the_later_open() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_open_file(ClientId(1), path, "open".to_owned());
        // pico records no dependency on a never-written tracked counter; see refactors/pending/pico-missing-key-dependency.md; when that lands, flip this assertion to the buffer text.
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn open_paths_of_client_of_an_empty_database_is_empty() {
        let db = IsographDb::<TestProfile>::default();
        assert_eq!(
            db.open_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn open_paths_of_client_excludes_disk_only_paths() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(intern_path("src/a.ts"), "disk".to_owned());
        assert_eq!(
            db.open_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn open_paths_of_client_returns_that_clients_open_path() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        db.set_open_file(ClientId(1), a, "one".to_owned());
        assert_eq!(db.open_paths_of_client(ClientId(1)), a.wrap_vec());
    }

    #[test]
    fn open_paths_of_client_lists_a_path_on_disk_and_open_once() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        db.insert_disk_file(a, "disk".to_owned());
        db.set_open_file(ClientId(1), a, "one".to_owned());
        assert_eq!(db.open_paths_of_client(ClientId(1)), a.wrap_vec());
    }

    #[test]
    fn open_paths_of_client_excludes_another_clients_open_path() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        db.set_open_file(ClientId(1), a, "one".to_owned());
        db.set_open_file(ClientId(2), b, "two".to_owned());
        assert_eq!(db.open_paths_of_client(ClientId(1)), a.wrap_vec());
        assert_eq!(db.open_paths_of_client(ClientId(2)), b.wrap_vec());
    }

    #[test]
    fn open_paths_of_client_drops_a_removed_open_file() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        db.set_open_file(ClientId(1), a, "one".to_owned());
        db.remove_open_file(ClientId(1), a);
        assert_eq!(
            db.open_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }
}
