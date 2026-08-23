# `textDocument/semanticTokens/full`

Requires lsp-port.md (landed). Independent of filesystem-watcher.md. Independent of lsp-sessions.md: the session thread already writes responses on `connection.sender`. lsp-sessions.md is for server-to-client notifications to N clients, not this request.

`run_session` answers every request with `MethodNotFound`. This file adds `textDocument/semanticTokens/full`. `isograph/event` stays a notification deserialized in the session. `handle` does not grow an LSP arm. Event-model: an LSP request is request/response in the adapter; `handle` is not request/response.

`IsographState` is owned by `run_event_loop`. The session thread does not hold it. The adapter sends a query on a channel that `run_event_loop` recvs next to `IsographEvent`. The query is not an `IsographEvent`. The reply is a `lsp_server::Response` on a `std::sync::mpsc::sync_channel(0)`. The session blocks on that recv, then `connection.sender.send`. That is the outer adapter, not `handle`.

Origin of dispatch: isograph `lsp_request_dispatch.rs` / `server.rs` `dispatch_request`. Origin of the method: `lsp_types::request::SemanticTokensFullRequest`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of initialize options: isograph `server.rs` `initialize`. Delta: extract is `Result`; URI to path has no `expect`; missing `DiskFile` is `Ok(None)`; query is outer, not `handle`.

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
// from crates/isograph_cli/src/adapter.rs
pub(crate) enum AdapterQuery {
    SemanticTokens(SemanticTokensQuery),
}

pub(crate) struct SemanticTokensQuery {
    pub uri: lsp_types::Uri,
    pub id: lsp_server::RequestId,
    pub reply: std::sync::mpsc::SyncSender<lsp_server::Response>,
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    mut query_rx: UnboundedReceiver<AdapterQuery>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    loop {
        tokio::select! {
            event = event_rx.recv() => {
                let Some(event) = event else { break; };
                for effect in handle(&mut state, event) {
                    let _ = effect_tx.send(effect);
                }
            }
            query = query_rx.recv() => {
                let Some(query) = query else { break; };
                answer_query(&state, query);
            }
        }
    }
}

fn answer_query<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    query: AdapterQuery,
) {
    match query {
        AdapterQuery::SemanticTokens(SemanticTokensQuery { uri, id, reply }) => {
            let response = semantic_tokens_response(state, uri, id);
            let _ = reply.send(response);
        }
    }
}
```

`serve` creates the query channel. `accept_loop` / `session` / `run_session` take `query_tx: UnboundedSender<AdapterQuery>`. `_hold_events` still holds `event_tx`. Clone `query_tx` into each session. Dropping `query_tx` on `Kill` is fine: `process::exit(0)` follows.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request)
                if request.method == lsp_types::request::SemanticTokensFullRequest::METHOD =>
            {
                let id = request.id.clone();
                match serde_json::from_value::<lsp_types::SemanticTokensParams>(request.params) {
                    Ok(params) => {
                        let (reply, rx) = std::sync::mpsc::sync_channel(0);
                        let _ = query_tx.send(AdapterQuery::SemanticTokens(SemanticTokensQuery {
                            uri: params.text_document.uri,
                            id,
                            reply,
                        }));
                        if let Ok(response) = rx.recv() {
                            let _ = connection.sender.send(lsp_server::Message::Response(response));
                        }
                    }
                    Err(e) => {
                        warn!(error = %e, "semanticTokens params");
                        let _ = connection.sender.send(lsp_server::Message::Response(
                            lsp_server::Response::new_err(
                                id,
                                lsp_server::ErrorCode::InvalidParams as i32,
                                "invalid request params".to_owned(),
                            ),
                        ));
                    }
                }
            }
            lsp_server::Message::Request(request) => {
                // MethodNotFound as today
            }
```

`bounded(0)` / `sync_channel(0)`: session send of the query does not block on the event loop forever if the loop is in `handle`; the unbounded query channel does not block. `rx.recv` waits for `answer_query`. Fine: the session is a std thread.

Do not copy isograph `LSPRequestDispatch` until a second domain request exists. One method is a `match`.

```rust
// from crates/isograph_cli/src/adapter.rs
fn semantic_tokens_response<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    uri: lsp_types::Uri,
    id: lsp_server::RequestId,
) -> lsp_server::Response {
    let Some(absolute) = file_path(uri.reference()) else {
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

`file_path` is a function, not a trait. Missing cwd or `DiskFile`: JSON `null` (`Option::None` serializes). Present file with no iso: `Some` empty `data`. This path does not call `handle`.

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

- `run_session` Request `semanticTokens/full` -> `query_tx` -> `answer_query` -> `lsp_semantic_tokens_for_file` -> `connection.sender`
- other requests -> `MethodNotFound` on the session, as today
