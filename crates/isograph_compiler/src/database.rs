use std::any::TypeId;
use std::collections::{BTreeSet, HashMap};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::marker::PhantomData;

use common_lang_types::RelativePathToSourceFile;
use pico::{Database, Key, Singleton, SourceId, Storage};
use pico_macros::{Db, Source, memo};
use prelude::Postfix;

use crate::{CompilationProfile, IncludeFile, NetworkProtocol};

/// A reader of file text.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ClientId(pub u64);

#[derive(Debug, Db)]
pub struct IsographDb<TProfile: CompilationProfile> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    #[tracked]
    editor_buffer_map: EditorBufferMap,
    #[tracked]
    source_path_map: SourcePathMap,
    #[tracked]
    schema_path_map: SchemaPathMap,
    phantom_data: PhantomData<TProfile>,
}

impl<TProfile: CompilationProfile> Default for IsographDb<TProfile> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            editor_buffer_map: EditorBufferMap::default(),
            source_path_map: SourcePathMap::default(),
            schema_path_map: SchemaPathMap::default(),
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

/// Every source file path on disk, and every editor buffer of a source file path.
#[derive(Debug, Default)]
pub struct SourcePathMap {
    pub disk: BTreeSet<RelativePathToSourceFile>,
    pub editor_buffers: BTreeSet<EditorBufferKey>,
}

/// Every schema file path on disk, and every editor buffer of a schema file path.
#[derive(Debug, Default)]
pub struct SchemaPathMap {
    pub disk: BTreeSet<RelativePathToSourceFile>,
    pub editor_buffers: BTreeSet<EditorBufferKey>,
}

/// Which of the config's file lists a path belongs to. A path can be in both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FileMembership {
    pub source_file: IncludeFile,
    pub schema_file: IncludeFile,
}

/// Every client's editor buffers.
#[derive(Debug, Default)]
pub struct EditorBufferMap(pub HashMap<EditorBufferKey, SourceId<EditorBuffer>>);

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct EditorBufferKey {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
}

/// One client's buffer of one path.
#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct EditorBuffer {
    #[key]
    pub key: EditorBufferKey,
    pub contents: String,
}

/// The network protocol's options, a singleton source.
#[derive(Debug, PartialEq, Eq)]
pub struct ProtocolOptions<TOptions>(pub TOptions);

impl<TOptions: 'static> Singleton for ProtocolOptions<TOptions> {
    fn get_singleton_key() -> Key {
        let mut hasher = DefaultHasher::new();
        TypeId::of::<Self>().hash(&mut hasher);
        hasher.finish().to()
    }
}

impl<TOptions: 'static> pico::Source for ProtocolOptions<TOptions> {
    fn get_key(&self) -> Key {
        Self::get_singleton_key()
    }
}

pub type ProfileProtocolOptions<TProfile> =
    <<TProfile as CompilationProfile>::NetworkProtocol as NetworkProtocol>::Options;

/// The protocol's options as set, or its default options when none are set.
pub fn protocol_options<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
) -> ProfileProtocolOptions<TProfile> {
    db.get_singleton::<ProtocolOptions<ProfileProtocolOptions<TProfile>>>()
        .map(|options| options.0.clone())
        .unwrap_or_default()
}

/// The text of `path` as `client` reads it: that client's editor buffer, otherwise disk.
#[memo]
pub fn file_contents<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    match db.try_get(EditorBuffer::id_for_key(&EditorBufferKey { client, path })) {
        Some(editor_buffer) => editor_buffer.contents.clone().wrap_some(),
        None => db
            .try_get(DiskFile::id_for_key(&path))
            .map(|disk_file| disk_file.contents.clone()),
    }
}

/// Every source file path `client` reads: disk paths and `client`'s editor buffer paths, sorted.
#[memo]
pub fn source_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let view = db.get_source_path_map();
    let map = view.tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.editor_buffers
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

/// Every schema file `client` reads: disk paths and this client's editor buffer paths, sorted.
#[memo]
pub fn schema_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let view = db.get_schema_path_map();
    let map = view.tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.editor_buffers
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

