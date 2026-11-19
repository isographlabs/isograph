use isograph_compiler::HostLanguage;
use pico::Database;
use prelude::Postfix;

use crate::event::ClientId;

pub(crate) fn semantic_tokens_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    _client: ClientId,
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
    _client: ClientId,
    params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    format_iso_literals(state, params.text_document.uri.reference()).wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: HostLanguage>(
    state: &isograph_compiler::IsographDb<THostLanguage>,
    _client: ClientId,
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

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use intern::string_key::Intern;
    use isograph_compiler::IsographDb;
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

    fn uri() -> lsp_types::Uri {
        lsp_types::Uri::from_str("file:///tmp/proj/src/Pet.ts").expect("a file URI")
    }

    fn source_with_blank_spaces(spaces: usize) -> String {
        format!(
            "export const fullName = iso(`\n  field Pet.fullName @foo(\n    bar: 123,\n    baz: $qux,\n  ) {{\n    id,\n{}\n  }}\n`);\n",
            " ".repeat(spaces)
        )
    }

    fn byte_offset(source: &str, position: lsp_types::Position) -> usize {
        let mut rest = source;
        let mut offset = 0;
        for _ in 0..position.line {
            let Some((line, after)) = rest.split_once('\n') else {
                return source.len();
            };
            offset += line.len() + 1;
            rest = after;
        }
        let line = rest.split('\n').next().unwrap_or(rest);
        let mut utf16 = 0u32;
        for (index, ch) in line.char_indices() {
            if utf16 == position.character {
                return offset + index;
            }
            utf16 += ch.len_utf16() as u32;
        }
        offset + line.len()
    }

    fn apply_text_edits(source: &str, edits: &[lsp_types::TextEdit]) -> String {
        let mut ordered: Vec<&lsp_types::TextEdit> = edits.iter().collect();
        ordered.sort_by(|left, right| {
            right
                .range
                .start
                .line
                .cmp(&left.range.start.line)
                .then(right.range.start.character.cmp(&left.range.start.character))
        });
        ordered
            .into_iter()
            .fold(source.to_owned(), |mut out, edit| {
                let start = byte_offset(&out, edit.range.start);
                let end = byte_offset(&out, edit.range.end);
                out.replace_range(start..end, &edit.new_text);
                out
            })
    }

    fn formatted_source(contents: String) -> String {
        let db = db_with(contents.clone());
        let uri = uri();
        let edits = format_iso_literals(&db, uri.reference()).expect("edits");
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
            let uri = uri();
            let edits = format_iso_literals(&db, uri.reference()).expect("edits");
            let once = apply_text_edits(&source, &edits);
            assert_eq!(once, PRETTY, "spaces={spaces}");
            let twice = apply_text_edits(&once, &edits);
            assert!(!twice.contains("`);"), "spaces={spaces}\n{twice}");
        });
    }

    #[test]
    fn pretty_disk_edits_applied_to_a_buffer_with_an_extra_blank_line_leave_an_extra_brace() {
        const EXTRA_BRACE: &str = "export const fullName = iso(`\n  field Pet.fullName @foo(\n    bar: 123,\n    baz: $qux,\n  ) {\n    id,\n  }\n  }\n`);\n";
        let db = db_with(PRETTY.to_owned());
        let uri = uri();
        let edits = format_iso_literals(&db, uri.reference()).expect("edits");
        (0usize..=4).for_each(|spaces| {
            assert_eq!(
                apply_text_edits(&source_with_blank_spaces(spaces), &edits),
                EXTRA_BRACE,
                "spaces={spaces}"
            );
        });
    }

    #[test]
    fn formatting_disk_applied_to_a_buffer_with_a_different_blank_width_is_pretty() {
        (0usize..=4).for_each(|disk_spaces| {
            let disk = source_with_blank_spaces(disk_spaces);
            let db = db_with(disk);
            let uri = uri();
            let edits = format_iso_literals(&db, uri.reference()).expect("edits");
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
