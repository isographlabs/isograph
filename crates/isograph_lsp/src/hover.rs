use common_lang_types::RelativePathToSourceFile;
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, LiteralId, extract_iso_literals,
    iso_literal_extraction, parsed_iso_literal_by_id,
};
use isograph_parser::{IsographResolutionNode, ParsedIsoLiteral, resolve_offset};
use prelude::Postfix;
use span::Span;

use crate::line_index::{byte_offset_at_lsp_position, line_index};

/// The hover at `position` in `path` as `client` reads it. Its range is the char under
/// `position`.
pub fn hover_for_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Hover> {
    let (literal_id, node, cursor) = resolved_position_at_lsp_position(db, client, path, position)?;
    let span = file_span_of_literal_span(db, literal_id, cursor)?;
    lsp_types::Hover {
        contents: lsp_types::HoverContents::Markup(lsp_types::MarkupContent {
            kind: lsp_types::MarkupKind::Markdown,
            value: node.to_string(),
        }),
        range: line_index(db, client, path)
            .as_ref()?
            .range(span)
            .wrap_some(),
    }
    .wrap_some()
}

fn resolved_position_at_lsp_position<'db, TProfile: CompilationProfile>(
    db: &'db IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, IsographResolutionNode<'db>, Span)> {
    let (literal_id, parsed, cursor) = ast_at_lsp_position(db, client, path, position)?;
    (literal_id, resolve_offset(parsed, cursor)?, cursor).wrap_some()
}

fn ast_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, &ParsedIsoLiteral, Span)> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    (
        literal_id,
        parsed_iso_literal_by_id(db, literal_id)?,
        cursor,
    )
        .wrap_some()
}

/// The file span of `span`, a span relative to the interior of `literal_id`.
pub(crate) fn file_span_of_literal_span<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    literal_id: LiteralId,
    span: Span,
) -> Option<Span> {
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    span.with_offset(extraction.iso_literal_start_index.0 as u32)
        .wrap_some()
}

/// The literal whose interior contains `position`, and the span of the char under
/// `position` relative to that interior.
pub(crate) fn iso_literal_at_lsp_position<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Span)> {
    let offset = byte_offset_at_lsp_position(db, client, path, position)?;
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .enumerate()
        .find_map(|(index, extraction)| {
            let within = offset.checked_sub(extraction.iso_literal_start_index.0 as u32)?;
            let character = extraction
                .iso_literal_text
                .get(within as usize..)?
                .chars()
                .next()?;
            (
                LiteralId {
                    client,
                    path,
                    index,
                },
                Span::new(within, within + character.len_utf8() as u32),
            )
                .wrap_some()
        })
}

#[cfg(test)]
mod tests {
    use crate::test_profile::TestProfile;
    use common_lang_types::RelativePathToSourceFile;
    use intern::string_key::Intern;
    use isograph_compiler::{ClientId, IsographDb, LiteralId, parsed_iso_literal};
    use lsp_types::Position;
    use prelude::Postfix;
    use span::Span;

    use super::{ast_at_lsp_position, hover_for_position, iso_literal_at_lsp_position};

    fn client() -> ClientId {
        ClientId(1)
    }

    fn intern_path(s: &str) -> RelativePathToSourceFile {
        s.intern().to()
    }

    fn db_with(contents: &str) -> (IsographDb<TestProfile>, RelativePathToSourceFile) {
        let mut db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        db.insert_disk_file(path, contents.to_owned());
        (db, path)
    }

    fn at(line: u32, character: u32) -> Position {
        Position { line, character }
    }

    fn literal_at(
        db: &IsographDb<TestProfile>,
        path: RelativePathToSourceFile,
        position: Position,
    ) -> Option<(LiteralId, Span)> {
        iso_literal_at_lsp_position(db, client(), path, position)
    }

    fn index(path: RelativePathToSourceFile, index: usize) -> LiteralId {
        LiteralId {
            client: client(),
            path,
            index,
        }
    }

    fn column_of(contents: &str, needle: &str) -> u32 {
        contents
            .find(needle)
            .expect("the fixture contains the needle") as u32
    }

    fn hover_range(
        db: &IsographDb<TestProfile>,
        path: RelativePathToSourceFile,
        position: Position,
    ) -> Option<Option<lsp_types::Range>> {
        hover_for_position(db, client(), path, position).map(|hover| hover.range)
    }

    /// The range on `line` from UTF-16 column `start` to `end`, as a hover range.
    fn char_range(line: u32, start: u32, end: u32) -> Option<Option<lsp_types::Range>> {
        lsp_types::Range {
            start: at(line, start),
            end: at(line, end),
        }
        .wrap_some()
        .wrap_some()
    }

