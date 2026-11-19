pub type LSPRuntimeResult<T> = std::result::Result<T, LSPRuntimeError>;

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum LSPRuntimeError {
    ExpectedError,
    UnexpectedError(String),
}
