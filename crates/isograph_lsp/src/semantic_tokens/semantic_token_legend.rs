#![allow(unused)]

use lsp_types::{SemanticTokenType, SemanticTokensLegend};

pub(crate) fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::VARIABLE,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::CLASS,
            SemanticTokenType::TYPE,
            SemanticTokenType::PROPERTY,
        ],
        token_modifiers: vec![],
    }
}

pub(crate) fn semantic_token_variable() -> u32 {
    0
}
pub(crate) fn semantic_token_keyword() -> u32 {
    1
}
pub(crate) fn semantic_token_class() -> u32 {
    2
}
pub(crate) fn semantic_token_type() -> u32 {
    3
}
pub(crate) fn semantic_token_property() -> u32 {
    4
}
