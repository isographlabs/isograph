# `textDocument/semanticTokens/full`

Requires lsp-request-response.md and lsp-dispatch.md. Independent of filesystem-watcher.md. Independent of lsp-sessions.md.

`dispatch_lsp` request arm is `method_not_found`. This file puts `LSPRequestDispatch` around it: `.on_request_sync::<SemanticTokensFullRequest>(semantic_tokens_response)?` then leftover `method_not_found`. Do not special-case tokens in `run_session`. `isograph/event` stays a notification arm.

Origin of the method: `lsp_types::request::SemanticTokensFullRequest`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of initialize options: isograph `server.rs` `initialize`. Origin of the dispatcher: isograph `LSPRequestDispatch`. Delta: URI to path has no `expect`; missing `DiskFile` is `Ok(None)` (JSON `null`).

One shippable change. An e2e that notifies DiskChanged and immediately asks for tokens can race; that is later.

## What the user does

The daemon is up.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Send notifies `isograph/event` and exits. Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts`. A URI with no `DiskFile` returns JSON `null`.

## Types

```rust
// from crates/isograph_cli/src/state.rs
        lsp_server::Message::Request(request) => dispatch_lsp_request(state, lsp.reply, request),

fn dispatch_lsp_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    reply: crossbeam::channel::Sender<lsp_server::Message>,
    request: lsp_server::Request,
) -> Vec<IsographEffect> {
    let get_response = || {
        let request = isograph_lsp::lsp_request_dispatch::LSPRequestDispatch::new(request, state)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                semantic_tokens_response::<THostLanguage>,
            )?
            .request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => crate::effect::IsographEffect::SendLspResponse(
            crate::effect::SendLspResponse { reply, response }.boxed(),
        )
        .wrap_vec(),
        ControlFlow::Continue(request) => method_not_found(reply, request),
    }
}
```

`semantic_tokens_response` takes `&IsographState`, `SemanticTokensParams`, returns `isograph_lsp::lsp_runtime_error::LSPRuntimeResult<<lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result>`. Missing file is `Ok(None)` (JSON `null`). Non-file URI is `Err(LSPRuntimeError::UnexpectedError(...))`.

```rust
// from crates/isograph_cli/src/adapter.rs
fn semantic_tokens_response<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> isograph_lsp::lsp_runtime_error::LSPRuntimeResult<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
> {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return isograph_lsp::lsp_runtime_error::LSPRuntimeError::UnexpectedError(
            "textDocument.uri is not a file path".to_owned(),
        )
        .wrap_err();
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

```toml
# from crates/isograph_cli/Cargo.toml
isograph_lsp = { path = "../isograph_lsp" }
url = { workspace = true }
```

`lib.rs`: `mod adapter;`

## Tests

`lsp_socket.rs` (multi-thread, settle as today):

- initialize legend `tokenTypes[15]` is `keyword`
- `notify` DiskChanged of `/tmp/proj/src/Home.ts` with the one-literal contents, settle, then `semanticTokens/full` for `file:///tmp/proj/src/Home.ts` on a second connection that has initialized: first token type 15, length 10
- `full` for a URI that was never interned: `result` is JSON `null`
- `full` with a non-file URI: `UnknownErrorCode`, message `textDocument.uri is not a file path`
- unknown request is still `MethodNotFound`

`state.rs` is unchanged. No `handle` of a tokens event.

## Call sites

- `handle` `Lsp` request -> `on_request_sync::<SemanticTokensFullRequest>` -> `semantic_tokens_response` -> `lsp_semantic_tokens_for_file` -> `SendLspResponse`
- other requests -> Continue -> `method_not_found`
