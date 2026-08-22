mod file_semantic_tokens;
mod semantic_tokens;

pub use file_semantic_tokens::lsp_semantic_tokens_for_file;
pub use semantic_tokens::{lsp_semantic_tokens, semantic_token_legend};