impl<TProfile: CompilationProfile> IsographDb<TProfile> {
    pub fn set_protocol_options(&mut self, options: ProfileProtocolOptions<TProfile>) {
        self.set(ProtocolOptions(options));
    }

    pub fn set_editor_buffer(
        &mut self,
        client: ClientId,
        path: RelativePathToSourceFile,
        membership: FileMembership,
        contents: String,
    ) {
        let key = EditorBufferKey { client, path };
        let editor_buffer_id = self.set(EditorBuffer { key, contents });
        if matches!(membership.source_file, IncludeFile::Include)
            && !self
                .get_source_path_map()
                .untracked()
                .editor_buffers
                .contains(&key)
        {
            self.get_source_path_map_mut()
                .tracked()
                .editor_buffers
                .insert(key);
        }
        if matches!(membership.schema_file, IncludeFile::Include)
            && !self
                .get_schema_path_map()
                .untracked()
                .editor_buffers
                .contains(&key)
        {
            self.get_schema_path_map_mut()
                .tracked()
                .editor_buffers
                .insert(key);
        }
        if self
            .get_editor_buffer_map()
            .untracked()
            .0
            .contains_key(&key)
        {
            return;
        }
        self.get_editor_buffer_map_mut()
            .tracked()
            .0
            .insert(key, editor_buffer_id);
    }

    pub fn remove_editor_buffer(&mut self, client: ClientId, path: RelativePathToSourceFile) {
        let key = EditorBufferKey { client, path };
        if self
            .get_source_path_map()
            .untracked()
            .editor_buffers
            .contains(&key)
        {
            self.get_source_path_map_mut()
                .tracked()
                .editor_buffers
                .remove(&key);
        }
        if self
            .get_schema_path_map()
            .untracked()
            .editor_buffers
            .contains(&key)
        {
            self.get_schema_path_map_mut()
                .tracked()
                .editor_buffers
                .remove(&key);
        }
        let Some(editor_buffer_id) = self
            .get_editor_buffer_map()
            .untracked()
            .0
            .get(&key)
            .copied()
        else {
            return;
        };
        self.get_editor_buffer_map_mut().tracked().0.remove(&key);
        self.remove(editor_buffer_id);
    }

