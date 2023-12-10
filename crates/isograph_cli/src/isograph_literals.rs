use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;

use crate::batch_compile::BatchCompileError;

pub(crate) fn read_files_in_folder(
    canonicalized_root_path: &PathBuf,
) -> Result<Vec<(PathBuf, String)>, BatchCompileError> {
    if !canonicalized_root_path.is_dir() {
        return Err(BatchCompileError::ProjectRootNotADirectory {
            // TODO avoid cloning
            path: canonicalized_root_path.clone(),
        });
    }

    read_dir_recursive(&canonicalized_root_path)?
        .into_iter()
        .map(|path| {
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

            Ok((
                path.strip_prefix(&canonicalized_root_path)?.to_path_buf(),
                contents,
            ))
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
    // This is regex is inadequate, as iso<typeof foo`...`> is invalid, and
    // it's certainly possible to want that.
    static ref EXTRACT_ISO_LITERAL: Regex = Regex::new(r"iso(<[^`]+>)?`([^`]+)`(\()?").unwrap();
    static ref EXTRACT_ISO_FETCH: Regex = Regex::new(r"isoFetch(<[^`]+>)?`([^`]+)`").unwrap();
}

pub(crate) struct IsoLiteralExtraction<'a> {
    pub(crate) iso_literal_text: &'a str,
    pub(crate) iso_literal_start_index: usize,
    pub(crate) has_associated_js_function: bool,
}

pub(crate) fn extract_iso_literal_from_file_content<'a>(
    content: &'a str,
) -> impl Iterator<Item = IsoLiteralExtraction<'a>> + 'a {
    EXTRACT_ISO_LITERAL
        .captures_iter(content)
        .into_iter()
        .map(|captures| {
            let iso_literal_match = captures.get(2).unwrap();
            IsoLiteralExtraction {
                iso_literal_text: iso_literal_match.as_str(),
                iso_literal_start_index: iso_literal_match.start(),
                has_associated_js_function: captures.get(3).is_some(),
            }
        })
}

pub(crate) struct IsoFetchExtraction<'a> {
    pub(crate) iso_fetch_text: &'a str,
    pub(crate) iso_fetch_start_index: usize,
}

pub(crate) fn extract_iso_fetch_from_file_content<'a>(
    content: &'a str,
) -> impl Iterator<Item = IsoFetchExtraction<'a>> + 'a {
    EXTRACT_ISO_FETCH
        .captures_iter(content)
        .into_iter()
        .map(|captures| {
            let iso_fetch_match = captures.get(2).unwrap();
            IsoFetchExtraction {
                iso_fetch_text: iso_fetch_match.as_str(),
                iso_fetch_start_index: iso_fetch_match.start(),
            }
        })
}
