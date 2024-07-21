use std::io::Error as IOError;

use crossbeam::channel::SendError;
use isograph_compiler::batch_compile::BatchCompileError;
use lsp_server::Message;
use lsp_server::ProtocolError;
use serde_json::Error as SerdeError;
use tokio::task::JoinError;

pub type LSPProcessResult<T> = std::result::Result<T, LSPProcessError>;

macro_rules! extend_error {
    ($error: ident) => {
        impl From<$error> for LSPProcessError {
            fn from(err: $error) -> Self {
                LSPProcessError::$error(err)
            }
        }
    };
}

#[derive(Debug)]
pub enum LSPProcessError {
    ProtocolError(ProtocolError),
    BatchCompileError(BatchCompileError),
    IOError(IOError),
    SerdeError(SerdeError),
    JoinError(JoinError),
    SendError(SendError<Message>),
}

extend_error!(BatchCompileError);
extend_error!(IOError);
extend_error!(ProtocolError);
extend_error!(SerdeError);
extend_error!(JoinError);

impl From<SendError<Message>> for LSPProcessError {
    fn from(err: SendError<Message>) -> Self {
        LSPProcessError::SendError(err)
    }
}
