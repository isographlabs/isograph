# `textDocument/semanticTokens/full`

Requires lsp-dispatch.md. Independent of filesystem-watcher.md. Independent of lsp-sessions.md.

`dispatch_lsp_request` is `LSPRequestDispatch` with zero `on_request_sync` calls. This file inserts `.on_request_sync::<SemanticTokensFullRequest>(semantic_tokens_response)?` before `.request()`. Do not special-case tokens in `run_session`. `isograph/event` stays a notification arm. `notify` takes `Internal`. `DiskChanged` is `File` / `FolderRemoved`.

Origin of the method: `lsp_types::request::SemanticTokensFullRequest`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of initialize options: isograph `server.rs` `initialize`. Origin of the dispatcher: landed `LSPRequestDispatch`. Delta: URI to path has no `expect`; missing `DiskFile` is `Ok(None)` (JSON `null`). Extract `JsonError` uses the request id, not `"default-lsp-id"`.

One shippable change. An e2e that uses `isograph send` then `semanticTokens/full` is e2e-send-semantic-tokens.md. Send does not wait for ingest; that e2e polls.

## What the user does

The daemon is up.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Send notifies `isograph/event` (`Internal`) and exits. Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts`. A URI with no `DiskFile` returns JSON `null`.

## Types

Insert before `.request()` in lsp-dispatch.md `dispatch_lsp_request`:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                semantic_tokens_response::<THostLanguage>,
            )?
```

`semantic_tokens_response` takes `&IsographState`, `SemanticTokensParams`, returns `isograph_lsp::lsp_runtime_error::LSPRuntimeResult<<lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result>`. Missing file is `Ok(None)` (JSON `null`). Non-file URI is `ExpectedError` (JSON `null`).

```rust
// from crates/isograph_cli/src/adapter.rs
fn semantic_tokens_response<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
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

fn semantic_tokens<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    absolute: &std::path::Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let cwd = state.get_singleton::<common_lang_types::CurrentWorkingDirectory>()?;
    let path = common_lang_types::relative_path_from_absolute_and_working_directory(
        *cwd,
        &absolute.to_path_buf(),
    );
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(state, path)
}
```

`file_path` is a function, not a trait. Missing cwd or `DiskFile`: JSON `null`. Present file with no iso: `Some` empty `data`.

### `initialize` legend

Replace `ServerCapabilities::default()` in `run_session` with:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        serde_json::to_value(lsp_types::ServerCapabilities {
            semantic_tokens_provider:
                lsp_types::SemanticTokensServerCapabilities::SemanticTokensOptions(
                    lsp_types::SemanticTokensOptions {
                        work_done_progress_options: lsp_types::WorkDoneProgressOptions::default(),
                        legend: isograph_lsp::semantic_token_legend(),
                        range: None,
                        full: lsp_types::SemanticTokensFullOptions::Bool(true).wrap_some(),
                    },
                )
                .wrap_some(),
            ..Default::default()
        })
```

### Cargo

lsp-dispatch.md already adds `isograph_lsp`. This slice:

```toml
# from crates/isograph_cli/Cargo.toml
url = { workspace = true }
```

`lib.rs`: `mod adapter;`

## Tests

`lsp_socket.rs` (multi-thread, settle as today). Use `listen_and_reply` so `handle` and `perform` run. Intern `/tmp/proj/isograph.config.json` on that loop's `IsographState` (`intern_config_directory`) so `DiskChanged` does not panic.

`notify` takes `Internal`. DiskChanged intern:

```
notify(
    connect(port),
    crate::event::Internal::DiskChanged(DiskChanged::File(DiskFileChanged {
        path: PathBuf::from("/tmp/proj/src/Home.ts"),
        presence: Presence::Present(
            "export const Home = iso(`entrypoint Query.HomeRoute`)".to_owned(),
        ),
    })),
)
```

- initialize legend `tokenTypes[15]` is `keyword`
- `notify` that `DiskChanged::File`, settle, then `semanticTokens/full` for `file:///tmp/proj/src/Home.ts` on a second connection that has initialized: first token type 15, length 10
- `full` for a URI that was never interned: `result` is JSON `null`
- `full` with a non-file URI: `result` is JSON `null`
- `full` with params `{}`: extract fails; response `id` is the request id, not `"default-lsp-id"`
- unknown request is still `MethodNotFound`

`state.rs` is otherwise unchanged. No `handle` of a tokens event.

## Call sites

- `Lsp::Request` -> `on_request_sync::<SemanticTokensFullRequest>` -> `semantic_tokens_response` -> `lsp_semantic_tokens_for_file` -> `SendLspResponse`
- other requests -> Continue -> `method_not_found`
