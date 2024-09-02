use common_lang_types::Span;
use common_lang_types::TextSource;
use common_lang_types::WithSpan;
use isograph_compiler::extract_iso_literal_from_file_content;
use isograph_compiler::IsoLiteralExtraction;
use isograph_lang_parser::parse_iso_literal;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::ClientFieldDeclarationWithUnvalidatedDirectives;
use isograph_lang_types::EntrypointTypeAndField;
use lsp_types::request::SemanticTokensFullRequest;
use lsp_types::request::Request;
use lsp_types::SemanticToken;
use lsp_types::SemanticTokenType;
use lsp_types::SemanticTokens;
use lsp_types::SemanticTokensParams;
use lsp_types::SemanticTokensResult;
use crate::lsp_runtime_error::LSPRuntimeResult;
use crate::lsp_state::LSPState;
use intern::{string_key::Intern, Lookup};

pub fn on_semantic_token_full_request(
    state: &mut LSPState,
    params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    let SemanticTokensParams {
        text_document,
        work_done_progress_params:_,
        partial_result_params:_,
        
    } = params;
    let text = state.text_for(&text_document.uri).expect(format!("Retrieving semantic tokens for document not opened before {}", text_document.uri).as_str());
    let literal_extractions = extract_iso_literal_from_file_content(text);
    let mut semantic_tokens = vec![];
    for literal_extraction in literal_extractions{
        let IsoLiteralExtraction {
            iso_literal_text,
            iso_literal_start_index,
            has_associated_js_function,
            const_export_name,
            has_paren,
        } = literal_extraction;
        let file_path = text_document.uri.path().intern();
        let text_source = TextSource {
            path: file_path.into(),
            span: Some(Span::new(
                iso_literal_start_index as u32,
                (iso_literal_start_index + iso_literal_text.len()) as u32,
            )),
        };
        let iso_literal_extraction_result = parse_iso_literal(
            iso_literal_text,
            file_path.into(),
            const_export_name,
            text_source,
        );
        if let Ok(iso_literal_extraction_result) = iso_literal_extraction_result {
            semantic_tokens.extend(
                iso_literal_parse_result_to_tokens(iso_literal_extraction_result )
            )
        }
    }
    let result = SemanticTokensResult::Tokens(SemanticTokens{
        data: semantic_tokens,
        result_id: None,
    });
    Ok(Some(result))

}

fn iso_literal_parse_result_to_tokens(iso_literal_extraction_result:IsoLiteralExtractionResult) -> Vec<SemanticToken> {
    match iso_literal_extraction_result {
        IsoLiteralExtractionResult::ClientFieldDeclaration(client_field_declaration) => {
            client_field_declaration_to_tokens(client_field_declaration)
        }
        IsoLiteralExtractionResult::EntrypointDeclaration(entrypoint_declaration) => {
            entrypoint_declaration_to_tokens(entrypoint_declaration)
        }
    }
}

fn client_field_declaration_to_tokens(client_field_declaration:WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>) -> Vec<SemanticToken> {
    vec![]
}

fn entrypoint_declaration_to_tokens(entrypoint_declaration:WithSpan<EntrypointTypeAndField>) -> Vec<SemanticToken> {
    let parent_semantic_token = SemanticToken{
        delta_line: 0,
        delta_start: 0,
        length: 1,
        token_type: 1,
        token_modifiers_bitset: 0,
    };
    let client_field_name_token = SemanticToken{
        delta_line: 1,
        delta_start: 1,
        length: 1,
        token_type: 0,
        token_modifiers_bitset: 0,
    };
    vec![parent_semantic_token, client_field_name_token]
}