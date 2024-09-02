use isograph_config::CompilerConfig;
use lsp_process_error::LSPProcessResult;
use lsp_server::Connection;

pub mod lsp_notification_dispatch;
pub mod lsp_process_error;
mod lsp_request_dispatch;
pub mod lsp_runtime_error;
mod lsp_state;
mod row_col_offset;
mod semantic_tokens;
pub mod server;
pub mod text_document;

pub async fn start_language_server(config: CompilerConfig) -> LSPProcessResult<()> {
    let (connection, io_handles) = Connection::stdio();
    let params = server::initialize(&connection)?;
    server::run(connection, config, params).await?;
    io_handles.join()?;
    Ok(())
}
