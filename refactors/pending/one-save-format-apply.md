# One format apply on save

Independent of format-file-from-segments.md. Independent of lsp-open-files.md.

Save currently runs two `waitUntil` participants that return the same `TextEdit[]`. The second apply uses the original ranges on the already-formatted document and deletes the closing `` `); ``.

`adapter::tests::applying_the_same_edits_twice_with_stale_ranges_drops_the_closing_template` is that second apply.

## What the user does

Save a `.ts` file whose interned text matches the buffer. Iso interiors format once. The closing `` `); `` stays. `isograph logs --level debug` has one of `textDocument/formatting` or `textDocument/willSaveWaitUntil`, not both.

`isograph.autoformatIsoLiterals` false: save does not format.

Format Document With Isograph Language Client still sends `textDocument/formatting`.

## The two participants

The server advertises `willSaveWaitUntil`. vscode-languageclient registers `WillSaveWaitUntilFeature`, which on save sends `textDocument/willSaveWaitUntil` and returns those edits from `event.waitUntil`.

The extension also registers `workspace.onWillSaveTextDocument`, sends `textDocument/formatting`, and returns those edits from `event.waitUntil`.

VS Code concatenates both lists and applies them. The ranges were computed on the pre-save text. After the first list is applied, those ranges no longer match. The original interior is longer than the pretty interior (a blank line is gone), so the stale end sits past the iso interior and the second apply eats `` `); ``.

isograph had only the hook. It did not advertise `willSaveWaitUntil`. i2 advertises it and kept the hook.

## Change: delete the hook. `willSaveWaitUntil` is the save path.

No new types. No new functions.

```ts
// from vscode-extension/src/languageClient.ts
export async function createAndStartLanguageClient(
  context: IsographExtensionContext,
): Promise<void> {
```

After: no `workspace.onWillSaveTextDocument`. `clientOptions.middleware` is set only when autoformat is off, so the advertised `willSaveWaitUntil` does not still format.

```ts
// from vscode-extension/src/languageClient.ts
  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: 'file', language: 'javascript' },
      { scheme: 'file', language: 'typescript' },
      { scheme: 'file', language: 'typescriptreact' },
      { scheme: 'file', language: 'javascriptreact' },
    ],
    outputChannel: context.lspOutputChannel,
    revealOutputChannelOn: RevealOutputChannelOn.Never,
    middleware: config.autoformatIsoLiterals
      ? undefined
      : {
          willSaveWaitUntil: () => Promise.resolve([]),
        },
  };
```

Delete from this file: the `if (config.autoformatIsoLiterals) { workspace.onWillSaveTextDocument(...) }` block after `client.start()`. Delete imports used only by that block: `FormattingOptions`, `workspace`, `TextDocumentIdentifier`. Keep `TextEdit` only if the empty-array middleware needs it; `Promise.resolve([])` does not.

`await client.start()` stays. `context.client = client` stays.

Server capabilities stay:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
        text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                will_save_wait_until: true.wrap_some(),
                ..Default::default()
            },
        )
        .wrap_some(),
```

`initialize_advertises_formatting_and_will_save_wait_until` stays. `will_save_wait_until_matches_formatting` stays.

## Tests

`adapter::tests::applying_the_same_edits_twice_with_stale_ranges_drops_the_closing_template` stays. It documents the apply helper, not save. Do not add an extension test.

No new Rust tests. The save path is one `waitUntil` from vscode-languageclient. That is not a Rust unit.

## Who calls what

vscode-languageclient `WillSaveWaitUntilFeature.callback` still calls `client.sendRequest(WillSaveTextDocumentWaitUntilRequest)`. `will_save_wait_until_response` still calls `format_iso_literals`. The extension no longer calls `textDocument/formatting` on save. Format Document still does.
