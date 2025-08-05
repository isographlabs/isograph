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

pub const NAMESPACE: IsographSemanticToken = IsographSemanticToken(0);
pub const TYPE_TYPE: IsographSemanticToken = IsographSemanticToken(1);
pub const TYPE_CLASS: IsographSemanticToken = IsographSemanticToken(2);
pub const TYPE_ENUM: IsographSemanticToken = IsographSemanticToken(3);
pub const TYPE_INTERFACE: IsographSemanticToken = IsographSemanticToken(4);
pub const TYPE_STRUCT: IsographSemanticToken = IsographSemanticToken(5);
pub const TYPE_TYPE_PARAMETER: IsographSemanticToken = IsographSemanticToken(6);
pub const TYPE_PARAMETER: IsographSemanticToken = IsographSemanticToken(7);
pub const TYPE_VARIABLE: IsographSemanticToken = IsographSemanticToken(8);
pub const TYPE_PROPERTY: IsographSemanticToken = IsographSemanticToken(9);
pub const TYPE_ENUM_MEMBER: IsographSemanticToken = IsographSemanticToken(10);
pub const TYPE_EVENT: IsographSemanticToken = IsographSemanticToken(11);
pub const TYPE_FUNCTION: IsographSemanticToken = IsographSemanticToken(12);
pub const TYPE_METHOD: IsographSemanticToken = IsographSemanticToken(13);
pub const TYPE_MACRO: IsographSemanticToken = IsographSemanticToken(14);
pub const TYPE_KEYWORD: IsographSemanticToken = IsographSemanticToken(15);
pub const TYPE_MODIFIER: IsographSemanticToken = IsographSemanticToken(16);
pub const TYPE_COMMENT: IsographSemanticToken = IsographSemanticToken(17);
pub const TYPE_STRING: IsographSemanticToken = IsographSemanticToken(18);
pub const TYPE_NUMBER: IsographSemanticToken = IsographSemanticToken(19);
pub const TYPE_REGEXP: IsographSemanticToken = IsographSemanticToken(20);
pub const TYPE_OPERATOR: IsographSemanticToken = IsographSemanticToken(21);
pub const TYPE_DECORATOR: IsographSemanticToken = IsographSemanticToken(22);
