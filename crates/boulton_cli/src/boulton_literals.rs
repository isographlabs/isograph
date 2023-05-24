use std::{fs, path::PathBuf};

use lazy_static::lazy_static;
use regex::Regex;

use crate::batch_compile::BatchCompileError;

pub(crate) fn read_files_in_folder(
    root_js_path: PathBuf,
) -> Result<Vec<(PathBuf, String)>, BatchCompileError> {
    // current_dir is the directory from which boult was run, and we expect paths
    // to be relative to that.
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(root_js_path);
    let canonicalized_existing_path =
        joined
            .canonicalize()
            .map_err(|message| BatchCompileError::UnableToLoadSchema {
                path: joined.clone(),
                message,
            })?;

    if !canonicalized_existing_path.is_dir() {
        return Err(BatchCompileError::ProjectRootNotADirectory {
            path: canonicalized_existing_path,
        });
    }

    fs::read_dir(canonicalized_existing_path)
        .map_err(|e| BatchCompileError::UnableToTraverseDirectory { message: e })?
        .into_iter()
        .map(move |entry| {
            let entry =
                entry.map_err(|e| BatchCompileError::UnableToTraverseDirectory { message: e })?;

            let contents = std::fs::read(entry.path()).map_err(|message| {
                BatchCompileError::UnableToReadFile {
                    path: entry.path(),
                    message,
                }
            })?;

            let contents = std::str::from_utf8(&contents)
                .map_err(|message| BatchCompileError::UnableToConvertToString { message })?
                .to_owned();

            Ok((entry.path().strip_prefix(&joined)?.to_path_buf(), contents))
        })
        .collect()
}

lazy_static! {
    static ref EXTRACT_BDECLARE: Regex = Regex::new(r"bDeclare`([^`]+)`").unwrap();
}

pub(crate) fn extract_b_declare_literal_from_file_content(
    content: &str,
) -> impl Iterator<Item = &str> {
    EXTRACT_BDECLARE
        .captures_iter(content)
        .into_iter()
        .map(|x| x.get(1).unwrap().as_str())
}
