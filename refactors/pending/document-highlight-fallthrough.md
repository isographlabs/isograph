# Document highlight: empty inside a literal, null outside

VS Code asks document highlight providers in order and stops at the first non-null result. An empty array is a final answer. `null` falls through to the next provider, TypeScript, which highlights the whole literal content.

## End-user experience

- On a semantic token inside an iso literal (a name, a keyword, `.`), the editor highlights that token, as today.
- Inside an iso literal with no token under the cursor (whitespace, a comma, trailing whitespace after the last token), the editor highlights nothing. The server returns `[]`, so TypeScript's whole-literal highlight does not apply.
- Outside every literal (TypeScript code, `iso(`, either backtick, past the end of a line or the file, no file, a non-file URI), the server returns `null` and TypeScript's highlighting applies.

`.` is a semantic token (`IsographSemanticToken::Period`), so it is highlighted, not `[]`.

## Types

```rust
// from crates/isograph_lsp/src/document_highlight.rs
/// What lies under a position inside an iso literal.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TokenUnderCursor<TLocation> {
    Token(TLocation),
    BetweenTokens,
}
```

`TLocation` is `Span` inside the literal, `Span` in the file, then `lsp_types::Range`. The outer `Option` of each function below is `None` when the position is not inside a literal.

## Functions

Before:

```rust
// from crates/isograph_lsp/src/document_highlight.rs
pub fn token_range_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<lsp_types::Range> {
    let span = token_span_at_lsp_position(db, client, path, position)?;
    line_index(db, client, path)
        .as_ref()?
        .range(span)
        .wrap_some()
}
```

After:

```rust
// from crates/isograph_lsp/src/document_highlight.rs
/// The file range of the semantic token under `position` in `path` as `client` reads it.
/// `None` when `position` is not inside an iso literal.
pub fn token_range_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<TokenUnderCursor<lsp_types::Range>> {
    match token_span_at_lsp_position(db, client, path, position)? {
        TokenUnderCursor::Token(span) => {
            TokenUnderCursor::Token(line_index(db, client, path).as_ref()?.range(span))
        }
        TokenUnderCursor::BetweenTokens => TokenUnderCursor::BetweenTokens,
    }
    .wrap_some()
}

/// The file span of the semantic token under `position`.
fn token_span_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<TokenUnderCursor<Span>> {
    let (literal_id, token) = literal_token_at_lsp_position(db, client, path, position)?;
    match token {
        TokenUnderCursor::Token(span) => {
            let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
            TokenUnderCursor::Token(span.with_offset(extraction.iso_literal_start_index.0 as u32))
        }
        TokenUnderCursor::BetweenTokens => TokenUnderCursor::BetweenTokens,
    }
    .wrap_some()
}

/// The literal under `position`, and what lies under it within that literal.
fn literal_token_at_lsp_position<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
    position: lsp_types::Position,
) -> Option<(LiteralId, TokenUnderCursor<Span>)> {
    let (literal_id, cursor) = iso_literal_at_lsp_position(db, client, path, position)?;
    let parsed = parsed_iso_literal_by_id(db, literal_id)?;
    let token = semantic_token_span_at(&parsed.tokens, cursor)
        .map_or(TokenUnderCursor::BetweenTokens, TokenUnderCursor::Token);
    (literal_id, token).wrap_some()
}
```

`semantic_token_span_at` is unchanged. `lib.rs` also exports `TokenUnderCursor`.

Handler, before:

```rust
// from crates/isograph_cli/src/adapter.rs
    isograph_lsp::token_range_at_lsp_position(db, client, path, position.position)
        .map(|range| {
            lsp_types::DocumentHighlight {
                range,
                kind: lsp_types::DocumentHighlightKind::TEXT.wrap_some(),
            }
            .wrap_vec()
        })
        .wrap_ok()
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
    isograph_lsp::token_range_at_lsp_position(db, client, path, position.position)
        .map(|token| match token {
            isograph_lsp::TokenUnderCursor::Token(range) => lsp_types::DocumentHighlight {
                range,
                kind: lsp_types::DocumentHighlightKind::TEXT.wrap_some(),
            }
            .wrap_vec(),
            isograph_lsp::TokenUnderCursor::BetweenTokens => Vec::new(),
        })
        .wrap_ok()
```

## Tests

`state.rs`: whitespace inside the literal is `[]`; trailing whitespace after the last token is `[]`; a comma in `field Query.x { a, b }` is `[]`; `.` is the `.`'s highlight; JS outside, `iso(`, the opening and the closing backtick, past the line end, past the end of the file, no file, and a non-file URI are `null`.
