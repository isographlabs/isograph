#![allow(unused)]

use lsp_types::{SemanticTokenType, SemanticTokensLegend};

pub(crate) fn semantic_token_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: vec![
            SemanticTokenType::NAMESPACE,
            SemanticTokenType::TYPE,
            SemanticTokenType::CLASS,
            SemanticTokenType::ENUM,
            SemanticTokenType::INTERFACE,
            SemanticTokenType::STRUCT,
            SemanticTokenType::TYPE_PARAMETER,
            SemanticTokenType::PARAMETER,
            SemanticTokenType::VARIABLE,
            SemanticTokenType::PROPERTY,
            SemanticTokenType::ENUM_MEMBER,
            SemanticTokenType::EVENT,
            SemanticTokenType::FUNCTION,
            SemanticTokenType::METHOD,
            SemanticTokenType::MACRO,
            SemanticTokenType::KEYWORD,
            SemanticTokenType::MODIFIER,
            SemanticTokenType::COMMENT,
            SemanticTokenType::STRING,
            SemanticTokenType::NUMBER,
            SemanticTokenType::REGEXP,
            SemanticTokenType::OPERATOR,
            SemanticTokenType::DECORATOR,
        ],
        token_modifiers: vec![],
    }
}

pub(crate) fn semantic_token_namespace() -> u32 {
    0
}
pub(crate) fn semantic_token_type() -> u32 {
    1
}
pub(crate) fn semantic_token_class() -> u32 {
    2
}
pub(crate) fn semantic_token_enum() -> u32 {
    3
}
pub(crate) fn semantic_token_interface() -> u32 {
    4
}
pub(crate) fn semantic_token_struct() -> u32 {
    5
}
pub(crate) fn semantic_token_type_parameter() -> u32 {
    6
}
pub(crate) fn semantic_token_parameter() -> u32 {
    7
}
pub(crate) fn semantic_token_variable() -> u32 {
    8
}
pub(crate) fn semantic_token_property() -> u32 {
    9
}
pub(crate) fn semantic_token_enum_member() -> u32 {
    10
}
pub(crate) fn semantic_token_event() -> u32 {
    11
}
pub(crate) fn semantic_token_function() -> u32 {
    12
}
pub(crate) fn semantic_token_method() -> u32 {
    13
}
pub(crate) fn semantic_token_macro() -> u32 {
    14
}
pub(crate) fn semantic_token_keyword() -> u32 {
    15
}
pub(crate) fn semantic_token_modifier() -> u32 {
    16
}
pub(crate) fn semantic_token_comment() -> u32 {
    17
}
pub(crate) fn semantic_token_string() -> u32 {
    18
}
pub(crate) fn semantic_token_number() -> u32 {
    19
}
pub(crate) fn semantic_token_regexp() -> u32 {
    20
}
pub(crate) fn semantic_token_operator() -> u32 {
    21
}
pub(crate) fn semantic_token_decorator() -> u32 {
    22
}
