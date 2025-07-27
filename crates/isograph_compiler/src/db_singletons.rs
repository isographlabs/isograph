use common_lang_types::CurrentWorkingDirectory;
use isograph_config::CompilerConfig;
use pico::Database;

pub(crate) fn get_current_working_directory(db: &Database) -> CurrentWorkingDirectory {
    *db.get_singleton::<CurrentWorkingDirectory>()
        .expect("Expected CurrentWorkingDirectory to have been set")
}

pub(crate) fn get_isograph_config(db: &Database) -> &CompilerConfig {
    db.get_singleton::<CompilerConfig>()
        .expect("Expected CompilerConfig to have been set")
}
