use common_lang_types::CurrentWorkingDirectory;
use pico::Database;

pub(crate) fn get_current_working_directory(db: &Database) -> CurrentWorkingDirectory {
    let current_working_directory = *db
        .get_singleton::<CurrentWorkingDirectory>()
        .expect("Expected CurrentWorkingDirectory to have been set");
    current_working_directory
}
