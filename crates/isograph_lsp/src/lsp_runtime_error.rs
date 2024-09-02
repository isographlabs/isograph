use lsp_server::ErrorCode;
use lsp_server::ResponseError;

pub type LSPRuntimeResult<T> = std::result::Result<T, LSPRuntimeError>;

#[derive(Debug, Clone)]
pub enum LSPRuntimeError {
    ExpectedError,
    UnexpectedError(String),
}

impl From<LSPRuntimeError> for Option<ResponseError> {
    fn from(err: LSPRuntimeError) -> Self {
        match err {
            LSPRuntimeError::ExpectedError => None,
            LSPRuntimeError::UnexpectedError(message) => Some(ResponseError {
                code: ErrorCode::UnknownErrorCode as i32,
                message,
                data: None,
            }),
        }
    }
}
