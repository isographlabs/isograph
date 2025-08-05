use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};

pub fn semantic_token_legend() -> SemanticTokensLegend {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct IsographSemanticToken(pub u32);

pub const ST_NAMESPACE: IsographSemanticToken = IsographSemanticToken(0);
pub const ST_TYPE: IsographSemanticToken = IsographSemanticToken(1);
pub const ST_CLASS: IsographSemanticToken = IsographSemanticToken(2);
pub const ST_ENUM: IsographSemanticToken = IsographSemanticToken(3);
pub const ST_INTERFACE: IsographSemanticToken = IsographSemanticToken(4);
pub const ST_STRUCT: IsographSemanticToken = IsographSemanticToken(5);
pub const ST_TYPE_PARAMETER: IsographSemanticToken = IsographSemanticToken(6);
pub const ST_PARAMETER: IsographSemanticToken = IsographSemanticToken(7);
pub const ST_VARIABLE: IsographSemanticToken = IsographSemanticToken(8);
pub const ST_PROPERTY: IsographSemanticToken = IsographSemanticToken(9);
pub const ST_ENUM_MEMBER: IsographSemanticToken = IsographSemanticToken(10);
pub const ST_EVENT: IsographSemanticToken = IsographSemanticToken(11);
pub const ST_FUNCTION: IsographSemanticToken = IsographSemanticToken(12);
pub const ST_METHOD: IsographSemanticToken = IsographSemanticToken(13);
pub const ST_MACRO: IsographSemanticToken = IsographSemanticToken(14);
pub const ST_KEYWORD: IsographSemanticToken = IsographSemanticToken(15);
pub const ST_MODIFIER: IsographSemanticToken = IsographSemanticToken(16);
pub const ST_COMMENT: IsographSemanticToken = IsographSemanticToken(17);
pub const ST_STRING: IsographSemanticToken = IsographSemanticToken(18);
pub const ST_NUMBER: IsographSemanticToken = IsographSemanticToken(19);
pub const ST_REGEXP: IsographSemanticToken = IsographSemanticToken(20);
pub const ST_OPERATOR: IsographSemanticToken = IsographSemanticToken(21);
pub const ST_DECORATOR: IsographSemanticToken = IsographSemanticToken(22);
