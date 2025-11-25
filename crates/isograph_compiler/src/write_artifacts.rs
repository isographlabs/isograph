use std::{fs, path::PathBuf};

use common_lang_types::{
    ArtifactPathAndContent, Diagnostic, DiagnosticResult, FileSystemOperation,
};

use artifact_content::FileSystemState;

#[tracing::instrument(skip_all)]
pub(crate) fn get_file_system_operations(
    paths_and_contents: impl IntoIterator<Item = ArtifactPathAndContent>,
    artifact_directory: &PathBuf,
    file_system_state: &mut FileSystemState,
) -> Vec<FileSystemOperation> {
    let artifacts: Vec<ArtifactPathAndContent> = paths_and_contents.into_iter().collect();
    let new_file_system_state = FileSystemState::from_artifacts(artifacts);
    let operations = file_system_state.diff(&new_file_system_state, artifact_directory);
    *file_system_state = new_file_system_state;
    operations
}

#[tracing::instrument(skip_all)]
pub(crate) fn apply_file_system_operations(
    operations: Vec<FileSystemOperation>,
) -> DiagnosticResult<usize> {
    let mut count = 0;

    for operation in operations {
        count += 1;

        match operation {
            FileSystemOperation::DeleteDirectory(path) => {
                if path.exists() {
                    fs::remove_dir_all(path.clone()).map_err(|e| {
                        unable_to_do_something_at_path_diagnostic(
                            &path,
                            &e.to_string(),
                            "delete directory",
                        )
                    })?;
                }
            }
            FileSystemOperation::CreateDirectory(path) => {
                fs::create_dir_all(path.clone()).map_err(|e| {
                    unable_to_do_something_at_path_diagnostic(
                        &path,
                        &e.to_string(),
                        "create directory",
                    )
                })?;
            }
            FileSystemOperation::WriteFile(path, content) => {
                fs::write(path.clone(), content.as_bytes()).map_err(|e| {
                    unable_to_do_something_at_path_diagnostic(
                        &path,
                        &e.to_string(),
                        "write contents of file",
                    )
                })?;
            }
            FileSystemOperation::DeleteFile(path) => {
                fs::remove_file(path.clone()).map_err(|e| {
                    unable_to_do_something_at_path_diagnostic(&path, &e.to_string(), "delete file")
                })?;
            }
        }
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
