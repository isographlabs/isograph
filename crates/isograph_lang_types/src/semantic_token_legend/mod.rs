pub mod line_behavior;

use lsp_types::{SemanticTokenModifier, SemanticTokenType, SemanticTokensLegend};

use crate::semantic_token_legend::line_behavior::{
    EndsLineBehavior, InlineBehavior, LineBehavior, SpaceAfter, SpaceBefore, StartsNewLineBehavior,
};

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
pub struct LspSemanticToken(pub u32);

#[expect(unused)]
const LSP_ST_NAMESPACE: LspSemanticToken = LspSemanticToken(0);
const LSP_ST_TYPE: LspSemanticToken = LspSemanticToken(1);
const LSP_ST_CLASS: LspSemanticToken = LspSemanticToken(2);
#[expect(unused)]
const LSP_ST_ENUM: LspSemanticToken = LspSemanticToken(3);
#[expect(unused)]
const LSP_ST_INTERFACE: LspSemanticToken = LspSemanticToken(4);
#[expect(unused)]
const LSP_ST_STRUCT: LspSemanticToken = LspSemanticToken(5);
#[expect(unused)]
const LSP_ST_TYPE_PARAMETER: LspSemanticToken = LspSemanticToken(6);
const LSP_ST_PARAMETER: LspSemanticToken = LspSemanticToken(7);
const LSP_ST_VARIABLE: LspSemanticToken = LspSemanticToken(8);
const LSP_ST_PROPERTY: LspSemanticToken = LspSemanticToken(9);
#[expect(unused)]
const LSP_ST_ENUM_MEMBER: LspSemanticToken = LspSemanticToken(10);
#[expect(unused)]
const LSP_ST_EVENT: LspSemanticToken = LspSemanticToken(11);
#[expect(unused)]
const LSP_ST_FUNCTION: LspSemanticToken = LspSemanticToken(12);
const LSP_ST_METHOD: LspSemanticToken = LspSemanticToken(13);
#[expect(unused)]
const LSP_ST_MACRO: LspSemanticToken = LspSemanticToken(14);
const LSP_ST_KEYWORD: LspSemanticToken = LspSemanticToken(15);
#[expect(unused)]
const LSP_ST_MODIFIER: LspSemanticToken = LspSemanticToken(16);
const LSP_ST_COMMENT: LspSemanticToken = LspSemanticToken(17);
const LSP_ST_STRING: LspSemanticToken = LspSemanticToken(18);
const LSP_ST_NUMBER: LspSemanticToken = LspSemanticToken(19);
#[expect(unused)]
const LSP_ST_REGEXP: LspSemanticToken = LspSemanticToken(20);
const LSP_ST_OPERATOR: LspSemanticToken = LspSemanticToken(21);
const LSP_ST_DECORATOR: LspSemanticToken = LspSemanticToken(22);

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub struct IsographSemanticToken {
    pub lsp_semantic_token: LspSemanticToken,
    pub line_behavior: LineBehavior,
    pub indent_change: IndentChange,
}

// entrypoint is a "use" of another selectable
pub const ST_KEYWORD_USE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_KEYWORD,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
// field, pointer are "declarations" of a selectable
pub const ST_KEYWORD_DECLARATION: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_KEYWORD,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_SERVER_OBJECT_TYPE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_CLASS,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_DOT: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(false),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_TO: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_KEYWORD,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

/// Selectable names used outside of selection sets, in "definition-like" locations,
/// which is to say, used in entrypoints as well.
pub const ST_CLIENT_SELECTABLE_NAME: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_METHOD,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

// {}
pub const ST_OPEN_BRACE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::EndsLine(EndsLineBehavior {
        space_before: SpaceBefore(true),
    }),
    indent_change: IndentChange::Indent,
};
pub const ST_CLOSE_BRACE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::IsOwnLine,
    indent_change: IndentChange::Dedent,
};
// ()
pub const ST_OPEN_PAREN: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::EndsLine(EndsLineBehavior {
        space_before: SpaceBefore(false),
    }),
    indent_change: IndentChange::Indent,
};
pub const ST_CLOSE_PAREN: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Dedent,
};
// Brackets are only used as part of GraphQL literals, where they're treated
// as types
// // []
// pub const ST_OPEN_BRACKET: IsographSemanticToken = IsographSemanticToken {
//     lsp_semantic_token: LSP_ST_OPERATOR,
//     line_behavior: LineBehavior::EndsLine(EndsLineBehavior {
//         space_before: SpaceBefore(false),
//     }),
// };
// pub const ST_CLOSE_BRACKET: IsographSemanticToken = IsographSemanticToken {
//     lsp_semantic_token: LSP_ST_OPERATOR,
//     line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
//         space_after: SpaceAfter(false),
//     }),
// };

pub const ST_COMMA: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::Remove,
    indent_change: IndentChange::Same,
};
// TODO split this up
pub const ST_SELECTION_NAME_OR_ALIAS: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_PROPERTY,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_COLON: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_SELECTION_NAME_OR_ALIAS_POST_COLON: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_PROPERTY,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_DIRECTIVE_AT: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_DECORATOR,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(false),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_DIRECTIVE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_DECORATOR,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_ARGUMENT_NAME: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_PARAMETER,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_VARIABLE_DOLLAR_DECLARATION: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_VARIABLE,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(false),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_VARIABLE_DOLLAR_USAGE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_VARIABLE,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(false),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_VARIABLE: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_VARIABLE,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(false),
        space_after: SpaceAfter(false),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_VARIABLE_EQUALS: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_OPERATOR,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_STRING_LITERAL: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_STRING,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_NUMBER_LITERAL: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_NUMBER,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};
pub const ST_BOOL_OR_NULL: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_VARIABLE,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_OBJECT_LITERAL_KEY: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_PROPERTY,
    line_behavior: LineBehavior::StartsNewLine(StartsNewLineBehavior {
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

// TODO have spaces
pub const ST_TYPE_ANNOTATION: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_TYPE,
    line_behavior: LineBehavior::Inline(InlineBehavior {
        space_before: SpaceBefore(true),
        space_after: SpaceAfter(true),
    }),
    indent_change: IndentChange::Same,
};

pub const ST_COMMENT: IsographSemanticToken = IsographSemanticToken {
    lsp_semantic_token: LSP_ST_COMMENT,
    line_behavior: LineBehavior::IsOwnLine,
    indent_change: IndentChange::Same,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord, Hash)]
pub enum IndentChange {
    Indent,
    Dedent,
    Same,
}
