use std::path::PathBuf;

use common_lang_types::CurrentWorkingDirectory;
use isograph_schema::NetworkProtocol;
use lsp_server::Connection;

use crate::server::LSPProcessResult;

mod hover;
pub mod lsp_notification_dispatch;
mod lsp_request_dispatch;
pub mod lsp_runtime_error;
mod semantic_tokens;
pub mod server;
pub mod text_document;

pub async fn start_language_server<TNetworkProtocol: NetworkProtocol + 'static>(
    config_location: &PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) -> LSPProcessResult<(), TNetworkProtocol> {
    let (connection, io_handles) = Connection::stdio();
    let params = server::initialize(&connection)?;
    server::run(
        connection,
        config_location,
        params,
        current_working_directory,
    )
    .await?;
    io_handles.join()?;
    Ok(())
}
