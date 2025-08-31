use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::Uri;

use crate::uri_file_path_ext::UriFilePathExt;

// TODO we should have a function that goes from Uri to Option<ProjectFile>
// and use ProfileFile everywhere in the LSP, instead of this one-off check
pub(crate) fn uri_is_project_file<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
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
