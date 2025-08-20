use std::ops::Deref;

use crate::{
    lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult},
    uri_file_path_ext::UriFilePathExt,
};
use common_lang_types::{
    relative_path_from_absolute_and_working_directory, Span, TextSource, WithSpan,
};
use isograph_compiler::{
    get_current_working_directory, parse_iso_literals_in_file_content_and_return_all,
    read_iso_literals_source_from_relative_path, CompilerState,
};
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::IsographSemanticToken;
use isograph_schema::{IsographDatabase, NetworkProtocol};
use lsp_types::{
    request::{Request, SemanticTokensFullRequest},
    SemanticToken as LspSemanticToken, SemanticTokens as LspSemanticTokens,
    SemanticTokensResult as LspSemanticTokensResult, Uri,
};
use pico_macros::memo;

pub fn on_semantic_token_full_request<TNetworkProtocol: NetworkProtocol + 'static>(
    compiler_state: &CompilerState<TNetworkProtocol>,
    params: <SemanticTokensFullRequest as Request>::Params,
) -> LSPRuntimeResult<<SemanticTokensFullRequest as Request>::Result> {
    let uri = params.text_document.uri;
    let db = &compiler_state.db;

    get_semantic_tokens(db, uri).to_owned()
}

/// Overall algorithm:
/// - for each parsed iso literal, convert the semantic tokens (whose span is relative to the
///   iso literal text) to absolute tokens (whose span is relative to the file), and
///   concatenate those
///   - Throughout this, we have to split multi-line tokens into multiple absolute tokens.
/// - then go through each absolute semantic token and produce an LspSemanticToken by
///   making them relative to the previous one.
///
/// This is somewhat inefficient, as we are iterating over the content in the page at least
/// twice (once when parsing, another time when calculating line breaks.) It would be better
/// to produce lsp semantic tokens at parse time, and then either mutate the first token or
/// insert a token with zero-length, which acts solely to make the delta line/delta start
/// of the lsp semantic tokens accurate.
///
/// Note that we should produce LspSemanticTokens that are accurately placed when parsing,
/// as that implies that if you move the iso literal around (e.g. by typing stuff before it),
/// we cannot reuse that cached value (as the output changes.) (We already can't reuse the
/// cached value, but that is a bug.) See https://github.com/isographlabs/isograph/issues/548
#[memo]
fn get_semantic_tokens<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    uri: Uri,
) -> Result<Option<LspSemanticTokensResult>, LSPRuntimeError> {
    let current_working_directory = get_current_working_directory(db);

    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &uri.to_file_path().expect("Expected file path to be valid."),
    );

    let parse_results = parse_iso_literals_in_file_content_and_return_all(
        db,
        relative_path_to_source_file,
        get_current_working_directory(db),
    );

    // TODO call this earlier, pass it as a param to parse_iso_literal_in_relative_file
    let page_content_memo_ref =
        read_iso_literals_source_from_relative_path(db, relative_path_to_source_file);
    let page_content: &str = &page_content_memo_ref
        .deref()
        .as_ref()
        .expect("Expected source to exist")
        .content;

    let absolute_tokens = concatenate_and_absolutize_relative_tokens(
        parse_results
            .iter()
            .filter_map(|parse_result| parse_result.as_ref().ok()),
        page_content,
    );
    let lsp_tokens = convert_absolute_token_to_lsp_token(absolute_tokens, page_content);

    return Ok(Some(LspSemanticTokensResult::Tokens(LspSemanticTokens {
        result_id: None,
        data: lsp_tokens.collect(),
    })));
}

#[derive(Debug)]
struct AbsoluteIsographSemanticToken {
    absolute_char_start: u32,
    len: u32,
    semantic_token: IsographSemanticToken,
}

fn concatenate_and_absolutize_relative_tokens<'a>(
    parsed_iso_literals: impl Iterator<Item = &'a (IsoLiteralExtractionResult, TextSource)> + 'a,
    page_content: &'a str,
) -> impl Iterator<Item = AbsoluteIsographSemanticToken> + 'a {
    parsed_iso_literals.flat_map(move |(extraction, text_source)| {
        let iso_literal_extraction_span = text_source
            .span
            .expect("Expected span to exist. This is indicative of a bug in Isograph.");

        extraction
            .semantic_tokens()
            .iter()
            .flat_map(move |relative_token| {
                absolutize_relative_token(page_content, iso_literal_extraction_span, relative_token)
            })
    })
}

fn absolutize_relative_token<'a>(
    page_content: &'a str,
    iso_literal_extraction_span: Span,
    relative_token: &'a WithSpan<IsographSemanticToken>,
) -> impl Iterator<Item = AbsoluteIsographSemanticToken> + 'a {
    let span_content = &page_content[(iso_literal_extraction_span.start as usize
        + relative_token.span.start as usize)
        ..(iso_literal_extraction_span.start as usize + relative_token.span.end as usize)];

    // Note the split inclusive here. This makes it so that the lines include the
    // line break, i.e. 'foo\nbar' -> 'foo\n', 'bar'
    span_content
        .split_inclusive('\n')
        .scan(0, move |iterated_so_far_within_token, line_text| {
            let token = AbsoluteIsographSemanticToken {
                absolute_char_start: iso_literal_extraction_span.start
                    + relative_token.span.start
                    + *iterated_so_far_within_token,
                len: line_text.len() as u32,
                semantic_token: relative_token.item,
            };
            *iterated_so_far_within_token += line_text.len() as u32;
            Some(token)
        })
}

fn convert_absolute_token_to_lsp_token<'a>(
    absolute_tokens: impl Iterator<Item = AbsoluteIsographSemanticToken> + 'a,
    page_content: &'a str,
) -> impl Iterator<Item = LspSemanticToken> + 'a {
    absolute_tokens.scan(0, |last_token_start, absolute_token| {
        let new_token_start = absolute_token.absolute_char_start;
        let in_between_content =
            &page_content[(*last_token_start as usize)..(new_token_start as usize)];

        let (delta_line, delta_start) = delta_line_delta_start(in_between_content);

        let token = LspSemanticToken {
            delta_line,
            delta_start,
            length: absolute_token.len,
            token_type: absolute_token.semantic_token.lsp_semantic_token.0,
            token_modifiers_bitset: 0,
        };

        *last_token_start = absolute_token.absolute_char_start;
        Some(token)
    })
}

pub fn delta_line_delta_start(text: &str) -> (u32, u32) {
    let mut last_line_break_index = 0;
    let mut line_break_count = 0;
    for (index, char) in text.chars().enumerate() {
        if char == '\n' {
            line_break_count += 1;
            last_line_break_index = index as u32 + 1;
        }
    }

    (line_break_count, text.len() as u32 - last_line_break_index)
}
