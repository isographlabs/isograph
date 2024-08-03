use isograph_config::CompilerConfig;
use lsp_process_error::LSPProcessResult;
use lsp_server::Connection;

pub mod lsp_process_error;
pub mod server;

pub async fn start_language_server(config: CompilerConfig) -> LSPProcessResult<()> {
    let (connection, io_handles) = Connection::stdio();
    let params = server::initialize(&connection)?;
    server::run(connection, config, params).await?;
    io_handles.join()?;
    Ok(())
}
