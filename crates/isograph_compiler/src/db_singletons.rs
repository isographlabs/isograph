use common_lang_types::CurrentWorkingDirectory;
use isograph_config::CompilerConfig;
use isograph_schema::StandardSources;
use pico::Database;

use crate::source_files::IsoLiteralMap;

pub(crate) fn get_current_working_directory(db: &Database) -> CurrentWorkingDirectory {
    *db.get_singleton::<CurrentWorkingDirectory>()
        .expect("Expected CurrentWorkingDirectory to have been set")
}

pub(crate) fn get_isograph_config(db: &Database) -> &CompilerConfig {
    db.get_singleton::<CompilerConfig>()
        .expect("Expected CompilerConfig to have been set")
}

pub(crate) fn get_standard_sources(db: &Database) -> &StandardSources {
    db.get_singleton::<StandardSources>()
        .expect("Expected StandardSources to have been set")
}

pub(crate) fn get_iso_literal_map(db: &Database) -> &IsoLiteralMap {
    db.get_singleton::<IsoLiteralMap>()
        .expect("Expected IsoLiteralMap to have been set")
}
