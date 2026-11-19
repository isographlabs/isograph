use std::sync::atomic::{AtomicUsize, Ordering};

use common_lang_types::RelativePathToSourceFile;
use graphql_network_protocol::GraphQLAndJavascriptProfile;
use intern::string_key::Intern;
use isograph_schema::{
    IsoLiteralsSource, IsographDatabase, entrypoint_declarations, parse_iso_literal_in_source,
    read_iso_literals_source_from_relative_path,
};
use pico_macros::memo;

type TestDatabase = IsographDatabase<GraphQLAndJavascriptProfile>;

static PATH_READS: AtomicUsize = AtomicUsize::new(0);
static FILE_READS: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn file_count(db: &TestDatabase) -> usize {
    PATH_READS.fetch_add(1, Ordering::Relaxed);
    db.get_iso_literals().tracked().len()
}

#[memo]
fn file_content(db: &TestDatabase, path: RelativePathToSourceFile) -> Option<String> {
    FILE_READS.fetch_add(1, Ordering::Relaxed);
    db.get_iso_literal(path)
        .map(|source| source.content.clone())
}

#[test]
fn path_set_tracks_only_membership_changes() {
    let mut db = TestDatabase::default();
    let a = "src/a.ts".intern().into();
    let b = "src/b.ts".intern().into();
    let other = "other.ts".intern().into();

    assert_eq!(*file_count(&db), 0);
    db.insert_iso_literal(a, String::new());
    assert_eq!(*file_count(&db), 1);
    assert_eq!(PATH_READS.load(Ordering::Relaxed), 2);
    assert!(entrypoint_declarations(&db).is_empty());

    // An edit changes the parsed entrypoints while the cached file count stays valid.
    db.insert_iso_literal(a, "iso(`entrypoint Query.Foo`)".into());
    assert_eq!(entrypoint_declarations(&db).len(), 1);
    assert!(db.remove_iso_literal(b).is_none());
    db.remove_iso_literals_from_path("missing/");
    assert_eq!(*file_count(&db), 1);
    assert_eq!(PATH_READS.load(Ordering::Relaxed), 2);

    db.insert_iso_literal(b, String::new());
    db.insert_iso_literal(other, String::new());
    assert_eq!(*file_count(&db), 3);

    // Removing a folder drops all its sources and leaves files outside it available.
    db.remove_iso_literals_from_path("src/");
    assert_eq!(*file_count(&db), 1);
    assert!(entrypoint_declarations(&db).is_empty());
    assert!(db.get_iso_literal(a).is_none());
    assert!(db.get_iso_literal(b).is_none());
    assert_eq!(
        db.remove_iso_literal(other),
        Some(IsoLiteralsSource::id_for_key(&other))
    );
    assert_eq!(*file_count(&db), 0);
}

#[test]
fn single_file_lookup_tracks_its_path_on_miss_and_hit() {
    let mut db = TestDatabase::default();
    let a = "a.ts".intern().into();
    let b = "b.ts".intern().into();

    assert_eq!(file_content(&db, a).as_deref(), None);
    db.insert_iso_literal(b, String::new());
    assert_eq!(file_content(&db, a).as_deref(), None);
    assert_eq!(FILE_READS.load(Ordering::Relaxed), 1);

    db.insert_iso_literal(a, "first".into());
    assert_eq!(file_content(&db, a).as_deref(), Some("first"));
    db.remove_iso_literal(b);
    assert_eq!(file_content(&db, a).as_deref(), Some("first"));
    assert_eq!(FILE_READS.load(Ordering::Relaxed), 2);

    db.remove_iso_literal(a);
    assert_eq!(file_content(&db, a).as_deref(), None);
}

#[test]
fn editor_reads_follow_the_buffer_until_close() {
    let mut db = TestDatabase::default();
    let path = "a.ts".intern().into();
    db.insert_iso_literal(path, "disk".into());
    let editor_content = |db: &TestDatabase| {
        read_iso_literals_source_from_relative_path(db, path)
            .as_ref()
            .unwrap()
            .content
            .clone()
    };

    assert_eq!(editor_content(&db), "disk");
    assert!(parse_iso_literal_in_source(&db, path).is_empty());

    let unsaved = "iso(`entrypoint Query.Unsaved`)";
    db.set_editor_buffer(path, unsaved.into());
    assert_eq!(db.get_iso_literal(path).unwrap().content, "disk");
    assert_eq!(editor_content(&db), unsaved);
    assert!(matches!(
        parse_iso_literal_in_source(&db, path).as_slice(),
        [Ok(_)]
    ));

    db.remove_editor_buffer(path);
    assert_eq!(editor_content(&db), "disk");
    assert!(parse_iso_literal_in_source(&db, path).is_empty());
}

#[test]
fn project_parsing_follows_disk_membership_while_a_buffer_is_open() {
    let mut db = TestDatabase::default();
    let path = "New.ts".intern().into();

    // A buffer alone does not make the file a project source.
    db.set_editor_buffer(path, "iso(`entrypoint Query.New`)".into());
    assert!(db.get_iso_literals().tracked().is_empty());
    assert!(read_iso_literals_source_from_relative_path(&db, path).is_none());
    assert!(entrypoint_declarations(&db).is_empty());

    // Discovering an empty file on disk makes its buffered entrypoint visible.
    db.insert_iso_literal(path, String::new());
    assert_eq!(entrypoint_declarations(&db).len(), 1);

    // Deleting the disk file removes it from the project even while its buffer is open.
    assert!(db.remove_iso_literal(path).is_some());
    assert!(db.get_editor_buffer(path).is_some());
    assert!(db.get_iso_literals().tracked().is_empty());
    assert!(read_iso_literals_source_from_relative_path(&db, path).is_none());
    assert!(entrypoint_declarations(&db).is_empty());
}
