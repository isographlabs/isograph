#![allow(unused)]

use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};

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
        token_modifiers: vec![
            SemanticTokenModifier::DECLARATION,
            SemanticTokenModifier::DEFINITION,
            SemanticTokenModifier::READONLY,
            SemanticTokenModifier::STATIC,
            SemanticTokenModifier::DEPRECATED,
            SemanticTokenModifier::ABSTRACT,
            SemanticTokenModifier::ASYNC,
        ],
    }
}

pub(crate) fn semantic_token_type_namespace() -> u32 {
    0
}
pub(crate) fn semantic_token_type_type() -> u32 {
    1
}
pub(crate) fn semantic_token_type_class() -> u32 {
    2
}
pub(crate) fn semantic_token_type_enum() -> u32 {
    3
}
pub(crate) fn semantic_token_type_interface() -> u32 {
    4
}
pub(crate) fn semantic_token_type_struct() -> u32 {
    5
}
pub(crate) fn semantic_token_type_type_parameter() -> u32 {
    6
}
pub(crate) fn semantic_token_type_parameter() -> u32 {
    7
}
pub(crate) fn semantic_token_type_variable() -> u32 {
    8
}
pub(crate) fn semantic_token_type_property() -> u32 {
    9
}
pub(crate) fn semantic_token_type_enum_member() -> u32 {
    10
}
pub(crate) fn semantic_token_type_event() -> u32 {
    11
}
pub(crate) fn semantic_token_type_function() -> u32 {
    12
}
pub(crate) fn semantic_token_type_method() -> u32 {
    13
}
pub(crate) fn semantic_token_type_macro() -> u32 {
    14
}
pub(crate) fn semantic_token_type_keyword() -> u32 {
    15
}
pub(crate) fn semantic_token_type_modifier() -> u32 {
    16
}
pub(crate) fn semantic_token_type_comment() -> u32 {
    17
}
pub(crate) fn semantic_token_type_string() -> u32 {
    18
}
pub(crate) fn semantic_token_type_number() -> u32 {
    19
}
pub(crate) fn semantic_token_type_regexp() -> u32 {
    20
}
pub(crate) fn semantic_token_type_operator() -> u32 {
    21
}
pub(crate) fn semantic_token_type_decorator() -> u32 {
    22
}
