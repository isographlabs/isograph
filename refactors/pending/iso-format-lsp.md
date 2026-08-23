# LSP format requests

Requires lsp-tokens.md (landed). Independent of iso-format.md, lsp-open-files.md, lsp-diagnostics.md.

The server advertises `documentFormattingProvider` and `willSaveWaitUntil`. Both requests return `[]`. iso-format.md puts interiors in the same handlers.

Origin of the methods: `lsp_types::request::Formatting` and `lsp_types::request::WillSaveWaitUntil`. vscode-languageclient sends `willSaveWaitUntil` when the server advertises it.

One shippable change.

## What the user does

Initialize lists formatting and `willSaveWaitUntil`. `textDocument/formatting` and `textDocument/willSaveWaitUntil` respond with `result: []`. Save applies no edits.

## Types

Most important first.

Insert before `.request()` in `dispatch_lsp_request`, after the semantic-tokens arm:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::Formatting>(
                crate::adapter::formatting_response::<THostLanguage>,
            )?
            .on_request_sync::<lsp_types::request::WillSaveWaitUntil>(
                crate::adapter::will_save_wait_until_response::<THostLanguage>,
            )?
```

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn formatting_response<THostLanguage: isograph_compiler::HostLanguage>(
    _state: &isograph_compiler::IsographState<THostLanguage>,
    _params: lsp_types::DocumentFormattingParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::Formatting as lsp_types::request::Request>::Result,
> {
    Vec::new().wrap_some().wrap_ok()
}

pub(crate) fn will_save_wait_until_response<THostLanguage: isograph_compiler::HostLanguage>(
    _state: &isograph_compiler::IsographState<THostLanguage>,
    _params: lsp_types::WillSaveTextDocumentParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::WillSaveWaitUntil as lsp_types::request::Request>::Result,
> {
    Vec::new().wrap_some().wrap_ok()
}
```

If lsp-open-files.md change 3 has already added `LspClientId` to request handlers, these functions take that argument.

### `initialize`

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

On `TextDocumentSyncOptions` (create that options object if it is still `Default`):

```rust
                    will_save_wait_until: true.wrap_some(),
```

If lsp-open-files.md already has a `TextDocumentSyncOptions` with `open_close` and `change`, add `will_save_wait_until` on that same struct.

`will_save_wait_until: true` and `OneOf::Left(true)` are `lsp_types` protocol fields.

## Tests

`lsp_socket.rs` (listen_and_reply, intern config as today):

- initialize result has `documentFormattingProvider == true` and `textDocumentSync.willSaveWaitUntil == true`
- `textDocument/formatting` for a URI that has a `DiskFile` with `iso(\`field Query.x{id}\`)`: `result` is JSON `[]`
- `textDocument/willSaveWaitUntil` for that URI: `result` is JSON `[]`
- `textDocument/formatting` for a URI that was never interned: `result` is JSON `[]`
- semantic tokens for the interned file still encode `field` as keyword
- unknown request is still `MethodNotFound`

## Call sites

- `Lsp::Request` `textDocument/formatting` -> `formatting_response` -> `SendLspResponse` with `[]`
- `Lsp::Request` `textDocument/willSaveWaitUntil` -> `will_save_wait_until_response` -> `SendLspResponse` with `[]`
- vscode-languageclient sends `willSaveWaitUntil` on save
- iso-format.md replaces the two handler bodies

## Docs this change amends

vscode-extension.md: the server advertises formatting; vscode-languageclient sends `willSaveWaitUntil` from that.

zed-and-vscode-extensions.md: the language server answers format requests.
