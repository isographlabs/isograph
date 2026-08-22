# Encode every iso literal in the file in one walk

Requires lsp-semantic-token-encoding.md (landed). `lsp_semantic_tokens` takes spans that are already byte offsets into `page_content`. Callers that have parse tokens (relative to one literal) rebase with `with_offset` first. Encoding tests do that as `rebased` / `encode_rebased`. Production must not intern a rebased copy, and must not encode one extraction by itself. LSP `semanticTokens/full` is one delta-encoded array for the document. The encoder takes the file text and every literal in extract order, each with its start offset. One `LineIndex`, one `last_start`, one `previous_token_end`, all in file coordinates. JS between literals has no iso tokens.

Origin of rebase: encoding tests’ `rebased`. Origin of one document stream: LSP `semanticTokens/full`. Delta: the arguments are the file and all `(offset, tokens)` pairs; the walk adds each offset per token and does not collect a rebased vec; there is no function that encodes a single extraction.

file-semantic-tokens.md is the only production caller. It passes every parsed literal in the file.

Offset is `IsoLiteralStartIndex`. `isograph_lsp` depends on `isograph_compiler` for that type. The walk calls `with_offset` with `offset.0 as u32` because `Span` is `u32`. Callers never cast.

One shippable change: new `lsp_semantic_tokens` signature and tests. Existing assertions that used a string as both parse input and `page_content` keep treating that string as a one-interior file whose interior starts at `IsoLiteralStartIndex(0)`.

## What the user does

No editor highlighting. A test whose `page_content` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)` parses that interior and passes one pair: `IsoLiteralStartIndex` of the byte index of `entrypoint` in the file. The first encoded token is keyword, `length` 11, `delta_start` the UTF-16 column of `entrypoint`. A test with two `iso(\`...\`)` interiors in one string is one call with two pairs.

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
use isograph_compiler::IsoLiteralStartIndex;

pub fn lsp_semantic_tokens<'a>(
    page_content: &str,
    literals: impl IntoIterator<Item = (IsoLiteralStartIndex, &'a [WithSpan<IsographSemanticToken>])>,
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

`literals` is every iso interior in `page_content`, left to right. Empty is a file with no iso tokens: `vec![]`.

`lsp_semantic_tokens_with` keeps one `LineCursor`, one `previous_token_end`, one `LastStart`. For each `(offset, tokens)` it walks `tokens` in order. Each span used for `check_span` and `emit_pieces` is `token.location.with_offset(offset.0 as u32)`. It does not collect a rebased vec. It does not encode a pair in isolation and concatenate LSP arrays.

A later literal whose file start is before the previous token’s file end is a caller bug; `check_span` still asserts exclusive ordered spans.

Existing tests that called `lsp_semantic_tokens(tokens, source)` where `source` was the parse input become `lsp_semantic_tokens(source, [(IsoLiteralStartIndex(0), tokens)])`: that fixture is a one-interior file. The test helper `encode` does that. Delete `rebased` and `encode_rebased`. Tests that need JS around the interior pass the file string as `page_content` and `IsoLiteralStartIndex` of the interior’s start.

`lib.rs` still re-exports `lsp_semantic_tokens`. There is no `lsp_semantic_tokens` overload that takes one token slice without the file list.

## Tests

Keep every existing encoding assertion, through `encode` as a one-interior file at `IsoLiteralStartIndex(0)`.

Add:

- `page_content` is `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. Tokens are `parse_iso_literal("entrypoint Query.HomeRoute").tokens`. One pair; offset is `IsoLiteralStartIndex` of the byte index of `entrypoint` in `page_content`. First encoded token: `delta_line` 0, `delta_start` UTF-16 of `export const Home = iso(\``, `length` 11, `token_type` 15.
- Prefix `"const x = 1;\n"` on that same `page_content`. Same tokens, offset is `IsoLiteralStartIndex` of the new start. First token `delta_line` 1, `delta_start` equals the previous test’s `delta_start`.
- Two interiors in one `page_content`: `iso(\`entrypoint Query.A\`)` then later `iso(\`entrypoint Query.B\`)`. One call, two pairs in extract order. First token of the second pair is keyword at `B`’s `entrypoint`: its `delta_line` / `delta_start` place it on that line and column. Tokens remain ordered.
- Same two pairs in reverse order `should_panic` on exclusive ordered spans.
- Empty `literals`: `vec![]`.

`expect` names the fixture string the test built.

## Call sites

- Encoding tests as above.
- file-semantic-tokens.md: `lsp_semantic_tokens_for_file` passes the file text and every `(*start_index, parsed.tokens)` in extract order. One call.
