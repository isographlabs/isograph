use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use lazy_static::lazy_static;
use regex::Regex;

use crate::batch_compile::BatchCompileError;

pub(crate) fn read_files_in_folder(
    canonicalized_root_path: &Path,
) -> Result<Vec<(PathBuf, String)>, BatchCompileError> {
    if !canonicalized_root_path.is_dir() {
        return Err(BatchCompileError::ProjectRootNotADirectory {
            // TODO avoid cloning
            path: canonicalized_root_path.to_path_buf(),
        });
    }

    read_dir_recursive(canonicalized_root_path)?
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
        .map(|path| read_file(path, canonicalized_root_path))
        .collect()
}

fn read_file(
    path: PathBuf,
    canonicalized_root_path: &Path,
) -> Result<(PathBuf, String), BatchCompileError> {
    // This isn't ideal. We can avoid a clone if we changed .map_err to match
    let path_2 = path.clone();

    // N.B. we have previously ensured that path is a file
    let contents = std::fs::read(&path).map_err(|message| BatchCompileError::UnableToReadFile {
        path: path_2,
        message,
    })?;

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| BatchCompileError::UnableToConvertToString {
            path: path.clone(),
            reason: e,
        })?
        .to_owned();

    Ok((
        path.strip_prefix(canonicalized_root_path)?.to_path_buf(),
        contents,
    ))
}

fn read_dir_recursive(root_js_path: &Path) -> Result<Vec<PathBuf>, BatchCompileError> {
    let mut paths = vec![];

    visit_dirs_skipping_isograph(root_js_path, &mut |dir_entry| {
        paths.push(dir_entry.path());
    })
    .map_err(BatchCompileError::from)?;

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

pub(crate) static ISOGRAPH_FOLDER: &str = "__isograph";
lazy_static! {
    static ref EXTRACT_ISO_LITERAL: Regex =
        Regex::new(r"(export const ([^ ]+) =\s+)?iso(\()?`([^`]+)`(\))?(\()?").unwrap();
}

pub struct IsoLiteralExtraction<'a> {
    pub const_export_name: Option<&'a str>,
    pub iso_literal_text: &'a str,
    pub iso_literal_start_index: usize,
    pub has_associated_js_function: bool,
    pub has_paren: bool,
}

pub fn extract_iso_literals_from_file_content(
    content: &str,
) -> impl Iterator<Item = IsoLiteralExtraction> + '_ {
    EXTRACT_ISO_LITERAL.captures_iter(content).map(|captures| {
        let iso_literal_match = captures.get(4).unwrap();
        IsoLiteralExtraction {
            const_export_name: captures.get(1).map(|_| captures.get(2).unwrap().as_str()),
            iso_literal_text: iso_literal_match.as_str(),
            iso_literal_start_index: iso_literal_match.start(),
            has_associated_js_function: captures.get(6).is_some(),
            has_paren: captures.get(3).is_some(),
        }
    })
}
