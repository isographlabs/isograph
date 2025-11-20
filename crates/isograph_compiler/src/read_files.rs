use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
    str::Utf8Error,
};

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use isograph_config::ISOGRAPH_FOLDER;
use prelude::Postfix;
use thiserror::Error;

pub fn read_files_in_folder(
    folder: &Path,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<Vec<(RelativePathToSourceFile, String)>, ReadFileError> {
    read_dir_recursive(folder)?
        .into_iter()
        .filter(|p| {
            let extension = p.extension().and_then(|x| x.to_str());

            matches!(
                extension,
                Some("ts") | Some("tsx") | Some("js") | Some("jsx")
            )
        })
        .filter(|p| {
            !p.to_str()
                .expect("Expected path to be stringable")
                .contains("__isograph")
        })
        .map(|path| read_file(path, current_working_directory))
        .collect()
}

pub fn read_file(
    path: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(RelativePathToSourceFile, String), ReadFileError> {
    // N.B. we have previously ensured that path is a file
    let contents = std::fs::read(&path).map_err(|e| ReadFileError::UnableToReadFile {
        path: path.clone(),
        message: e.to_string(),
    })?;

    let relative_path =
        relative_path_from_absolute_and_working_directory(current_working_directory, &path);

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| ReadFileError::UnableToConvertToString { path, reason: e })?
        .to_owned();

    (relative_path, contents).ok()
}

#[expect(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum ReadFileError {
    #[error("Unable to read the file at the following path: {path:?}.\nReason: {message}")]
    UnableToReadFile { path: PathBuf, message: String },

    #[error("Unable to convert file {path:?} to utf8.\nDetailed reason: {reason}")]
    UnableToConvertToString { path: PathBuf, reason: Utf8Error },

    #[error("Unable to traverse directory.\nReason: {message}")]
    UnableToTraverseDirectory { message: String },
}

fn read_dir_recursive(root_js_path: &Path) -> Result<Vec<PathBuf>, ReadFileError> {
    let mut paths = vec![];

    visit_dirs_skipping_isograph(root_js_path, &mut |dir_entry| {
        paths.push(dir_entry.path());
    })
    .map_err(|e| ReadFileError::UnableToTraverseDirectory {
        message: e.to_string(),
    })?;

    Ok(paths)
}

// Thanks https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
fn visit_dirs_skipping_isograph(dir: &Path, cb: &mut dyn FnMut(&DirEntry)) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !dir.ends_with(ISOGRAPH_FOLDER) {
                visit_dirs_skipping_isograph(&path, cb)?;
            }
        } else {
            cb(&entry);
        }
    }
    Ok(())
}
