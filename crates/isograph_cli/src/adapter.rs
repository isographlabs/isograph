use isograph_compiler::HostLanguage;
use pico::Database;
use prelude::Postfix;

pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return isograph_lsp::lsp_runtime_error::LSPRuntimeError::ExpectedError.wrap_err();
    };
    let tokens = semantic_tokens(state, absolute.reference());
    tokens
        .map(|data| {
            lsp_types::SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            })
        })
        .wrap_ok()
}

fn file_path(uri: &lsp_types::Uri) -> Option<std::path::PathBuf> {
    url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

fn semantic_tokens<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    absolute: &std::path::Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let cwd = state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(*cwd, absolute);
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(state, path).clone()
}

pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Option<Vec<lsp_types::TextEdit>> {
    let absolute = file_path(uri)?;
    let cwd = *state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(cwd, &absolute);
    let literals = isograph_compiler::parsed_iso_literals_in_file(state, path).as_ref()?;
    let page_content = isograph_compiler::text_through_last_iso_literal(state, path).as_ref()?;
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            let start = extraction.iso_literal_start_index.0 as u32;
            let end = start + extraction.iso_literal_text.len() as u32;
            lsp_types::TextEdit {
                range: isograph_lsp::lsp_range_for_byte_range(page_content, start, end),
                new_text: isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ),
            }
            .wrap_some()
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