    pub fn remove_editor_buffers_of_client(&mut self, client: ClientId) {
        let is_client = |key: &EditorBufferKey| key.client == client;
        if self
            .get_source_path_map()
            .untracked()
            .editor_buffers
            .iter()
            .any(is_client)
        {
            self.get_source_path_map_mut()
                .tracked()
                .editor_buffers
                .retain(|key| !is_client(key));
        }
        if self
            .get_schema_path_map()
            .untracked()
            .editor_buffers
            .iter()
            .any(is_client)
        {
            self.get_schema_path_map_mut()
                .tracked()
                .editor_buffers
                .retain(|key| !is_client(key));
        }
        if !self
            .get_editor_buffer_map()
            .untracked()
            .0
            .keys()
            .any(is_client)
        {
            return;
        }
        let editor_buffer_ids: Vec<SourceId<EditorBuffer>> = self
            .get_editor_buffer_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| is_client(key))
            .map(|(_, editor_buffer_id)| editor_buffer_id)
            .collect();
        for editor_buffer_id in editor_buffer_ids {
            self.remove(editor_buffer_id);
        }
    }

    /// Every schema file path `client` has an editor buffer of.
    pub fn editor_buffer_schema_paths_of_client(
        &self,
        client: ClientId,
    ) -> Vec<RelativePathToSourceFile> {
        self.get_schema_path_map()
            .tracked()
            .editor_buffers
            .iter()
            .filter(|key| key.client == client)
            .map(|key| key.path)
            .collect()
    }

    /// Every source file path `client` has an editor buffer of.
    pub fn editor_buffer_paths_of_client(&self, client: ClientId) -> Vec<RelativePathToSourceFile> {
        self.get_source_path_map()
            .tracked()
            .editor_buffers
            .iter()
            .filter(|key| key.client == client)
            .map(|key| key.path)
            .collect()
    }

    pub fn insert_disk_file(
        &mut self,
        path: RelativePathToSourceFile,
        membership: FileMembership,
        contents: String,
    ) {
        let source_id = self.set(DiskFile { path, contents });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
        if matches!(membership.source_file, IncludeFile::Include)
            && !self.get_source_path_map().untracked().disk.contains(&path)
        {
            self.get_source_path_map_mut().tracked().disk.insert(path);
        }
        if matches!(membership.schema_file, IncludeFile::Include)
            && !self.get_schema_path_map().untracked().disk.contains(&path)
        {
            self.get_schema_path_map_mut().tracked().disk.insert(path);
        }
    }

    pub fn remove_disk_file(&mut self, path: RelativePathToSourceFile) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(&path) {
            self.remove(source_id);
        }
        if self.get_source_path_map().untracked().disk.contains(&path) {
            self.get_source_path_map_mut().tracked().disk.remove(&path);
        }
        if self.get_schema_path_map().untracked().disk.contains(&path) {
            self.get_schema_path_map_mut().tracked().disk.remove(&path);
        }
    }

    pub fn remove_disk_files_from_path(&mut self, path: RelativePathToSourceFile) {
        let in_folder = |key: &RelativePathToSourceFile| key.as_ref().starts_with(path.as_ref());
        let ids: Vec<_> = self
            .get_disk_file_map_mut()
            .tracked()
            .0
            .extract_if(|key, _| in_folder(key))
            .map(|(_, source_id)| source_id)
            .collect();
        for source_id in ids {
            self.remove(source_id);
        }
        if self
            .get_source_path_map()
            .untracked()
            .disk
            .iter()
            .any(in_folder)
        {
            self.get_source_path_map_mut()
                .tracked()
                .disk
                .retain(|key| !in_folder(key));
        }
        if self
            .get_schema_path_map()
            .untracked()
            .disk
            .iter()
            .any(in_folder)
        {
            self.get_schema_path_map_mut()
                .tracked()
                .disk
                .retain(|key| !in_folder(key));
        }
    }
}

