use std::path::PathBuf;

use common_lang_types::CurrentWorkingDirectory;
use isograph_schema::NetworkProtocol;
use lsp_server::Connection;

use crate::server::LSPProcessResult;

mod completion;
mod format;
mod goto_definition;
mod hover;
mod location_utils;
pub mod lsp_notification_dispatch;
mod lsp_request_dispatch;
pub mod lsp_runtime_error;
mod semantic_tokens;
pub mod server;
pub mod text_document;
mod uri_file_path_ext;

pub async fn start_language_server<TNetworkProtocol: NetworkProtocol>(
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> LSPProcessResult<()> {
    let (connection, io_handles) = Connection::stdio();
    let params = server::initialize(&connection)?;
    server::run::<TNetworkProtocol>(
        connection,
        config_location,
        params,
        current_working_directory,
    )
    .await?;
    io_handles.join()?;
    Ok(())
}
