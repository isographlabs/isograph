use std::ops::Deref;

use crate::{
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    row_col_offset::{rcd_to_end_of_slice, RowColDiff},
};
use common_lang_types::{relative_path_from_absolute_and_working_directory, Span, WithSpan};
use isograph_compiler::{
    get_current_working_directory, parse_iso_literal_in_relative_file,
    read_iso_literals_source_from_relative_path, CompilerState,
};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{IsographDatabase, IsographSemanticToken};
use lsp_types::{
    request::{Request, SemanticTokensFullRequest},
    SemanticToken as LspSemanticToken, SemanticTokens as LspSemanticTokens,
    SemanticTokensResult as LspSemanticTokensResult, Url,
};
use pico_macros::memo;

pub fn on_semantic_token_full_request(
    compiler_state: &CompilerState,
    params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    let uri = params.text_document.uri;
    let db = &compiler_state.db;

    get_semantic_tokens(db, uri).to_owned()
}

#[memo]
fn get_semantic_tokens(
    db: &IsographDatabase,
    uri: Url,
) -> Result<Option<LspSemanticTokensResult>, LSPRuntimeError> {
    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    let memo_ref = parse_iso_literal_in_relative_file(db, relative_path_to_source_file);
    let parse_result = memo_ref.deref();

    if let Some(Ok(parsed_iso_literals)) = parse_result {
        // TODO call this earlier, pass it as a param to parse_iso_literal_in_relative_file
        let memo_ref =
            read_iso_literals_source_from_relative_path(db, relative_path_to_source_file);

        let page_content: &str = &memo_ref
            .deref()
            .as_ref()
            .expect("Expected source to exist")
            .content;

        // Track how many characters we've processed in the entire file.
        // This accumulates across all isograph literals to maintain proper positioning.
        let mut processed_until = 0 as usize;

        let lsp_semantic_tokens = parsed_iso_literals
            .iter()
            .flat_map(|(iso_literal_extraction, text_source)| {
                let iso_literal_extraction_span = text_source
                    .span
                    .expect("Expected span to exist. This is indicative of a bug in Isograph.");
                let iso_literal_extraction_span_start = iso_literal_extraction_span.start as usize;
                let iso_literal_extraction_span_end = iso_literal_extraction_span.end as usize;

                let iso_literal_content = &page_content
                    [iso_literal_extraction_span_start..iso_literal_extraction_span_end];

                // Track the span of the previous semantic token within this isograph literal.
                // LSP semantic tokens use relative positioning, so we need the previous token's
                // end position to calculate the delta for the next token.
                let mut last_semantic_token_span = Span::new(0, 0);

                // During the first iteration, we need to offset by the unprocessed content
                // before this isograph literal, but after the last processed isograph literal.
                let initial_rcd_offset = {
                    let unprocessed_content_before_isograph_literal =
                        &page_content[processed_until..iso_literal_extraction_span_start];
                    rcd_to_end_of_slice(unprocessed_content_before_isograph_literal)
                };

                let output_tokens = get_isograph_semantic_tokens(iso_literal_extraction)
                    .iter()
                    .enumerate()
                    .map(move |(index, current_semantic_token)| {
                        let content_between_semantic_tokens = &iso_literal_content
                            [(last_semantic_token_span.end as usize)
                                ..(current_semantic_token.span.start as usize)];

                        let token = convert_isograph_semantic_token_to_lsp_semantic_token(
                            if index == 0 {
                                initial_rcd_offset
                            } else {
                                RowColDiff::default()
                            },
                            content_between_semantic_tokens,
                            current_semantic_token,
                            last_semantic_token_span,
                        );

                        last_semantic_token_span = current_semantic_token.span;

                        token
                    });

                processed_until =
                    iso_literal_extraction_span_start + (last_semantic_token_span.end as usize);

                output_tokens
            })
            .collect();

        return Ok(Some(LspSemanticTokensResult::Tokens(LspSemanticTokens {
            result_id: None,
            data: lsp_semantic_tokens,
        })));
    }

    Ok(None)
}

fn convert_isograph_semantic_token_to_lsp_semantic_token(
    rcd_offset: RowColDiff,
    content_between_semantic_tokens: &str,
    isograph_semantic_token: &WithSpan<IsographSemanticToken>,
    last_semantic_token_span: Span,
) -> LspSemanticToken {
    let rcd_for_content_before_semantic_token =
        rcd_offset + rcd_to_end_of_slice(content_between_semantic_tokens);

    let delta_line = rcd_for_content_before_semantic_token.delta_line();
    let has_line_break = delta_line > 0;

    let token = LspSemanticToken {
        delta_line,
        // LSP delta_start is relative to line start if there's a line break,
        // otherwise it's relative to the end of the previous token
        delta_start: rcd_for_content_before_semantic_token.delta_start()
            + if has_line_break {
                0
            } else {
                last_semantic_token_span.len()
            },
        length: isograph_semantic_token.span.len(),
        token_type: isograph_semantic_token.item.0,
        token_modifiers_bitset: 0,
    };
    token
}

fn get_isograph_semantic_tokens(
    result: &IsoLiteralExtractionResult,
) -> &Vec<WithSpan<IsographSemanticToken>> {
    let semantic_tokens = match result {
        IsoLiteralExtractionResult::ClientPointerDeclaration(s) => &s.item.semantic_tokens,
        IsoLiteralExtractionResult::ClientFieldDeclaration(s) => &s.item.semantic_tokens,
        IsoLiteralExtractionResult::EntrypointDeclaration(s) => &s.item.semantic_tokens,
    };
    semantic_tokens
}
