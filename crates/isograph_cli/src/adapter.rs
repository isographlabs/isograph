use common_lang_types::{
    CurrentWorkingDirectory, RelativePathError, RelativePathToSourceFile,
    relative_path_from_absolute_and_working_directory,
};
use isograph_compiler::{ClientId, HostLanguage, IsographDb};
use isograph_lsp::lsp_runtime_error::{LSPRuntimeError, LSPRuntimeResult};
use pico::Database;
use prelude::Postfix;
use tracing::warn;

use crate::effect::{IsographEffect, no_effects};
use crate::state::IsographState;

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum UriPathError {
    #[error("the URI is not a file URI")]
    NotAFileUri,
    #[error("CurrentWorkingDirectory is not interned")]
    MissingWorkingDirectory,
    #[error("{0}")]
    Relative(RelativePathError),
}

pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::SemanticTokensParams,
) -> LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(db, client, path)
        .clone()
        .map(|data| {
            lsp_types::SemanticTokensResult::Tokens(lsp_types::SemanticTokens {
                result_id: None,
                data,
            })
        })
        .wrap_ok()
}

pub(crate) fn hover_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::HoverParams,
) -> LSPRuntimeResult<<lsp_types::request::HoverRequest as lsp_types::request::Request>::Result> {
    let position = params.text_document_position_params;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::hover_for_position(db, client, path, position.position).wrap_ok()
}

pub(crate) fn document_highlight_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DocumentHighlightParams,
) -> LSPRuntimeResult<
    <lsp_types::request::DocumentHighlightRequest as lsp_types::request::Request>::Result,
> {
    let position = params.text_document_position_params;
    let path = relative_from_uri(db, position.text_document.uri.reference()).map_err(lsp_error)?;
    isograph_lsp::token_range_at_lsp_position(db, client, path, position.position)
        .map(|range| {
            lsp_types::DocumentHighlight {
                range,
                kind: lsp_types::DocumentHighlightKind::TEXT.wrap_some(),
            }
            .wrap_vec()
        })
        .wrap_ok()
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum DidChangeError {
    #[error("didChange carried {0} content changes; full document sync sends exactly one")]
    NotOneChange(usize),
    #[error("didChange carried a ranged change; full document sync sends the whole text")]
    RangedChange,
    #[error("{0}")]
    Uri(#[from] UriPathError),
}

pub(crate) fn on_did_open<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Vec<IsographEffect> {
    did_open(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didOpen"));
    no_effects()
}

pub(crate) fn on_did_change<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Vec<IsographEffect> {
    did_change(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didChange"));
    no_effects()
}

pub(crate) fn on_did_close<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Vec<IsographEffect> {
    did_close(&mut state.db, client, params).unwrap_or_else(|error| warn!(%error, "didClose"));
    no_effects()
}

fn did_open<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidOpenTextDocumentParams,
) -> Result<(), UriPathError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, params.text_document.text);
    ().wrap_ok()
}

fn did_change<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidChangeTextDocumentParams,
) -> Result<(), DidChangeError> {
    let [change] =
        <[lsp_types::TextDocumentContentChangeEvent; 1]>::try_from(params.content_changes)
            .map_err(|changes| DidChangeError::NotOneChange(changes.len()))?;
    if change.range.is_some() {
        return DidChangeError::RangedChange.wrap_err();
    }
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.set_open_file(client, path, change.text);
    ().wrap_ok()
}

fn did_close<THostLanguage: HostLanguage>(
    db: &mut IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DidCloseTextDocumentParams,
) -> Result<(), UriPathError> {
    let path = relative_from_uri(db, params.text_document.uri.reference())?;
    db.remove_open_file(client, path);
    ().wrap_ok()
}

fn file_path(uri: &lsp_types::Uri) -> Option<std::path::PathBuf> {
    url::Url::parse(uri.as_str()).ok()?.to_file_path().ok()
}

pub(crate) fn relative_from_uri<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    uri: &lsp_types::Uri,
) -> Result<RelativePathToSourceFile, UriPathError> {
    let absolute = file_path(uri).ok_or(UriPathError::NotAFileUri)?;
    let cwd = db
        .get_singleton::<CurrentWorkingDirectory>()
        .ok_or(UriPathError::MissingWorkingDirectory)?
        .dereference();
    relative_path_from_absolute_and_working_directory(cwd, absolute.as_path())
        .map_err(UriPathError::Relative)
}

fn lsp_error(error: UriPathError) -> LSPRuntimeError {
    match error {
        UriPathError::NotAFileUri => LSPRuntimeError::ExpectedError,
        UriPathError::MissingWorkingDirectory | UriPathError::Relative(_) => {
            LSPRuntimeError::UnexpectedError(error.to_string())
        }
    }
}