#[cfg(test)]
mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use prelude::Postfix;

    use super::{ClientId, EditorBufferKey, FileMembership, IsographDb, file_contents};
    use crate::IncludeFile;
    use crate::test_profile::TestProfile;

    const SOURCE_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Skip,
    };
    const SCHEMA_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Skip,
        schema_file: IncludeFile::Include,
    };
    const BOTH_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Include,
        schema_file: IncludeFile::Include,
    };
    const NEITHER_FILE: FileMembership = FileMembership {
        source_file: IncludeFile::Skip,
        schema_file: IncludeFile::Skip,
    };

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    #[test]
    fn present_inserts_a_disk_file() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, SOURCE_FILE, "export const a = 1;\n".to_owned());
        assert_eq!(
            read(state.reference(), ClientId(1), path),
            "export const a = 1;\n".to_owned().wrap_some()
        );
    }

    #[test]
    fn a_second_present_replaces_contents() {
        let mut state = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        state.insert_disk_file(path, SOURCE_FILE, "first".to_owned());
        state.insert_disk_file(path, SOURCE_FILE, "second".to_owned());
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
        state.insert_disk_file(path, SOURCE_FILE, String::new());
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
        state.insert_disk_file(a, SOURCE_FILE, "a".to_owned());
        state.insert_disk_file(b, SOURCE_FILE, "b".to_owned());
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
        state.insert_disk_file(path, SOURCE_FILE, "export const a = 1;\n".to_owned());
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
        state.insert_disk_file(from, SOURCE_FILE, "export const a = 1;\n".to_owned());
        state.remove_disk_file(from);
        state.insert_disk_file(to, SOURCE_FILE, "export const a = 1;\n".to_owned());
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
        state.insert_disk_file(path, SOURCE_FILE, "empty".to_owned());
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
        state.insert_disk_file(a, SOURCE_FILE, "a".to_owned());
        state.insert_disk_file(b, SOURCE_FILE, "b".to_owned());
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
        state.insert_disk_file(a, SOURCE_FILE, "a".to_owned());
        state.insert_disk_file(b, SOURCE_FILE, "b".to_owned());
        state.insert_disk_file(c, SOURCE_FILE, "c".to_owned());
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
        state.insert_disk_file(path, SOURCE_FILE, "a".to_owned());
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
        state.insert_disk_file(path, SOURCE_FILE, "a".to_owned());
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
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_editor_buffer_wins_over_disk_for_its_client_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
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
    fn an_editor_buffer_without_a_disk_file_is_read_by_its_client_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        assert_eq!(read(db.reference(), ClientId(2), path), None);
    }

    #[test]
    fn a_second_set_editor_buffer_of_the_same_key_replaces_the_text() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "first".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "first".to_owned().wrap_some()
        );
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "second".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "second".to_owned().wrap_some()
        );
    }

    #[test]
    fn two_clients_with_one_path_open_each_read_their_own() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "one".to_owned());
        db.set_editor_buffer(ClientId(2), path, SOURCE_FILE, "two".to_owned());
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
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, String::new());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            String::new().wrap_some()
        );
    }

    #[test]
    fn a_disk_change_while_open_does_not_change_the_buffer_read() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.insert_disk_file(path, SOURCE_FILE, "disk changed".to_owned());
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
    fn remove_editor_buffer_falls_back_to_the_current_disk_text() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.insert_disk_file(path, SOURCE_FILE, "disk changed".to_owned());
        db.remove_editor_buffer(ClientId(1), path);
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk changed".to_owned().wrap_some()
        );
    }

    #[test]
    fn remove_editor_buffer_of_a_key_never_opened_leaves_every_read_unchanged() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        let other = intern_path("src/b.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        db.remove_editor_buffer(ClientId(1), other);
        db.remove_editor_buffer(ClientId(2), path);
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
    fn remove_editor_buffers_of_client_leaves_another_clients_buffers() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        db.insert_disk_file(a, SOURCE_FILE, "disk a".to_owned());
        db.insert_disk_file(b, SOURCE_FILE, "disk b".to_owned());
        db.set_editor_buffer(ClientId(1), a, SOURCE_FILE, "one a".to_owned());
        db.set_editor_buffer(ClientId(1), b, SOURCE_FILE, "one b".to_owned());
        db.set_editor_buffer(ClientId(2), a, SOURCE_FILE, "two a".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), a),
            "one a".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(1), b),
            "one b".to_owned().wrap_some()
        );
        db.remove_editor_buffers_of_client(ClientId(1));
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
    fn remove_editor_buffers_of_a_client_with_nothing_open_leaves_every_read_unchanged() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(2), path, SOURCE_FILE, "two".to_owned());
        db.remove_editor_buffers_of_client(ClientId(1));
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        assert_eq!(
            read(db.reference(), ClientId(2), path),
            "two".to_owned().wrap_some()
        );
        assert_eq!(db.get_editor_buffer_map().untracked().0.len(), 1);
    }

    #[test]
    fn remove_disk_file_of_a_path_open_in_one_client() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
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
        assert_eq!(read(db.reference(), ClientId(1), path), None);
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn read_disk_then_open_then_close_then_remove_disk() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open again".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open again".to_owned().wrap_some()
        );
        db.remove_editor_buffer(ClientId(1), path);
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
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_editor_buffer(ClientId(2), path, SOURCE_FILE, "two".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
    }

    #[test]
    fn read_before_first_editor_buffer_write_sees_the_later_open() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
    }

    #[test]
    fn a_read_of_a_missing_editor_buffer_without_a_disk_file_sees_the_later_open() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        assert_eq!(read(db.reference(), ClientId(1), path), None);
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_unrelated_open_edit_and_close_leaves_reads_of_missing_and_present_files_unchanged() {
        let mut db = IsographDb::<TestProfile>::default();
        let missing = intern_path("src/missing.ts");
        let on_disk = intern_path("src/on_disk.ts");
        let opened = intern_path("src/opened.ts");
        let other = intern_path("src/other.ts");
        db.insert_disk_file(on_disk, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), opened, SOURCE_FILE, "open".to_owned());
        let assert_reads = |db: &IsographDb<TestProfile>| {
            assert_eq!(read(db, ClientId(1), missing), None);
            assert_eq!(
                read(db, ClientId(1), on_disk),
                "disk".to_owned().wrap_some()
            );
            assert_eq!(read(db, ClientId(1), opened), "open".to_owned().wrap_some());
        };
        assert_reads(db.reference());
        db.set_editor_buffer(ClientId(1), other, SOURCE_FILE, "other".to_owned());
        assert_reads(db.reference());
        db.set_editor_buffer(ClientId(2), missing, SOURCE_FILE, "two".to_owned());
        db.set_editor_buffer(ClientId(2), on_disk, SOURCE_FILE, "two".to_owned());
        assert_reads(db.reference());
        db.set_editor_buffer(ClientId(1), other, SOURCE_FILE, "other edited".to_owned());
        assert_reads(db.reference());
        db.remove_editor_buffer(ClientId(1), other);
        db.remove_editor_buffer(ClientId(2), missing);
        db.remove_editor_buffer(ClientId(2), on_disk);
        assert_reads(db.reference());
    }

    #[test]
    fn closing_a_read_editor_buffer_without_a_disk_file_reads_none() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "open".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "open".to_owned().wrap_some()
        );
        db.remove_editor_buffer(ClientId(1), path);
        assert_eq!(read(db.reference(), ClientId(1), path), None);
    }

    #[test]
    fn reopening_a_closed_file_reads_the_new_buffer() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "first".to_owned());
        db.remove_editor_buffer(ClientId(1), path);
        assert_eq!(read(db.reference(), ClientId(1), path), None);
        db.set_editor_buffer(ClientId(1), path, SOURCE_FILE, "second".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "second".to_owned().wrap_some()
        );
    }

    #[test]
    fn an_unrelated_disk_insert_change_and_removal_leaves_reads_of_missing_and_present_files_unchanged()
     {
        let mut db = IsographDb::<TestProfile>::default();
        let missing = intern_path("src/missing.ts");
        let present = intern_path("src/present.ts");
        let other = intern_path("src/other.ts");
        db.insert_disk_file(present, SOURCE_FILE, "present".to_owned());
        let assert_reads = |db: &IsographDb<TestProfile>| {
            assert_eq!(read(db, ClientId(1), missing), None);
            assert_eq!(
                read(db, ClientId(1), present),
                "present".to_owned().wrap_some()
            );
        };
        assert_reads(db.reference());
        db.insert_disk_file(other, SOURCE_FILE, "other".to_owned());
        assert_reads(db.reference());
        db.insert_disk_file(other, SOURCE_FILE, "other changed".to_owned());
        assert_reads(db.reference());
        db.remove_disk_file(other);
        assert_reads(db.reference());
    }

    #[test]
    fn removing_a_read_disk_file_reads_none() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk".to_owned().wrap_some()
        );
        db.remove_disk_file(path);
        assert_eq!(read(db.reference(), ClientId(1), path), None);
        db.insert_disk_file(path, SOURCE_FILE, "disk again".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "disk again".to_owned().wrap_some()
        );
    }

    fn source_disk_paths(db: &IsographDb<TestProfile>) -> Vec<RelativePathToSourceFile> {
        db.get_source_path_map()
            .untracked()
            .disk
            .iter()
            .copied()
            .collect()
    }

    fn schema_disk_paths(db: &IsographDb<TestProfile>) -> Vec<RelativePathToSourceFile> {
        db.get_schema_path_map()
            .untracked()
            .disk
            .iter()
            .copied()
            .collect()
    }

    fn schema_editor_buffer_keys(db: &IsographDb<TestProfile>) -> Vec<EditorBufferKey> {
        db.get_schema_path_map()
            .untracked()
            .editor_buffers
            .iter()
            .copied()
            .collect()
    }

    fn source_editor_buffer_keys(db: &IsographDb<TestProfile>) -> Vec<EditorBufferKey> {
        db.get_source_path_map()
            .untracked()
            .editor_buffers
            .iter()
            .copied()
            .collect()
    }

    #[test]
    fn a_schema_disk_file_is_read_and_listed_as_schema_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("schema/a.graphql");
        db.insert_disk_file(path, SCHEMA_FILE, "type Query".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "type Query".to_owned().wrap_some()
        );
        assert_eq!(schema_disk_paths(db.reference()), path.wrap_vec());
        assert_eq!(
            source_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn a_source_disk_file_is_listed_as_source_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, SOURCE_FILE, "a".to_owned());
        assert_eq!(source_disk_paths(db.reference()), path.wrap_vec());
        assert_eq!(
            schema_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn a_disk_file_in_both_lists_is_listed_in_both() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("both/a.ts");
        db.insert_disk_file(path, BOTH_FILE, "a".to_owned());
        assert_eq!(source_disk_paths(db.reference()), path.wrap_vec());
        assert_eq!(schema_disk_paths(db.reference()), path.wrap_vec());
    }

    #[test]
    fn a_disk_file_in_neither_list_is_read_and_not_listed() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("other/a.txt");
        db.insert_disk_file(path, NEITHER_FILE, "a".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "a".to_owned().wrap_some()
        );
        assert_eq!(
            source_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
        assert_eq!(
            schema_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn a_second_insert_of_a_listed_path_lists_it_once_and_reads_the_second_text() {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("schema/a.graphql");
        db.insert_disk_file(path, SCHEMA_FILE, "first".to_owned());
        db.insert_disk_file(path, SCHEMA_FILE, "second".to_owned());
        assert_eq!(
            read(db.reference(), ClientId(1), path),
            "second".to_owned().wrap_some()
        );
        assert_eq!(schema_disk_paths(db.reference()), path.wrap_vec());
    }

    #[test]
    fn remove_disk_file_removes_the_text_and_the_listing() {
        let mut db = IsographDb::<TestProfile>::default();
        let schema = intern_path("schema/a.graphql");
        let source = intern_path("src/a.ts");
        db.insert_disk_file(schema, SCHEMA_FILE, "type Query".to_owned());
        db.insert_disk_file(source, SOURCE_FILE, "a".to_owned());
        db.remove_disk_file(schema);
        db.remove_disk_file(source);
        assert!(read(db.reference(), ClientId(1), schema).is_none());
        assert_eq!(
            schema_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
        assert_eq!(
            source_disk_paths(db.reference()),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn remove_disk_files_from_path_removes_listed_descendants_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let inside = intern_path("schema/a.graphql");
        let outside = intern_path("schema2/b.graphql");
        let source_inside = intern_path("schema/a.ts");
        let source_outside = intern_path("schema2/b.ts");
        db.insert_disk_file(inside, SCHEMA_FILE, "a".to_owned());
        db.insert_disk_file(outside, SCHEMA_FILE, "b".to_owned());
        db.insert_disk_file(source_inside, SOURCE_FILE, "a".to_owned());
        db.insert_disk_file(source_outside, SOURCE_FILE, "b".to_owned());
        db.remove_disk_files_from_path(intern_path("schema"));
        assert!(read(db.reference(), ClientId(1), inside).is_none());
        assert_eq!(schema_disk_paths(db.reference()), outside.wrap_vec());
        assert_eq!(source_disk_paths(db.reference()), source_outside.wrap_vec());
    }

    #[test]
    fn an_editor_buffer_is_listed_by_its_membership_and_dropped_on_close() {
        let mut db = IsographDb::<TestProfile>::default();
        let source = intern_path("src/a.ts");
        let schema = intern_path("schema/a.graphql");
        db.set_editor_buffer(ClientId(1), source, SOURCE_FILE, "a".to_owned());
        db.set_editor_buffer(ClientId(1), schema, SCHEMA_FILE, "b".to_owned());
        let source_key = EditorBufferKey {
            client: ClientId(1),
            path: source,
        };
        let schema_key = EditorBufferKey {
            client: ClientId(1),
            path: schema,
        };
        assert_eq!(
            source_editor_buffer_keys(db.reference()),
            source_key.wrap_vec()
        );
        assert_eq!(
            schema_editor_buffer_keys(db.reference()),
            schema_key.wrap_vec()
        );
        db.remove_editor_buffer(ClientId(1), source);
        db.remove_editor_buffer(ClientId(1), schema);
        assert_eq!(
            source_editor_buffer_keys(db.reference()),
            Vec::<EditorBufferKey>::new()
        );
        assert_eq!(
            schema_editor_buffer_keys(db.reference()),
            Vec::<EditorBufferKey>::new()
        );
    }

    #[test]
    fn remove_editor_buffers_of_client_drops_that_clients_listed_keys_only() {
        let mut db = IsographDb::<TestProfile>::default();
        let source = intern_path("src/a.ts");
        let schema = intern_path("schema/a.graphql");
        db.set_editor_buffer(ClientId(1), source, SOURCE_FILE, "a".to_owned());
        db.set_editor_buffer(ClientId(1), schema, SCHEMA_FILE, "b".to_owned());
        db.set_editor_buffer(ClientId(2), source, SOURCE_FILE, "a".to_owned());
        db.set_editor_buffer(ClientId(2), schema, SCHEMA_FILE, "b".to_owned());
        db.remove_editor_buffers_of_client(ClientId(1));
        assert_eq!(
            source_editor_buffer_keys(db.reference()),
            EditorBufferKey {
                client: ClientId(2),
                path: source,
            }
            .wrap_vec()
        );
        assert_eq!(
            schema_editor_buffer_keys(db.reference()),
            EditorBufferKey {
                client: ClientId(2),
                path: schema,
            }
            .wrap_vec()
        );
    }

    #[test]
    fn editor_buffer_paths_of_client_of_an_empty_database_is_empty() {
        let db = IsographDb::<TestProfile>::default();
        assert_eq!(
            db.editor_buffer_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn editor_buffer_paths_of_client_excludes_disk_only_paths() {
        let mut db = IsographDb::<TestProfile>::default();
        db.insert_disk_file(intern_path("src/a.ts"), SOURCE_FILE, "disk".to_owned());
        assert_eq!(
            db.editor_buffer_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn editor_buffer_paths_of_client_excludes_a_buffer_that_is_not_a_source_file() {
        let mut db = IsographDb::<TestProfile>::default();
        db.set_editor_buffer(
            ClientId(1),
            intern_path("other/a.ts"),
            NEITHER_FILE,
            "one".to_owned(),
        );
        db.set_editor_buffer(
            ClientId(1),
            intern_path("schema/a.graphql"),
            SCHEMA_FILE,
            "two".to_owned(),
        );
        assert_eq!(
            db.editor_buffer_paths_of_client(ClientId(1)),
            Vec::<RelativePathToSourceFile>::new()
        );
    }

    #[test]
    fn editor_buffer_paths_of_client_returns_that_clients_editor_buffer_path() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        db.set_editor_buffer(ClientId(1), a, SOURCE_FILE, "one".to_owned());
        assert_eq!(db.editor_buffer_paths_of_client(ClientId(1)), a.wrap_vec());
    }

    #[test]
    fn editor_buffer_paths_of_client_lists_a_path_on_disk_and_open_once() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        db.insert_disk_file(a, SOURCE_FILE, "disk".to_owned());
        db.set_editor_buffer(ClientId(1), a, SOURCE_FILE, "one".to_owned());
        db.set_editor_buffer(ClientId(1), a, SOURCE_FILE, "two".to_owned());
        assert_eq!(db.editor_buffer_paths_of_client(ClientId(1)), a.wrap_vec());
    }

    #[test]
    fn editor_buffer_paths_of_client_excludes_another_clients_editor_buffer_path() {
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        db.set_editor_buffer(ClientId(1), a, SOURCE_FILE, "one".to_owned());
        db.set_editor_buffer(ClientId(2), b, SOURCE_FILE, "two".to_owned());
        assert_eq!(db.editor_buffer_paths_of_client(ClientId(1)), a.wrap_vec());
        assert_eq!(db.editor_buffer_paths_of_client(ClientId(2)), b.wrap_vec());
    }
}