    /// The UTF-16 column of the `nth` (0-based) `needle` in the one-line `contents`.
    fn utf16_column_of(contents: &str, needle: &str, nth: usize) -> u32 {
        let (byte, _) = contents
            .match_indices(needle)
            .nth(nth)
            .expect("the fixture contains the needle");
        contents[..byte].encode_utf16().count() as u32
    }

    #[test]
    fn no_file_is_none() {
        let db = IsographDb::<TestProfile>::default();
        let path = intern_path("src/a.ts");
        assert_eq!(literal_at(&db, path, at(0, 0)), None);
        assert_eq!(ast_at_lsp_position(&db, client(), path, at(0, 0)), None);
    }

    #[test]
    fn one_line_entrypoint_is_index_zero_from_the_first_to_the_last_interior_byte() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let (db, path) = db_with(contents);
        let interior = "entrypoint Query.HomeRoute";
        let start = column_of(contents, interior);
        let last = interior.len() as u32 - 1;
        assert_eq!(
            literal_at(&db, path, at(0, start)),
            (index(path, 0), Span::new(0, 1)).wrap_some()
        );
        assert_eq!(
            literal_at(&db, path, at(0, start + last)),
            (index(path, 0), Span::new(last, last + 1)).wrap_some()
        );
        let (_, parsed, _) = ast_at_lsp_position(&db, client(), path, at(0, start))
            .expect("the interior is inside the literal");
        assert_eq!(parsed, parsed_iso_literal(&db, interior.to_owned()));
        assert!(parsed.errors.is_empty());
    }

    #[test]
    fn iso_and_both_backticks_are_outside_the_literal() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let (db, path) = db_with(contents);
        let opening = column_of(contents, "`");
        let closing = contents
            .rfind('`')
            .expect("the fixture has a closing backtick") as u32;
        assert_eq!(literal_at(&db, path, at(0, 0)), None);
        assert_eq!(literal_at(&db, path, at(0, opening)), None);
        assert_eq!(literal_at(&db, path, at(0, closing)), None);
        assert_eq!(
            ast_at_lsp_position(&db, client(), path, at(0, closing)),
            None
        );
    }

    #[test]
    fn incomplete_entrypoint_has_parse_errors() {
        let contents = "iso(`entrypoint`)";
        let (db, path) = db_with(contents);
        let (_, parsed, _) = ast_at_lsp_position(
            &db,
            client(),
            path,
            at(0, column_of(contents, "entrypoint")),
        )
        .expect("the interior is inside the literal");
        assert!(!parsed.errors.is_empty());
    }

    #[test]
    fn multiline_literal_is_found_on_its_interior_line() {
        let (db, path) = db_with("iso(`\nentrypoint Query.HomeRoute\n`)");
        assert_eq!(
            literal_at(&db, path, at(1, 0)),
            (index(path, 0), Span::new(1, 2)).wrap_some()
        );
        assert_eq!(literal_at(&db, path, at(0, 0)), None);
        assert_eq!(literal_at(&db, path, at(2, 0)), None);
    }

    #[test]
    fn two_literals_on_one_line_are_index_zero_and_one() {
        let contents = "iso(`field Pet.fullName { id }`) iso(`entrypoint Query.HomeRoute`)";
        let (db, path) = db_with(contents);
        assert_eq!(
            literal_at(&db, path, at(0, column_of(contents, "field")))
                .map(|(literal_id, _)| literal_id),
            index(path, 0).wrap_some()
        );
        assert_eq!(
            literal_at(&db, path, at(0, column_of(contents, "entrypoint")))
                .map(|(literal_id, _)| literal_id),
            index(path, 1).wrap_some()
        );
        assert_eq!(
            literal_at(&db, path, at(0, column_of(contents, ") iso"))),
            None
        );
    }

    #[test]
    fn the_same_text_in_two_files_is_the_same_parse() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let mut db = IsographDb::<TestProfile>::default();
        let a = intern_path("src/a.ts");
        let b = intern_path("src/b.ts");
        db.insert_disk_file(a, contents.to_owned());
        db.insert_disk_file(b, contents.to_owned());
        let position = at(0, column_of(contents, "entrypoint"));
        let (_, in_a, _) =
            ast_at_lsp_position(&db, client(), a, position).expect("file a has the literal");
        let (_, in_b, _) =
            ast_at_lsp_position(&db, client(), b, position).expect("file b has the literal");
        assert_eq!(in_a, in_b);
    }

    #[test]
    fn a_prepended_line_moves_the_literal_to_the_next_line() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let (mut db, path) = db_with(contents);
        let character = column_of(contents, "entrypoint");
        let before = literal_at(&db, path, at(0, character));
        db.insert_disk_file(path, "const x = 1;\n".to_owned() + contents);
        assert_eq!(literal_at(&db, path, at(1, character)), before);
        assert_eq!(literal_at(&db, path, at(0, character)), None);
    }

    #[test]
    fn lengthening_an_earlier_line_keeps_the_literal_at_the_position() {
        let (mut db, path) = db_with("const x = 1;\niso(`entrypoint Query.HomeRoute`)");
        let before = literal_at(&db, path, at(1, 5));
        assert_eq!(before, (index(path, 0), Span::new(0, 1)).wrap_some());
        db.insert_disk_file(
            path,
            "const x = 1; const y = 2;\niso(`entrypoint Query.HomeRoute`)".to_owned(),
        );
        assert_eq!(literal_at(&db, path, at(1, 5)), before);
    }

    #[test]
    fn a_buffer_names_its_literal_for_its_client_and_disk_for_another() {
        let (mut db, path) = db_with("iso(`entrypoint Query.Disk`)");
        db.set_open_file(client(), path, "iso(`entrypoint Query.Buffer`)".to_owned());
        let position = at(0, "iso(`".len() as u32);
        let text_for = |client: ClientId| {
            let (literal_id, _) = iso_literal_at_lsp_position(&db, client, path, position)
                .expect("the position is inside that client's literal");
            isograph_compiler::iso_literal_extraction(&db, literal_id)
                .as_ref()
                .expect("the LiteralId indexes that client's text")
                .iso_literal_text
                .clone()
        };
        assert_eq!(text_for(client()), "entrypoint Query.Buffer");
        assert_eq!(text_for(ClientId(2)), "entrypoint Query.Disk");
    }

    #[test]
    fn hover_range_is_the_first_and_the_last_interior_char() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let (db, path) = db_with(contents);
        let start = column_of(contents, "entrypoint");
        let last = column_of(contents, "`)") - 1;
        assert_eq!(
            hover_range(&db, path, at(0, start)),
            char_range(0, start, start + 1)
        );
        assert_eq!(
            hover_range(&db, path, at(0, last)),
            char_range(0, last, last + 1)
        );
    }

    #[test]
    fn hover_range_after_a_wide_char_on_the_line_is_in_utf16_columns() {
        for prefix in ["const s = \"😀\"; ", "const s = \"é\"; "] {
            let contents = format!("{prefix}iso(`entrypoint Query.A`)");
            let (db, path) = db_with(&contents);
            let query = utf16_column_of(&contents, "Query", 0);
            assert_eq!(
                hover_range(&db, path, at(0, query)),
                char_range(0, query, query + 1),
                "after {prefix:?}"
            );
        }
    }

    #[test]
    fn hover_range_on_argument_colons_and_commas_is_that_char() {
        let contents = "iso(`field Pet.fullName @foo(bar: 123, baz: $qux) { id }`)";
        let (db, path) = db_with(contents);
        for (needle, nth) in [(":", 0), (":", 1), (",", 0)] {
            let character = utf16_column_of(contents, needle, nth);
            assert_eq!(
                hover_range(&db, path, at(0, character)),
                char_range(0, character, character + 1),
                "{needle:?} {nth}"
            );
        }
    }

    #[test]
    fn hover_range_on_a_multi_byte_char_in_a_string_literal_spans_its_utf16_units() {
        let contents = "iso(`field Pet.fullName @foo(bar: \"é😀\") { id }`)";
        let (db, path) = db_with(contents);
        let e_acute = utf16_column_of(contents, "é", 0);
        let emoji = utf16_column_of(contents, "😀", 0);
        assert_eq!(
            hover_range(&db, path, at(0, e_acute)),
            char_range(0, e_acute, e_acute + 1)
        );
        assert_eq!(
            hover_range(&db, path, at(0, emoji)),
            char_range(0, emoji, emoji + 2)
        );
    }

    #[test]
    fn hover_range_in_a_multiline_literal_is_on_the_cursor_line() {
        let (db, path) = db_with("iso(`\nentrypoint Query.HomeRoute\n`)");
        assert_eq!(hover_range(&db, path, at(1, 11)), char_range(1, 11, 12));
    }

    #[test]
    fn hover_outside_the_literal_is_none() {
        let contents = "iso(`entrypoint Query.HomeRoute`)";
        let (db, path) = db_with(contents);
        let closing = contents
            .rfind('`')
            .expect("the fixture has a closing backtick") as u32;
        assert_eq!(hover_range(&db, path, at(0, 0)), None);
        assert_eq!(hover_range(&db, path, at(0, closing)), None);
    }
}
