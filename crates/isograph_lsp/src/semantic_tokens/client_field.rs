use common_lang_types::{Span, WithSpan};
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

    let name_span = client_field_declaration.item.client_field_name.span;
    let mut last_span_so_far = name_span;
    semantic_token_generator.generate_semantic_token(name_span, semantic_token_type_method());

    for directive in client_field_declaration.item.directives {
        last_span_so_far = directive.span;
        semantic_token_generator
            .generate_semantic_token(directive.span, semantic_token_type_decorator());
    }

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

                for directive in scalar_field_selection.directives {
                    semantic_token_generator
                        .generate_semantic_token(directive.span, semantic_token_type_decorator());
                }
            }
            ServerFieldSelection::LinkedField(linked_field_selection) => {
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
                let mut last_span_so_far = name_span;
                semantic_token_generator
                    .generate_semantic_token(name_span, semantic_token_type_variable());

                for directive in linked_field_selection.directives {
                    last_span_so_far = directive.span;
                    semantic_token_generator
                        .generate_semantic_token(directive.span, semantic_token_type_decorator());
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
        },
    }
}
