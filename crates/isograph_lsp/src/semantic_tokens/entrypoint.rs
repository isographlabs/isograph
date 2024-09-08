use common_lang_types::WithSpan;
use isograph_lang_types::EntrypointTypeAndField;
use lsp_types::SemanticToken;

use crate::row_col_offset::RowColDiff;

use super::{
    semantic_token_generator::SemanticTokenGenerator,
    semantic_token_legend::{
        semantic_token_type_keyword, semantic_token_type_method, semantic_token_type_operator,
        semantic_token_type_type,
    },
};

pub(crate) fn entrypoint_declaration_to_tokens(
    entrypoint_declaration: WithSpan<EntrypointTypeAndField>,
    iso_literal_text: &str,
    initial_diff: RowColDiff,
) -> (Vec<SemanticToken>, RowColDiff) {
    let mut semantic_token_generator = SemanticTokenGenerator::new(iso_literal_text, initial_diff);
    semantic_token_generator.generate_semantic_token(
        entrypoint_declaration.item.entrypoint_keyword.span,
        semantic_token_type_keyword(),
    );
    semantic_token_generator.generate_semantic_token(
        entrypoint_declaration.item.parent_type.span,
        semantic_token_type_type(),
    );
    semantic_token_generator.generate_semantic_token(
        entrypoint_declaration.item.dot.span,
        semantic_token_type_operator(),
    );
    semantic_token_generator.generate_semantic_token(
        entrypoint_declaration.item.client_field_name.span,
        semantic_token_type_method(),
    );
    semantic_token_generator.consume()
}