pub(crate) fn formatting_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::DocumentFormattingParams,
) -> LSPRuntimeResult<<lsp_types::request::Formatting as lsp_types::request::Request>::Result> {
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    params: lsp_types::WillSaveTextDocumentParams,
) -> LSPRuntimeResult<<lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result>
{
    let path = relative_from_uri(db, params.text_document.uri.reference()).map_err(lsp_error)?;
    format_iso_literals(db, client, path).wrap_ok()
}

fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let page_content =
        isograph_compiler::text_through_last_iso_literal(db, client, path).as_ref()?;
    let line_index = isograph_lsp::LineIndex::new(page_content);
    literals
        .iter()
        .filter_map(|(extraction, parsed)| {
            if !parsed.errors.is_empty() {
                return None;
            }
            lsp_types::TextEdit {
                range: line_index.range(extraction.span()),
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

#[cfg(test)]
pub(crate) mod tests {
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::{ClientId, IsographDb};
    use isograph_extract_typescript::TypeScriptHostLanguage;
    use prelude::Postfix;

    use super::format_iso_literals;
    use crate::state::intern_config_directory;

    const PRETTY: &str = "export const fullName = iso(`\n  field Pet.fullName @foo(\n    bar: 123,\n    baz: $qux,\n  ) {\n    id,\n  }\n`);\n";

    fn db_with(contents: String) -> IsographDb<TypeScriptHostLanguage> {
        let mut db = IsographDb::<TypeScriptHostLanguage>::default();
        intern_config_directory(
            &mut db,
            std::path::Path::new("/tmp/proj/isograph.config.json"),
        );
        db.insert_disk_file("src/Pet.ts".intern().to(), contents);
        db
    }

    fn client() -> ClientId {
        ClientId(1)
    }

    fn path() -> RelativePathToSourceFile {
        "src/Pet.ts".intern().to()
    }

    fn source_with_blank_spaces(spaces: usize) -> String {
        format!(
            "export const fullName = iso(`\n  field Pet.fullName @foo(\n    bar: 123,\n    baz: $qux,\n  ) {{\n    id,\n{}\n  }}\n`);\n",
            " ".repeat(spaces)
        )
    }

    /// Byte ranges of each line's content, excluding its `\n`, `\r\n`, or `\r` terminator.
    fn line_contents(source: &str) -> Vec<std::ops::Range<usize>> {
        let bytes = source.as_bytes();
        let mut lines = Vec::new();
        let mut line_start = 0;
        let mut index = 0;
        while index < bytes.len() {
            match bytes[index] {
                b'\r' if matches!(bytes.get(index + 1), Some(b'\n')) => {
                    lines.push(line_start..index);
                    index += 2;
                    line_start = index;
                }
                b'\r' | b'\n' => {
                    lines.push(line_start..index);
                    index += 1;
                    line_start = index;
                }
                _ => index += 1,
            }
        }
        lines.push(line_start..bytes.len());
        lines
    }

    /// The byte offset of a UTF-16 `position` in `source`, clamped to the end of its line.
    fn byte_offset(source: &str, position: lsp_types::Position) -> usize {
        let lines = line_contents(source);
        let Some(line) = lines.get(position.line as usize) else {
            return source.len();
        };
        let mut utf16 = 0u32;
        source[line.clone()]
            .char_indices()
            .find_map(|(index, ch)| {
                let here = utf16;
                utf16 += ch.len_utf16() as u32;
                (here >= position.character).then_some(line.start + index)
            })
            .unwrap_or(line.end)
    }

    /// Applies `edits` as LSP specifies: every range is resolved against `source`, and the
    /// edits are applied from the last start to the first.
    pub(crate) fn apply_text_edits(source: &str, edits: &[lsp_types::TextEdit]) -> String {
        let mut resolved: Vec<(usize, usize, &str)> = edits
            .iter()
            .map(|edit| {
                (
                    byte_offset(source, edit.range.start),
                    byte_offset(source, edit.range.end),
                    edit.new_text.as_str(),
                )
            })
            .collect();
        resolved.sort_by_key(|(start, _, _)| std::cmp::Reverse(*start));
        resolved
            .into_iter()
            .fold(source.to_owned(), |mut out, (start, end, new_text)| {
                out.replace_range(start..end, new_text);
                out
            })
    }

    /// Asserts the edits are pairwise disjoint and each replaces exactly one iso literal interior.
    pub(crate) fn assert_edits_replace_disjoint_interiors(
        source: &str,
        edits: &[lsp_types::TextEdit],
    ) {
        let mut ranges: Vec<(usize, usize)> = edits
            .iter()
            .map(|edit| {
                (
                    byte_offset(source, edit.range.start),
                    byte_offset(source, edit.range.end),
                )
            })
            .collect();
        ranges.sort();
        for pair in ranges.windows(2) {
            assert!(pair[0].1 <= pair[1].0, "{pair:?} overlap in {source:?}");
        }
        for &(start, end) in &ranges {
            assert!(start <= end, "{start}..{end} is inverted");
            assert!(
                source[..start].ends_with("iso(`"),
                "{start}..{end} in {source:?}"
            );
            assert!(
                source[end..].starts_with('`'),
                "{start}..{end} in {source:?}"
            );
            assert!(
                !source[start..end].contains('`'),
                "{start}..{end} in {source:?}"
            );
        }
    }

    fn formatted_source(contents: String) -> String {
        let db = db_with(contents.clone());
        let edits = format_iso_literals(&db, client(), path()).expect("edits");
        apply_text_edits(&contents, &edits)
    }

    #[test]
    fn formatting_a_pretty_literal_is_idempotent() {
        assert_eq!(formatted_source(PRETTY.to_owned()), PRETTY);
    }

    #[test]
    fn formatting_a_blank_line_after_id_is_pretty_for_each_space_count() {
        (0usize..=4).for_each(|spaces| {
            assert_eq!(
                formatted_source(source_with_blank_spaces(spaces)),
                PRETTY,
                "spaces={spaces}"
            );
        });
    }

    #[test]
    fn applying_the_same_edits_twice_with_stale_ranges_drops_the_closing_template() {
        (0usize..=4).for_each(|spaces| {
            let source = source_with_blank_spaces(spaces);
            let db = db_with(source.clone());
            let edits = format_iso_literals(&db, client(), path()).expect("edits");
            let once = apply_text_edits(&source, &edits);
            assert_eq!(once, PRETTY, "spaces={spaces}");
            let twice = apply_text_edits(&once, &edits);
            assert!(!twice.contains("`);"), "spaces={spaces}\n{twice}");
        });
    }

    #[test]
    fn edits_for_a_client_without_a_buffer_applied_to_a_longer_buffer_leave_an_extra_brace() {
        const EXTRA_BRACE: &str = "export const fullName = iso(`\n  field Pet.fullName @foo(\n    bar: 123,\n    baz: $qux,\n  ) {\n    id,\n  }\n  }\n`);\n";
        let db = db_with(PRETTY.to_owned());
        let edits = format_iso_literals(&db, client(), path()).expect("edits");
        (0usize..=4).for_each(|spaces| {
            assert_eq!(
                apply_text_edits(&source_with_blank_spaces(spaces), &edits),
                EXTRA_BRACE,
                "spaces={spaces}"
            );
        });
    }

    #[test]
    fn edits_for_a_client_with_that_buffer_open_are_pretty_when_applied_to_it() {
        let mut db = db_with(PRETTY.to_owned());
        (0usize..=4).for_each(|spaces| {
            let buffer = source_with_blank_spaces(spaces);
            db.set_open_file(client(), path(), buffer.clone());
            let edits = format_iso_literals(&db, client(), path()).expect("edits");
            assert_eq!(apply_text_edits(&buffer, &edits), PRETTY, "spaces={spaces}");
        });
    }

    pub(crate) const TWO_LITERALS: &str = "const a = iso(`field Query.x{id,name}`)(() => 1);\nconst b = 1;\nconst c = iso(`field Query.y{id}`);\nexport default c;\n";
    pub(crate) const TWO_LITERALS_PRETTY: &str = "const a = iso(`\n  field Query.x {\n    id,\n    name,\n  }\n`)(() => 1);\nconst b = 1;\nconst c = iso(`\n  field Query.y {\n    id,\n  }\n`);\nexport default c;\n";
    pub(crate) const THREE_LITERALS: &str = "const a = iso(`field Query.x{id,name}`);\nconst b = 1;\nconst c = iso(`field Query.y{id}`);\nfoo(c);\nconst d = iso(`field Query.z{a,b,c}`);\nexport default d;\n";
    pub(crate) const THREE_LITERALS_PRETTY: &str = "const a = iso(`\n  field Query.x {\n    id,\n    name,\n  }\n`);\nconst b = 1;\nconst c = iso(`\n  field Query.y {\n    id,\n  }\n`);\nfoo(c);\nconst d = iso(`\n  field Query.z {\n    a,\n    b,\n    c,\n  }\n`);\nexport default d;\n";

    /// Formats `buffer` as client 1's open buffer, checks the edits, and returns the edited text.
    fn formatted_buffer(db: &mut IsographDb<TypeScriptHostLanguage>, buffer: &str) -> String {
        db.set_open_file(client(), path(), buffer.to_owned());
        let edits = format_iso_literals(db, client(), path()).expect("edits");
        assert_edits_replace_disjoint_interiors(buffer, &edits);
        apply_text_edits(buffer, &edits)
    }

    /// `buffer` formats to `expected`, and `expected` formats to itself.
    fn assert_formats_to(buffer: &str, expected: &str) {
        let mut db = db_with(String::new());
        assert_eq!(formatted_buffer(&mut db, buffer), expected);
        assert_eq!(formatted_buffer(&mut db, expected), expected);
    }

    #[test]
    fn two_literals_where_the_first_grows_are_two_edits_that_format_both() {
        let mut db = db_with(String::new());
        db.set_open_file(client(), path(), TWO_LITERALS.to_owned());
        let edits = format_iso_literals(&db, client(), path()).expect("edits");
        assert_eq!(edits.len(), 2);
        assert_formats_to(TWO_LITERALS, TWO_LITERALS_PRETTY);
    }

    #[test]
    fn three_literals_format_all_three() {
        assert_formats_to(THREE_LITERALS, THREE_LITERALS_PRETTY);
    }

    #[test]
    fn two_literals_with_the_second_already_pretty_format_to_both_pretty() {
        assert_formats_to(
            "const a = iso(`field Query.x{id,name}`)(() => 1);\nconst b = 1;\nconst c = iso(`\n  field Query.y {\n    id,\n  }\n`);\nexport default c;\n",
            TWO_LITERALS_PRETTY,
        );
    }

    #[test]
    fn a_first_literal_that_shrinks_formats_the_second() {
        assert_formats_to(
            "const a = iso(`\n\n  field Query.x {\n\n    id,\n\n\n    name,\n\n  }\n\n`)(() => 1);\nconst b = 1;\nconst c = iso(`field Query.y{id}`);\nexport default c;\n",
            TWO_LITERALS_PRETTY,
        );
    }

    #[test]
    fn a_literal_with_parse_errors_between_two_good_ones_is_left_unchanged() {
        assert_formats_to(
            "const a = iso(`field Query.x{id,name}`);\nconst b = iso(`not a declaration`);\nconst c = iso(`field Query.y{id}`);\n",
            "const a = iso(`\n  field Query.x {\n    id,\n    name,\n  }\n`);\nconst b = iso(`not a declaration`);\nconst c = iso(`\n  field Query.y {\n    id,\n  }\n`);\n",
        );
    }

    #[test]
    fn multi_byte_text_before_a_literal_on_its_line_offsets_it_in_utf16() {
        assert_formats_to(
            "const a = iso(`field Pet.x \"Café ü 日本\" {id,name}`); const s = \"日本😀\"; const c = iso(`field Query.y{id}`);\n",
            "const a = iso(`\n  field Pet.x\n  \"Café ü 日本\"\n  {\n    id,\n    name,\n  }\n`); const s = \"日本😀\"; const c = iso(`\n  field Query.y {\n    id,\n  }\n`);\n",
        );
    }

    #[test]
    fn crlf_line_endings_outside_the_literals_are_unchanged() {
        assert_formats_to(
            "const a = iso(`field Query.x{id,name}`);\r\nconst b = 1;\r\nconst c = iso(`\r\n  field Query.y {\r\n\r\n    id,\r\n  }\r\n`);\r\nexport default c;\r\n",
            "const a = iso(`\n  field Query.x {\n    id,\n    name,\n  }\n`);\r\nconst b = 1;\r\nconst c = iso(`\n  field Query.y {\n    id,\n  }\n`);\r\nexport default c;\r\n",
        );
    }

    #[test]
    fn formatting_disk_applied_to_a_buffer_with_a_different_blank_width_is_pretty() {
        (0usize..=4).for_each(|disk_spaces| {
            let disk = source_with_blank_spaces(disk_spaces);
            let db = db_with(disk);
            let edits = format_iso_literals(&db, client(), path()).expect("edits");
            (0usize..=4).for_each(|buffer_spaces| {
                let applied = apply_text_edits(&source_with_blank_spaces(buffer_spaces), &edits);
                assert_eq!(
                    applied, PRETTY,
                    "disk_spaces={disk_spaces} buffer_spaces={buffer_spaces}\n{applied}"
                );
            });
        });
    }
}
