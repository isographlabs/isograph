use std::{fs, path::PathBuf};

use common_lang_types::{
    ArtifactPathAndContent, Diagnostic, DiagnosticResult, FileSystemOperation,
};

use artifact_content::FileSystemState;

#[tracing::instrument(skip_all)]
pub(crate) fn get_file_system_operations(
    paths_and_contents: &[ArtifactPathAndContent],
    artifact_directory: &PathBuf,
    file_system_state: &mut Option<FileSystemState>,
) -> Vec<FileSystemOperation> {
    let new_file_system_state = paths_and_contents.into();
    let operations = match file_system_state {
        None => FileSystemState::recreate_all(&new_file_system_state, artifact_directory),
        Some(file_system_state) => FileSystemState::diff(
            &file_system_state,
            &new_file_system_state,
            artifact_directory,
        ),
    };
    *file_system_state = Some(new_file_system_state);
    operations
}

#[tracing::instrument(skip_all)]
pub(crate) fn apply_file_system_operations(
    operations: &[FileSystemOperation],
    artifacts: &[ArtifactPathAndContent],
) -> DiagnosticResult<usize> {
    let mut count = 0;

    for operation in operations {
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
                count += 1;
                fs::create_dir_all(path.clone()).map_err(|e| {
                    unable_to_do_something_at_path_diagnostic(
                        &path,
                        &e.to_string(),
                        "create directory",
                    )
                })?;
            }
            FileSystemOperation::WriteFile(path, content) => {
                let content = &artifacts
                    .get(content.idx)
                    .expect("index should be valid for artifacts vec")
                    .file_content;
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
