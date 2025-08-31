use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::Uri;

use crate::uri_file_path_ext::UriFilePathExt;

pub(crate) fn uri_is_in_project_root<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    uri: &Uri,
) -> bool {
    let config = db.get_isograph_config();
    let project_root = &config.project_root;

    let uri_path = match uri.to_file_path() {
        Ok(path) => path,
        Err(_) => return false,
    };

    uri_path.starts_with(project_root)
}
