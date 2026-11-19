# LSP document highlight

The server answers `textDocument/documentHighlight`. Inside an iso literal, the result is the range of the semantic token under the cursor, as one `DocumentHighlight` of kind `Text`. Anywhere else the result is `null`.

## End-user experience

The editor advertises the feature because `initialize` returns `"documentHighlightProvider": true`. The user puts the caret on `HomeRoute` in

```ts
export const Home = iso(`entrypoint Query.HomeRoute`);
```

and the editor highlights `HomeRoute`. On `Query` it highlights `Query`. On `.` it highlights the `.`, because `.` is a semantic token (`IsographSemanticToken::Period`) and semantic tokens color it. On the space between `entrypoint` and `Query`, on JS outside the literal, on a backtick, past the end of a line, and past the end of the file, nothing is highlighted (`null`). Commas are not highlighted, matching semantic tokens, which skip `IsographSemanticToken::Comma`.

The text is the client's open buffer, or disk when that client has no buffer, and columns are UTF-16, as for hover.

## The chain

```text
document_highlight_response(db, client, params)              isograph_cli adapter.rs
  -> token_range_at_lsp_position(db, client, path, position)  isograph_lsp document_highlight.rs
    -> token_span_at_lsp_position(...)                        isograph_lsp document_highlight.rs
      -> literal_token_at_lsp_position(...)                   isograph_lsp document_highlight.rs
        -> iso_literal_at_lsp_position(...)                   isograph_lsp hover.rs (now pub(crate))
        -> parsed_iso_literal_by_id(db, literal_id)
        -> semantic_token_span_at(&parsed.tokens, cursor)
      -> iso_literal_extraction(db, literal_id)
    -> line_index(db, client, path)
```

All functions that take a position are plain functions.

```rust
// from crates/isograph_lsp/src/document_highlight.rs
/// The file range of the semantic token under `position` in `path` as `client` reads it.
pub fn token_range_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Range> {
    let span = token_span_at_lsp_position(db, client, path, position)?;
    line_index(db, client, path).as_ref()?.range(span).wrap_some()
}

/// The file span of the semantic token under `position`.
fn token_span_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<Span> {
    let (literal_id, token) = literal_token_at_lsp_position(db, client, path, position)?;
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    token
        .with_offset(extraction.iso_literal_start_index.0 as u32)
        .wrap_some()
}

/// The literal under `position`, and the span within it of the semantic token there.
fn literal_token_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, Span)> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    let parsed = parsed_iso_literal_by_id(db, literal_id)?;
    (literal_id, semantic_token_span_at(&parsed.tokens, cursor)?).wrap_some()
}

/// The span of the token that contains `cursor`. Tokens without a span and commas are
/// not candidates; semantic tokens do not emit them.
fn semantic_token_span_at(
    tokens: &[WithOptionalSpan<IsographSemanticToken>],
    cursor: Span,
) -> Option<Span> {
    tokens
        .iter()
        .filter(|token| token.item != IsographSemanticToken::Comma)
        .filter_map(|token| token.location)
        .find(|span| span.contains(cursor))
}
```

`lib.rs` adds `mod document_highlight;` and `pub use document_highlight::token_range_at_lsp_position;`. `hover.rs` makes `iso_literal_at_lsp_position` `pub(crate)`.

```rust
// from crates/isograph_cli/src/adapter.rs
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
```

`DocumentHighlightRequest::Result` is `Option<Vec<DocumentHighlight>>`. Registration in `dispatch_lsp_request`, after `HoverRequest`:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::DocumentHighlightRequest>(
                crate::adapter::document_highlight_response::<THostLanguage>,
            )?
```

Capability in `run_session` (`lsp_socket.rs`); the lsp-types 0.97 field is `Option<OneOf<bool, DocumentHighlightOptions>>`:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        document_highlight_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

## Tests

`document_highlight.rs`: `semantic_token_span_at` over `parse_iso_literal("entrypoint Query.HomeRoute").tokens` on each char of `HomeRoute` is the `HomeRoute` span; on the space is `None`; on `.` is the `.` span; on trailing whitespace past the last token is `None`; on a comma in `field Query.x { a, b }` is `None`.

`state.rs` (the `handle` seam):

- On `HomeRoute` the result is one highlight of kind `Text` whose range is exactly `HomeRoute`'s line and UTF-16 columns.
- On `.` the range is exactly the `.`.
- On whitespace, on JS outside the literal, and past the line end the result is `null`.
- Two literals: each highlights its own token.
- After `😀` before the literal, the range is in UTF-16 columns.
- Buffer over disk: client 1 with a buffer highlights the buffer's token; client 2 highlights disk's.
- A non-file URI is `null`.

`lsp_socket.rs`'s initialize capabilities test asserts `documentHighlightProvider` is `true` and is renamed `initialize_advertises_formatting_will_save_wait_until_hover_and_highlight`.

## Changes

1. `document_highlight.rs`, `document_highlight_response`, registration, capability, tests.
