use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use common_lang_types::{
    CurrentWorkingDirectory, DiagnosticResult, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use isograph_config::ISOGRAPH_FOLDER;
use prelude::Postfix;

use crate::write_artifacts::unable_to_do_something_at_path_diagnostic;

pub fn read_files_in_folder(
    folder: &Path,
    current_working_directory: CurrentWorkingDirectory,
) -> DiagnosticResult<Vec<(RelativePathToSourceFile, String)>> {
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
) -> DiagnosticResult<(RelativePathToSourceFile, String)> {
    // N.B. we have previously ensured that path is a file
    let contents = std::fs::read(&path).map_err(|e| {
        unable_to_do_something_at_path_diagnostic(&path, &e.to_string(), "read file")
    })?;

    let relative_path =
        relative_path_from_absolute_and_working_directory(current_working_directory, &path);

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| {
            unable_to_do_something_at_path_diagnostic(&path, &e.to_string(), "convert file to utf8")
        })?
        .to_owned();

    (relative_path, contents).wrap_ok()
}

fn read_dir_recursive(root_js_path: &Path) -> DiagnosticResult<Vec<PathBuf>> {
    let mut paths = vec![];

    visit_dirs_skipping_isograph(root_js_path, &mut |dir_entry| {
        paths.push(dir_entry.path());
    })
    .map_err(|e| {
        unable_to_do_something_at_path_diagnostic(
            &root_js_path.to_path_buf(),
            &e.to_string(),
            "traverse directory",
        )
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
