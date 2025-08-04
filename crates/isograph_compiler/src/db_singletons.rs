use common_lang_types::{CurrentWorkingDirectory, RelativePathToSourceFile};
use isograph_config::CompilerConfig;
use isograph_lang_types::{IsographDatabase, OpenFileMap, OpenFileSource};
use isograph_schema::StandardSources;
use pico::{Database, SourceId};
use pico_macros::memo;

use crate::source_files::IsoLiteralMap;

// TODO find a good place for this file to live

pub fn get_current_working_directory(db: &IsographDatabase) -> CurrentWorkingDirectory {
    *db.get_singleton::<CurrentWorkingDirectory>()
        .expect("Expected CurrentWorkingDirectory to have been set")
}

pub fn get_isograph_config(db: &IsographDatabase) -> &CompilerConfig {
    db.get_singleton::<CompilerConfig>()
        .expect("Expected CompilerConfig to have been set")
}

pub fn get_standard_sources(db: &IsographDatabase) -> &StandardSources {
    db.get_singleton::<StandardSources>()
        .expect("Expected StandardSources to have been set")
}

pub fn get_iso_literal_map(db: &IsographDatabase) -> &IsoLiteralMap {
    db.get_singleton::<IsoLiteralMap>()
        .expect("Expected IsoLiteralMap to have been set")
}

pub fn get_open_file_map(db: &IsographDatabase) -> &OpenFileMap {
    db.get_singleton::<OpenFileMap>()
        .expect("Expected OpenFileMap to have been set")
}

#[memo]
pub fn get_open_file(
    db: &IsographDatabase,
    file: RelativePathToSourceFile,
) -> Option<SourceId<OpenFileSource>> {
    let file_map = get_open_file_map(db);
    file_map.0.get(&file).cloned()
}
