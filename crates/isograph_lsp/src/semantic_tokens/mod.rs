pub(crate) mod semantic_token_legend;

use crate::{lsp_runtime_error::LSPRuntimeResult, lsp_state::LSPState};
use lsp_types::request::{Request, SemanticTokensFullRequest};

pub fn on_semantic_token_full_request(
    _state: &mut LSPState,
    _params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    Ok(None)
}
