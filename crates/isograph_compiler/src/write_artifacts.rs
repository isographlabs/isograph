use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use common_lang_types::{ArtifactPathAndContent, Diagnostic, DiagnosticResult};
use intern::string_key::Lookup;

#[tracing::instrument(skip_all)]
pub(crate) fn write_artifacts_to_disk(
    paths_and_contents: impl IntoIterator<Item = ArtifactPathAndContent>,
    artifact_directory: &PathBuf,
) -> DiagnosticResult<usize> {
    if artifact_directory.exists() {
        fs::remove_dir_all(artifact_directory).map_err(|e| {
            unable_to_do_something_at_path_diagnostic(
                artifact_directory,
                &e.to_string(),
                "delete directory",
            )
        })?;
    }
    fs::create_dir_all(artifact_directory).map_err(|e| {
        let message = e.to_string();
        unable_to_do_something_at_path_diagnostic(artifact_directory, &message, "create directory")
    })?;

    let mut count = 0;
    for path_and_content in paths_and_contents {
        // Is this better than materializing paths_and_contents sooner?
        count += 1;

        let absolute_directory = match path_and_content.type_and_field {
            Some(type_and_field) => artifact_directory
                .join(type_and_field.parent_object_entity_name.lookup())
                .join(type_and_field.selectable_name.lookup()),
            None => artifact_directory.clone(),
        };
        fs::create_dir_all(&absolute_directory).map_err(|e| {
            unable_to_do_something_at_path_diagnostic(
                &absolute_directory,
                &e.to_string(),
                "create directory",
            )
        })?;

        let absolute_file_path = absolute_directory.join(path_and_content.file_name.lookup());
        let mut file = File::create(&absolute_file_path).map_err(|e| {
            unable_to_do_something_at_path_diagnostic(
                &absolute_file_path,
                &e.to_string(),
                "create file",
            )
        })?;

        file.write(path_and_content.file_content.as_bytes())
            .map_err(|e| {
                unable_to_do_something_at_path_diagnostic(
                    &absolute_file_path,
                    &e.to_string(),
                    "write contents of file",
                )
            })?;
    }
    Ok(count)
}

pub fn unable_to_do_something_at_path_diagnostic(
    path: &PathBuf,
    message: &str,
    what: &str,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "Unable to {what} at path {path:?}. \
            \nReason: {message}"
        ),
        None,
    )
}
