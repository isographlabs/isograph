use std::path::{Path, PathBuf};

use intern::string_key::Intern;
use isograph_config::{absolute_and_relative_paths, CompilerConfig};

pub fn isograph_config_for_tests(current_working_directory: &Path) -> CompilerConfig {
    let current_working_directory = current_working_directory.to_str().unwrap().intern().into();

    CompilerConfig {
        config_location: PathBuf::from("/test-config-location"),
        project_root: PathBuf::from("/test-project-root"),
        artifact_directory: absolute_and_relative_paths(
            current_working_directory,
            PathBuf::from("/test-artifact-directory"),
        ),
        schema: absolute_and_relative_paths(
            current_working_directory,
            PathBuf::from("/test-schema"),
        ),
        schema_extensions: vec![],
        options: Default::default(),
        current_working_directory,
    }
}
