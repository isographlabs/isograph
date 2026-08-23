# LSP format requests, no-op

Requires lsp-tokens.md (landed). Independent of iso-format.md, lsp-open-files.md, lsp-diagnostics.md.

The server does not advertise formatting. vscode-extension.md dropped isograph's will-save hook and `autoformatIsoLiterals`. A client that sends `textDocument/formatting` or `textDocument/willSaveWaitUntil` gets `MethodNotFound`.

This slice advertises those methods and answers them with an empty edit list. Save and Format Document do not change the buffer. iso-format.md fills the same handlers with real interiors.

Origin of the methods: `lsp_types::request::Formatting` and `lsp_types::request::WillSaveWaitUntil`. Origin of the client hook we are not restoring: isograph `vscode-extension/src/languageClient.ts` `onWillSaveTextDocument` gated on `autoformatIsoLiterals`. Delta: the server advertises `documentFormattingProvider` and `willSaveWaitUntil`; the handlers return `Some([])`; the setting stays gone; the extension does not send formatting itself. vscode-languageclient already sends `willSaveWaitUntil` when the server advertises it.

One shippable change.

## What the user does

Same as after lsp-tokens.md. Save does not rewrite iso interiors. Format Document, if it reaches this server, applies no edits. Initialize lists formatting and `willSaveWaitUntil`. A `textDocument/formatting` request is a response with `result: []`, not `MethodNotFound`.

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

Both handlers ignore params and state. Empty edits, not `null`. `null` is the missing-file answer in iso-format.md.

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

If lsp-open-files.md change 3 has already added `LspClientId` to request handlers, these functions take that argument and do not read it. Do not add `LspClientId` here if dispatch still matches `fn(&TState, Params)`.

### `initialize`

Before: `ServerCapabilities` with only `semantic_tokens_provider` (and, if lsp-open-files.md change 1 has landed, `text_document_sync` for full document sync). After, also:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
```

and on `TextDocumentSyncOptions` (create that options object if it is still `Default`):

```rust
                    will_save_wait_until: true.wrap_some(),
```

If lsp-open-files.md already has a `TextDocumentSyncOptions` with `open_close` and `change`, add `will_save_wait_until` on that same struct. Do not invent a second `text_document_sync`.

`will_save_wait_until: true` and `OneOf::Left(true)` are `lsp_types` protocol fields.

Do not restore `autoformatIsoLiterals`. Do not add an `onWillSaveTextDocument` handler in the VS Code extension.

## Tests

`lsp_socket.rs` (listen_and_reply, intern config as today):

- initialize result has `documentFormattingProvider == true` and `textDocumentSync.willSaveWaitUntil == true`
- `textDocument/formatting` for a URI that has a `DiskFile` with `iso(\`field Query.x{id}\`)`: `result` is JSON `[]`
- `textDocument/willSaveWaitUntil` for that URI: `result` is JSON `[]`
- `textDocument/formatting` for a URI that was never interned: `result` is JSON `[]`
- semantic tokens for the interned file still encode `field` as keyword
- unknown request is still `MethodNotFound`

No production function only tests call.

## Call sites

- `Lsp::Request` `textDocument/formatting` -> `formatting_response` -> `SendLspResponse` with `[]`
- `Lsp::Request` `textDocument/willSaveWaitUntil` -> `will_save_wait_until_response` -> `SendLspResponse` with `[]`
- vscode-languageclient, once initialize advertises `willSaveWaitUntil`, sends that request on save and applies `[]`
- iso-format.md replaces the two handler bodies

## Docs this change amends

vscode-extension.md: the setting stays gone. The server now advertises formatting; the client still does not send it itself.

zed-and-vscode-extensions.md: the language server answers format requests with no edits until iso-format.md.
