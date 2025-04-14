use common_lang_types::{Span, WithSpan};
use isograph_lang_types::{
    ClientFieldDeclaration, SelectionTypeContainingSelections, UnvalidatedSelection,
};
use lsp_types::SemanticToken;

use crate::row_col_offset::RowColDiff;

use super::{
    semantic_token_generator::SemanticTokenGenerator,
    semantic_token_legend::{
        semantic_token_type_keyword, semantic_token_type_method, semantic_token_type_operator,
        semantic_token_type_type, semantic_token_type_variable,
    },
};

pub(crate) fn client_field_declaration_to_tokens(
    client_field_declaration: WithSpan<ClientFieldDeclaration>,
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

    let name_span = client_field_declaration.item.client_field_name.span;
    let last_span_so_far = name_span;
    semantic_token_generator.generate_semantic_token(name_span, semantic_token_type_method());

    // TODO: Handle directives

    let first_selection_set_span = client_field_declaration
        .item
        .selection_set
        .first()
        .as_ref()
        .map(|x| x.span);
    let last_selection_set_span = client_field_declaration
        .item
        .selection_set
        .last()
        .as_ref()
        .map(|x| x.span);

    if let Some(first_span) = first_selection_set_span {
        semantic_token_generator.generate_semantic_token(
            last_span_so_far.span_between(&first_span),
            semantic_token_type_operator(),
        );
    }

    selection_set_to_tokens(
        &mut semantic_token_generator,
        client_field_declaration.item.selection_set,
    );

    if let Some(last_span) = last_selection_set_span {
        semantic_token_generator.generate_semantic_token(
            Span::new(last_span.end + 1, client_field_declaration.span.end),
            semantic_token_type_operator(),
        );
    }

    semantic_token_generator.consume()
}

fn selection_set_to_tokens(
    semantic_token_generator: &mut SemanticTokenGenerator<'_>,
    selection_set: Vec<WithSpan<UnvalidatedSelection>>,
) {
    for selection in selection_set {
        selection_to_tokens(semantic_token_generator, selection)
    }
}

fn selection_to_tokens(
    semantic_token_generator: &mut SemanticTokenGenerator<'_>,
    selection: WithSpan<UnvalidatedSelection>,
) {
    match selection.item {
        SelectionTypeContainingSelections::Scalar(scalar_field_selection) => {
            let name_span = scalar_field_selection
                .name
                .location
                .span()
                .expect("Expected span to exist");
            if let Some(alias) = scalar_field_selection.reader_alias {
                let alias_span = alias.location.span().expect("Expected span to exist");
                semantic_token_generator
                    .generate_semantic_token(alias_span, semantic_token_type_variable());
                semantic_token_generator.generate_semantic_token(
                    alias_span.span_between(&name_span),
                    semantic_token_type_operator(),
                );
            }
            semantic_token_generator
                .generate_semantic_token(name_span, semantic_token_type_variable());

            todo!("This doesn't work because we don't store directives at the moment. Rethink it!")
        }
        SelectionTypeContainingSelections::Object(linked_field_selection) => {
            let name_span = linked_field_selection
                .name
                .location
                .span()
                .expect("Expected span to exist");
            if let Some(alias) = linked_field_selection.reader_alias {
                let alias_span = alias.location.span().expect("Expected span to exist");
                semantic_token_generator
                    .generate_semantic_token(alias_span, semantic_token_type_variable());
                semantic_token_generator.generate_semantic_token(
                    alias_span.span_between(&name_span),
                    semantic_token_type_operator(),
                )
            }

            // TODO this is awkward
            let last_span_so_far = name_span;
            semantic_token_generator
                .generate_semantic_token(name_span, semantic_token_type_variable());

            if true {
                todo!("This doesn't work because we don't store directives at the moment. Rethink it!");
            }

            let first_selection_set_span = linked_field_selection
                .selection_set
                .first()
                .as_ref()
                .map(|x| x.span);
            let last_selection_set_span = linked_field_selection
                .selection_set
                .last()
                .as_ref()
                .map(|x| x.span);

            if let Some(first_span) = first_selection_set_span {
                semantic_token_generator.generate_semantic_token(
                    last_span_so_far.span_between(&first_span),
                    semantic_token_type_operator(),
                );
            }

            selection_set_to_tokens(
                semantic_token_generator,
                linked_field_selection.selection_set,
            );

            if let Some(last_span) = last_selection_set_span {
                semantic_token_generator.generate_semantic_token(
                    Span::new(last_span.end + 1, selection.span.end),
                    semantic_token_type_operator(),
                );
            }
        }
    }
}
