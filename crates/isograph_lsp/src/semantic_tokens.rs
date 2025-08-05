use crate::lsp_runtime_error::LSPRuntimeResult;
use isograph_compiler::CompilerState;
use lsp_types::request::{Request, SemanticTokensFullRequest};

pub fn on_semantic_token_full_request(
    _compiler_state: &CompilerState,
    _params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    Ok(None)
}
