use std::{borrow::Cow, path::PathBuf, str::FromStr};

use common_lang_types::EmbeddedLocation;
use intern::string_key::Lookup;
use isograph_schema::{CompilationProfile, IsographDatabase};
use lsp_types::{Range, Uri};

use crate::{format::char_index_to_position, uri_file_path_ext::UriFilePathExt};

// TODO we should have a function that goes from Uri to Option<ProjectFile>
// and use ProfileFile everywhere in the LSP, instead of this one-off check
pub(crate) fn uri_is_project_file<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    uri: &Uri,
) -> bool {
    let config = db.get_isograph_config();
    let uri_path = match uri.to_file_path() {
        Ok(path) => path,
        Err(_) => return false,
    };

    uri_path.starts_with(&config.project_root)
        && !uri_path.starts_with(&config.artifact_directory.absolute_path)
}

pub fn isograph_location_to_lsp_location<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    location: EmbeddedLocation,
    content: &str,
) -> Option<lsp_types::Location> {
    let path_buf = PathBuf::from(db.get_current_working_directory().lookup())
        .join(location.text_source.relative_path_to_source_file.lookup());

    let path = path_buf.to_str()?;
    let normalized_path = if cfg!(windows) {
        Cow::Owned(format!(
            "/{}",
            path.strip_prefix(r"\\?\")
                .unwrap_or(path)
                .replace('\\', "/")
        ))
    } else {
        Cow::Borrowed(path)
    };
    let uri = Uri::from_str(&format!("file://{normalized_path}")).ok()?;

    let text_source_start = location
        .text_source
        .span
        .map(|span| span.start)
        .unwrap_or_default();

    Some(lsp_types::Location {
        uri,
        range: Range {
            start: char_index_to_position(
                content,
                (text_source_start + location.span.start)
                    .try_into()
                    .unwrap(),
            ),
            end: char_index_to_position(
                content,
                (text_source_start + location.span.end).try_into().unwrap(),
            ),
        },
    })
}
