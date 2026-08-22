# Encode relative tokens with a literal start offset

Requires lsp-semantic-token-encoding.md (landed). `lsp_semantic_tokens` takes spans that are already byte offsets into `page_content`. Callers that have parse tokens (relative to the literal) rebase with `with_offset` first. Encoding tests do that as `rebased` / `encode_rebased`. Production must not intern a rebased copy. The encoder takes each literal’s start offset and the file text. One call handles every literal in the file. `last_start` and `previous_token_end` are file coordinates and continue across literals.

Origin of rebase: encoding tests’ `rebased`. Origin of one document stream: LSP `semanticTokens/full`. Delta: offset is an argument; the walk adds it per token and does not collect a new vec; a sequence of `(offset, tokens)` is one encode.

file-semantic-tokens.md calls this. It does not expose file-absolute `WithSpan<IsographSemanticToken>`.

Offset is `u32`. `IsoLiteralStartIndex` stays in the compiler. Callers pass `.0 as u32`. `isograph_lsp` does not depend on `isograph_compiler` for this change.

One shippable change: new `lsp_semantic_tokens` signature and tests. Existing offset-0 tests keep the same assertions.

## What the user does

No editor highlighting. Tests that encoded a literal as the whole `page_content` still pass with offset `0`. A test whose `page_content` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)` and whose tokens are the parse of `entrypoint Query.HomeRoute` passes the start index of that interior as offset. The first encoded token is keyword, `length` 11, `delta_start` the UTF-16 column of `entrypoint`.

## Types

Before:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
pub fn lsp_semantic_tokens(
    tokens: &[WithSpan<IsographSemanticToken>],
    page_content: &str,
) -> Vec<lsp_types::SemanticToken>
```

After:

```rust
// from crates/isograph_lsp/src/semantic_tokens.rs
pub fn lsp_semantic_tokens<'a>(
    page_content: &str,
    literals: impl IntoIterator<Item = (u32, &'a [WithSpan<IsographSemanticToken>])>,
) -> Vec<lsp_types::SemanticToken> {
    let index = LineIndex::new(page_content);
    if page_content.is_ascii() {
        lsp_semantic_tokens_with(&index, literals, |text| text.len() as u32)
    } else {
        lsp_semantic_tokens_with(&index, literals, |text| {
            text.encode_utf16().count() as u32
        })
    }
}
```

`lsp_semantic_tokens_with` keeps one `LineCursor`, one `previous_token_end`, one `LastStart`, all in file coordinates. For each `(offset, tokens)` it walks `tokens` in order. Each span used for `check_span` and `emit_pieces` is `token.location.with_offset(offset)`. It does not collect a rebased vec.

One literal whose text is the whole `page_content` is `lsp_semantic_tokens(page_content, [(0, tokens)])`. Existing tests that called `lsp_semantic_tokens(tokens, source)` become that. The test helper `encode` does `lsp_semantic_tokens(source, [(0, tokens)])`. Delete `rebased` and `encode_rebased`; tests that used them pass the file string as `page_content` and the literal start as offset.

Empty `literals` is `vec![]`. JS between literals has no iso tokens. A later literal whose file start is before the previous token’s file end is a caller bug; `check_span` still asserts exclusive ordered spans.

`lib.rs` still re-exports `lsp_semantic_tokens`.

## Tests

Keep every existing encoding assertion, through `encode` with offset `0`.

Add:

- `page_content` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. Tokens are `parse_iso_literal("entrypoint Query.HomeRoute").tokens`. Offset is the byte index of `entrypoint` in `page_content`. First encoded token: `delta_line` 0, `delta_start` UTF-16 of `export const Home = iso(\``, `length` 11, `token_type` 15.
- Prefix `"const x = 1;\n"` on that same `page_content`. Same tokens, offset is the new start index. First token `delta_line` 1, `delta_start` equals the previous test’s `delta_start`.
- Two interiors in one `page_content`: `iso(\`entrypoint Query.A\`)` then later `iso(\`entrypoint Query.B\`)`. One call, two `(offset, tokens)` pairs in extract order. First token of the second pair is keyword at `B`’s `entrypoint`: its `delta_line` / `delta_start` place it on that line and column. Tokens remain ordered.
- Same two pairs in reverse order `should_panic` on exclusive ordered spans.

`expect` names the fixture string the test built.

## Call sites

- Encoding tests as above.
- file-semantic-tokens.md: `lsp_semantic_tokens_for_file` passes the file text and `(start_index.0 as u32, parsed.tokens)` per literal.
