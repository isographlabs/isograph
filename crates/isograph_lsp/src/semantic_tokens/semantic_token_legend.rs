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

pub struct SemanticToken(pub u32);

pub const NAMESPACE: SemanticToken = SemanticToken(0);
pub const TYPE_TYPE: SemanticToken = SemanticToken(1);
pub const TYPE_CLASS: SemanticToken = SemanticToken(2);
pub const TYPE_ENUM: SemanticToken = SemanticToken(3);
pub const TYPE_INTERFACE: SemanticToken = SemanticToken(4);
pub const TYPE_STRUCT: SemanticToken = SemanticToken(5);
pub const TYPE_TYPE_PARAMETER: SemanticToken = SemanticToken(6);
pub const TYPE_PARAMETER: SemanticToken = SemanticToken(7);
pub const TYPE_VARIABLE: SemanticToken = SemanticToken(8);
pub const TYPE_PROPERTY: SemanticToken = SemanticToken(9);
pub const TYPE_ENUM_MEMBER: SemanticToken = SemanticToken(10);
pub const TYPE_EVENT: SemanticToken = SemanticToken(11);
pub const TYPE_FUNCTION: SemanticToken = SemanticToken(12);
pub const TYPE_METHOD: SemanticToken = SemanticToken(13);
pub const TYPE_MACRO: SemanticToken = SemanticToken(14);
pub const TYPE_KEYWORD: SemanticToken = SemanticToken(15);
pub const TYPE_MODIFIER: SemanticToken = SemanticToken(16);
pub const TYPE_COMMENT: SemanticToken = SemanticToken(17);
pub const TYPE_STRING: SemanticToken = SemanticToken(18);
pub const TYPE_NUMBER: SemanticToken = SemanticToken(19);
pub const TYPE_REGEXP: SemanticToken = SemanticToken(20);
pub const TYPE_OPERATOR: SemanticToken = SemanticToken(21);
pub const TYPE_DECORATOR: SemanticToken = SemanticToken(22);
