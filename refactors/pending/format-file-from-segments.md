# Format a file from segments

Requires iso-format.md (landed) and lsp-open-files.md change 7 (landed; open decision 2 (a), lsp-open-files.md first). Independent of one-save-format-apply.md. `format_iso_literals` takes `client: ClientId` and reads `isograph_compiler::file_contents(db, client, path)`.

`format_iso_literals` emits one `TextEdit` per iso interior. Each range is a byte span converted through `text_through_last_iso_literal`. Those ranges are positions in the interned snapshot.

Two interiors in one file are two ranges. Applying them depends on reverse order. A file is one string: the text before the first interior, then each interior plus the text that follows it until the next interior or EOF.

`adapter::tests::edits_for_a_client_without_a_buffer_applied_to_a_longer_buffer_leave_an_extra_brace` is disk ranges applied to a longer string. lsp-open-files.md change 7 fixed the user-visible case: a client with the buffer open formats that buffer (`edits_for_a_client_with_that_buffer_open_are_pretty_when_applied_to_it`). This slice stops using N interior ranges.

## What the user does

A file with two iso literals. Save (or Format Document). Both interiors format. The JavaScript between them is unchanged. A failed parse in one literal is copied through; the other still formats.

A file with one literal still formats as today when the interned text is the text the editor applies to.

## Types

No new types. `IsoLiteralExtraction` and `ParsedIsoLiteral` stay.

```rust
// from crates/isograph_compiler/src/host_language.rs
pub struct IsoLiteralExtraction<THostLanguage: HostLanguage> {
    pub iso_literal_text: String,
    pub iso_literal_start_index: IsoLiteralStartIndex,
    pub context: THostLanguage::LiteralContext,
}
```

`iso_literal_start_index` is the byte offset of the interior in the file. `iso_literal_text.len()` is the interior length. The gap after that interior is `contents[end..next_start]` or `contents[end..]` after the last.

## `rewrite_iso_interiors`

Called only by `format_iso_literals`. Not pub.

```rust
// from crates/isograph_cli/src/adapter.rs
fn rewrite_iso_interiors<THostLanguage: HostLanguage>(
    contents: &str,
    literals: &[(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)],
) -> String {
    let (mut out, last) = literals.iter().fold(
        (String::new(), 0usize),
        |(mut out, last), (extraction, parsed)| {
            let start = extraction.iso_literal_start_index.0;
            let end = start + extraction.iso_literal_text.len();
            out.push_str(&contents[last..start]);
            if parsed.errors.is_empty() {
                out.push_str(&isograph_parser::format_recorded(
                    extraction.iso_literal_text.as_str(),
                    parsed.tokens.as_slice(),
                ));
            } else {
                out.push_str(&contents[start..end]);
            }
            (out, end)
        },
    );
    out.push_str(&contents[last..]);
    out
}
```

Empty `literals`: fold leaves `last == 0`, then `contents[0..]` is the file. `format_iso_literals` today returns `None` when `parsed_iso_literals_in_file` is `None` (no disk file / no extract). Keep that. A present file with no iso still has `Some(vec[])` from extract; rewrite is the file unchanged; one identity `TextEdit` is fine.

## `format_iso_literals`

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
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
```

After: full interned file, one edit of the whole file. No `text_through_last_iso_literal`.

```rust
// from crates/isograph_cli/src/adapter.rs
fn format_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<lsp_types::TextEdit>> {
    let literals = isograph_compiler::parsed_iso_literals_in_file(db, client, path).as_ref()?;
    let contents = isograph_compiler::file_contents(db, client, path).as_ref()?;
    let new_text = rewrite_iso_interiors(contents, literals);
    lsp_types::TextEdit {
        range: isograph_lsp::lsp_range_for_byte_range(contents, 0, contents.len() as u32),
        new_text,
    }
    .wrap_vec()
    .wrap_some()
}
```

`formatting_response` and `will_save_wait_until_response` still call `format_iso_literals`. They do not change.

`text_through_last_iso_literal` stays for semantic tokens. Format does not use it.

## Tests

Keep the existing adapter tests. They still apply `format_iso_literals` edits to a string.

Add, in `adapter.rs` `#[cfg(test)]`:

Two interned literals, JavaScript between them. After one apply, both interiors are the pretty wrap of `entrypoint Query.A` and `entrypoint Query.B`, and `const mid = 1;` is still between them.

```rust
// from crates/isograph_cli/src/adapter.rs
    #[test]
    fn two_literals_keep_the_javascript_between_them() {
        let source = "iso(`entrypoint Query.A`);\nconst mid = 1;\niso(`entrypoint Query.B`);\n";
        let out = formatted_source(source.to_owned());
        assert!(out.contains("const mid = 1;"), "{out}");
        assert_eq!(
            out,
            "iso(`\n  entrypoint Query.A\n`);\nconst mid = 1;\niso(`\n  entrypoint Query.B\n`);\n"
        );
    }
```

Confirm the pretty wrap of `entrypoint Query.HomeRoute` is `"\n  entrypoint Query.HomeRoute\n"` (`format::tests::entrypoint_is_on_its_own_indented_line`). Use that in the assertion. If the exact string above is wrong, the test is the source of truth: `formatted("entrypoint Query.A")` concatenated into the file shape.

A failed literal next to a good one: the failed interior is unchanged, the good one is pretty, the JS around both is unchanged. Same `formatted_source` helper.

`lsp_socket.rs` `formatting_of_an_interned_iso_literal_edits_the_interior` today asserts `edits.len() == 1` and `new_text` is the pretty interior. After this slice there is still one edit; `new_text` is the whole file with that interior rewritten. Update that assertion to the full rewritten file, or to `apply` the edit onto the interned source and assert the pretty file. Prefer apply-and-assert-file so the test names the user-visible result.

`formatting_skips_a_failed_literal_and_edits_the_good_one` currently `edits.len() == 1` (only the good interior). After: one whole-file edit; the failed interior is still the source text; the good interior is pretty.

`formatting_of_a_never_interned_uri_is_null` stays.

## Who calls what

`format_iso_literals` calls `rewrite_iso_interiors` and `lsp_range_for_byte_range` on the full interned contents. `parsed_iso_literals_in_file` still provides the list. `file_contents(db, client, path)` provides `contents`. Callers of `format_iso_literals` are unchanged.
