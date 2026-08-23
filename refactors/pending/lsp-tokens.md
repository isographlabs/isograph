# `textDocument/semanticTokens/full`

Requires lsp-request-response.md. Independent of filesystem-watcher.md. Independent of lsp-sessions.md.

`answer_request` is `MethodNotFound` for every method. This file adds `textDocument/semanticTokens/full`. The session already forwards every request; do not special-case tokens in `run_session`. `isograph/event` stays a notification. `handle` does not grow an LSP arm.

Origin of the method: `lsp_types::request::SemanticTokensFullRequest`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of initialize options: isograph `server.rs` `initialize`. Delta: extract is `Result`; URI to path has no `expect`; missing `DiskFile` is JSON `null`.

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
// from crates/isograph_cli/src/daemon.rs
fn answer_request<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    request: lsp_server::Request,
) -> lsp_server::Response {
    if request.method == lsp_types::request::SemanticTokensFullRequest::METHOD {
        return semantic_tokens_response(state, request);
    }
    lsp_server::Response {
        id: request.id,
        result: None,
        error: lsp_server::ResponseError {
            code: lsp_server::ErrorCode::MethodNotFound as i32,
            data: None,
            message: format!("No handler registered for method '{}'", request.method),
        }
        .wrap_some(),
    }
}
```

Do not copy isograph `LSPRequestDispatch` until a second domain request exists. One method is an `if`.

```rust
// from crates/isograph_cli/src/adapter.rs
fn semantic_tokens_response<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    request: lsp_server::Request,
) -> lsp_server::Response {
    let id = request.id.clone();
    let params = match serde_json::from_value::<lsp_types::SemanticTokensParams>(request.params) {
        Ok(params) => params,
        Err(e) => {
            warn!(error = %e, "semanticTokens params");
            return lsp_server::Response::new_err(
                id,
                lsp_server::ErrorCode::InvalidParams as i32,
                "invalid request params".to_owned(),
            );
        }
    };
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return lsp_server::Response::new_err(
            id,
            lsp_server::ErrorCode::InvalidParams as i32,
            "textDocument.uri is not a file path".to_owned(),
        );
    };
    let tokens = semantic_tokens(state, absolute.reference());
    let result = tokens.map(|data| lsp_types::SemanticTokens {
        result_id: None,
        data,
    });
    match serde_json::to_value(result) {
        Ok(result) => lsp_server::Response {
            id,
            result: result.wrap_some(),
            error: None,
        },
        Err(e) => {
            warn!(error = %e, "could not encode tokens");
            lsp_server::Response::new_err(
                id,
                lsp_server::ErrorCode::InternalError as i32,
                "could not encode request result".to_owned(),
            )
        }
    }
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

`file_path` is a function, not a trait. Missing cwd or `DiskFile`: JSON `null`. Present file with no iso: `Some` empty `data`. This path does not call `handle`.

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
- `full` with a non-file URI: `InvalidParams`
- unknown request is still `MethodNotFound`

`state.rs` is unchanged. No `handle` of a tokens event.

## Call sites

- `run_session` Request -> `LspRequest` (already) -> `answer_request` -> `semantic_tokens_response` -> `lsp_semantic_tokens_for_file`
- other requests -> `MethodNotFound` in `answer_request`, as after lsp-request-response.md
