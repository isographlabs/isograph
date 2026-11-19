use std::sync::atomic::{AtomicUsize, Ordering};

use common_lang_types::RelativePathToSourceFile;
use graphql_network_protocol::GraphQLAndJavascriptProfile;
use intern::string_key::Intern;
use isograph_schema::IsographDatabase;
use pico_macros::memo;

static READS: AtomicUsize = AtomicUsize::new(0);

#[memo]
fn open_file_content(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    path: RelativePathToSourceFile,
) -> Option<String> {
    READS.fetch_add(1, Ordering::Relaxed);
    db.get_open_file(path).map(|file| file.content.clone())
}

#[test]
fn open_file_tracks_only_its_path_on_miss_and_hit() {
    let mut db = IsographDatabase::default();
    let path = "a.ts".intern().into();
    let other = "b.ts".intern().into();

    // The first lookup records a dependency on a.ts even though it has never been opened.
    assert_eq!(open_file_content(&db, path).as_deref(), None);
    assert_eq!(READS.load(Ordering::Relaxed), 1);

    // Opening b.ts leaves a.ts absent, so the memo can reuse its cached None.
    db.insert_open_file(other, "other".into());
    assert_eq!(open_file_content(&db, path).as_deref(), None);
    assert_eq!(READS.load(Ordering::Relaxed), 1);

    // Opening a.ts invalidates that cached miss. The next lookup reads its content.
    db.insert_open_file(path, "first".into());
    assert_eq!(open_file_content(&db, path).as_deref(), Some("first"));
    assert_eq!(READS.load(Ordering::Relaxed), 2);

    // Editing b.ts also leaves the cached content of a.ts valid.
    db.insert_open_file(other, "edited".into());
    assert_eq!(open_file_content(&db, path).as_deref(), Some("first"));
    assert_eq!(READS.load(Ordering::Relaxed), 2);

    // Editing a.ts changes the source the memo read, so it picks up the new text.
    db.insert_open_file(path, "second".into());
    assert_eq!(open_file_content(&db, path).as_deref(), Some("second"));

    // Closing a.ts invalidates its cached text. The next lookup sees it as absent.
    db.remove_open_file(path);
    assert_eq!(open_file_content(&db, path).as_deref(), None);

    // Reopening a.ts uses the same source key, so it invalidates the miss recorded after close.
    db.insert_open_file(path, "reopened".into());
    assert_eq!(open_file_content(&db, path).as_deref(), Some("reopened"));
}
