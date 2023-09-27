use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;

use crate::batch_compile::BatchCompileError;

pub(crate) fn read_files_in_folder(
    root_js_path: &PathBuf,
) -> Result<Vec<(PathBuf, String)>, BatchCompileError> {
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

    read_dir_recursive(&canonicalized_existing_path)?
        .into_iter()
        .map(|path| {
            eprintln!("path {:?}", path);
            // This isn't ideal. We can avoid a clone if we changed .map_err to match
            let path_2 = path.clone();

            // N.B. we have previously ensured that path is a file
            let contents =
                std::fs::read(&path).map_err(|message| BatchCompileError::UnableToReadFile {
                    path: path_2,
                    message,
                })?;

            let contents = std::str::from_utf8(&contents)
                .map_err(|message| BatchCompileError::UnableToConvertToString { message })?
                .to_owned();

            Ok((path.strip_prefix(&joined)?.to_path_buf(), contents))
        })
        .collect()
}

fn read_dir_recursive(root_js_path: &PathBuf) -> Result<Vec<PathBuf>, BatchCompileError> {
    let mut paths = vec![];

    visit_dirs_skipping_isograph(&root_js_path, &mut |dir_entry| {
        paths.push(dir_entry.path());
    })
    .map_err(|e| BatchCompileError::UnableToTraverseDirectory { message: e })?;

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

pub(crate) static ISOGRAPH_FOLDER: &'static str = "__isograph";
lazy_static! {
    // This is regex is inadequate, as iso<typeof foo`...`>, and it's certainly possible
    // to want that.
    static ref EXTRACT_ISO: Regex = Regex::new(r"iso(<[^`]+>)?`([^`]+)`(\()?").unwrap();
}

pub(crate) fn extract_b_declare_literal_from_file_content(
    content: &str,
) -> impl Iterator<Item = (&str, bool)> {
    EXTRACT_ISO
        .captures_iter(content)
        .into_iter()
        .map(|x| (x.get(2).unwrap().as_str(), x.get(3).is_some()))
}
