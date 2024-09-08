use common_lang_types::WithSpan;
use isograph_lang_types::{
    ClientFieldDeclarationWithUnvalidatedDirectives, Selection, ServerFieldSelection,
};
use lsp_types::SemanticToken;

use crate::row_col_offset::RowColDiff;

use super::{
    semantic_token_generator::SemanticTokenGenerator,
    semantic_token_legend::{
        semantic_token_type_decorator, semantic_token_type_keyword, semantic_token_type_method,
        semantic_token_type_operator, semantic_token_type_type, semantic_token_type_variable,
    },
};

pub(crate) fn client_field_declaration_to_tokens(
    client_field_declaration: WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>,
    iso_literal_text: &str,
    initial_diff: RowColDiff,
) -> (Vec<SemanticToken>, RowColDiff) {
    let mut semantic_token_generator = SemanticTokenGenerator::new(iso_literal_text, initial_diff);
    semantic_token_generator.generate_semantic_token(
        client_field_declaration.item.field_keyword.span,
        semantic_token_type_keyword(),
    );
    semantic_token_generator.generate_semantic_token(
        client_field_declaration.item.parent_type.span,
        semantic_token_type_type(),
    );
    semantic_token_generator.generate_semantic_token(
        client_field_declaration.item.dot.span,
        semantic_token_type_operator(),
    );
    semantic_token_generator.generate_semantic_token(
        client_field_declaration.item.client_field_name.span,
        semantic_token_type_method(),
    );

    for directive in client_field_declaration.item.directives {
        semantic_token_generator
            .generate_semantic_token(directive.span, semantic_token_type_decorator());
    }

    selection_set_to_tokens(
        &mut semantic_token_generator,
        client_field_declaration.item.selection_set,
    );

    semantic_token_generator.consume()
}

fn selection_set_to_tokens(
    semantic_token_generator: &mut SemanticTokenGenerator<'_>,
    selection_set: Vec<WithSpan<Selection<(), ()>>>,
) {
    for selection in selection_set {
        selection_to_tokens(semantic_token_generator, selection)
    }
}

fn selection_to_tokens(
    semantic_token_generator: &mut SemanticTokenGenerator<'_>,
    selection: WithSpan<Selection<(), ()>>,
) {
    match selection.item {
        Selection::ServerField(server_field) => match server_field {
            ServerFieldSelection::ScalarField(scalar_field_selection) => {
                if let Some(alias) = scalar_field_selection.reader_alias {
                    semantic_token_generator.generate_semantic_token(
                        alias.location.span().expect("Expected span to exist"),
                        semantic_token_type_variable(),
                    )
                }
                semantic_token_generator.generate_semantic_token(
                    scalar_field_selection
                        .name
                        .location
                        .span()
                        .expect("Expected span to exist"),
                    semantic_token_type_variable(),
                );

                for directive in scalar_field_selection.directives {
                    semantic_token_generator
                        .generate_semantic_token(directive.span, semantic_token_type_decorator());
                }
            }
            ServerFieldSelection::LinkedField(linked_field_selection) => {
                if let Some(alias) = linked_field_selection.reader_alias {
                    semantic_token_generator.generate_semantic_token(
                        alias.location.span().expect("Expected span to exist"),
                        semantic_token_type_variable(),
                    )
                }
                semantic_token_generator.generate_semantic_token(
                    linked_field_selection
                        .name
                        .location
                        .span()
                        .expect("Expected span to exist"),
                    semantic_token_type_variable(),
                );

                for directive in linked_field_selection.directives {
                    semantic_token_generator
                        .generate_semantic_token(directive.span, semantic_token_type_decorator());
                }
                selection_set_to_tokens(
                    semantic_token_generator,
                    linked_field_selection.selection_set,
                );
            }
        },
    }
}
