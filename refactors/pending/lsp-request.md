# Answering LSP requests from the worker

Requires lsp-socket.md. `lsp_semantic_tokens_for_file` is already in `isograph_lsp`. Independent of filesystem-watcher.md.

The session can write an LSP `Response` (`Initialize`, `Shutdown`, `MethodNotFound`). It cannot read `IsographState`. `handle` stays ingest-only. A domain request is `Work::Query` plus a oneshot. The worker answers from `&state` and `send`s the result. The session thread `blocking_recv`s and writes the LSP response.

The first request is `textDocument/semanticTokens/full` (`lsp_types::request::SemanticTokensFullRequest`). Same typed extra-method style as `HelloWorld`: use the `lsp_types::request::Request` impl that already exists. Do not add a second port or `isograph semantic-tokens`.

Origin of `Work`: event-loop.md `IsographEvent` channel. Origin of oneshot query: e2e-semantic-tokens.md (not landed; that file's query port is not this). Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of the LSP method: `lsp_types` `SemanticTokensFullRequest`. Delta: in-process only; the session is the LSP client of the worker; `Query` is not serde on the socket.

One shippable change.

## What the user does

The daemon is up. Send interned a file. An LSP client that has `initialize`d asks for tokens.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts` returns `data` whose first `tokenType` is 15 (keyword, `entrypoint`). A URI with no `DiskFile` returns JSON `null` (`Option::None`).

`isograph send` is unchanged. `Initialize` / `Shutdown` / `Exit` stay on the session thread.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/daemon.rs
enum Work {
    Event(IsographEvent),
    Query(Query, tokio::sync::oneshot::Sender<QueryResult>),
}

enum Query {
    SemanticTokens(std::path::PathBuf),
}

enum QueryResult {
    SemanticTokens(Option<Vec<lsp_types::SemanticToken>>),
}
```

`Query` is in-process. No `Serialize`. The `PathBuf` is the `file://` URI made absolute. The worker turns it into `RelativePathToSourceFile` the same way `handle` turns `DiskChanged.path`.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop<THostLanguage: HostLanguage>(
    mut state: IsographState<THostLanguage>,
    mut work_rx: tokio::sync::mpsc::UnboundedReceiver<Work>,
    effect_tx: tokio::sync::mpsc::UnboundedSender<IsographEffect>,
) {
    while let Some(work) = work_rx.recv().await {
        match work {
            Work::Event(event) => {
                for effect in handle(&mut state, event) {
                    let _ = effect_tx.send(effect);
                }
            }
            Work::Query(query, reply) => {
                let _ = reply.send(answer(&state, query));
            }
        }
    }
}

fn answer<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    query: Query,
) -> QueryResult {
    match query {
        Query::SemanticTokens(absolute) => QueryResult::SemanticTokens(semantic_tokens(state, absolute.reference())),
    }
}

fn semantic_tokens<THostLanguage: HostLanguage>(
    state: &IsographState<THostLanguage>,
    absolute: &std::path::Path,
) -> Option<Vec<lsp_types::SemanticToken>> {
    let cwd = state.get_singleton::<CurrentWorkingDirectory>()?;
    let path = relative_path_from_absolute_and_working_directory(*cwd, &absolute.to_path_buf());
    isograph_lsp::lsp_semantic_tokens_for_file::<THostLanguage>(state, path)
}
```

`CurrentWorkingDirectory` missing: `None` (no tokens). No `expect`. `relative_path_to_source_file` in `state.rs` stays the `handle` helper. `answer` uses the same `relative_path_from_absolute_and_working_directory` it already imports.

`isograph_cli` depends on `isograph_lsp`.

Existing `run_event_loop` tests send `Work::Event(...)`. Add:

- intern config directory and a `DiskChanged` `Present` of `/tmp/proj/src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)` via `handle` / `Work::Event`. `Query::SemanticTokens` that path. `QueryResult::SemanticTokens` is `Some`. First `token_type` is 15, `length` is 10. No effects from the query.
- `Query::SemanticTokens` of a path that was never interned: `None`. No effects.

### Session

Origin: lsp-socket.md `step` `Running` + `Request` → `MethodNotFound`. Delta: `SemanticTokensFullRequest` becomes a query. `step` takes `&UnboundedSender<Work>`. One in-flight query per connection; the session thread waits.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn extract_request<R: lsp_types::request::Request>(
    request: &lsp_server::Request,
) -> Option<Result<R::Params, serde_json::Error>> {
    if request.method != R::METHOD {
        return None;
    }
    serde_json::from_value(request.params.clone()).wrap_some()
}

fn file_path(uri: &lsp_types::Uri) -> Option<std::path::PathBuf> {
    url::Url::parse(uri.as_str())
        .ok()?
        .to_file_path()
        .ok()
}

fn query_semantic_tokens(
    work_tx: &tokio::sync::mpsc::UnboundedSender<crate::daemon::Work>,
    id: lsp_server::RequestId,
    params: lsp_types::SemanticTokensParams,
) -> Step {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return Step::Reply(lsp_server::Response::new_err(
            id,
            lsp_server::ErrorCode::InvalidParams as i32,
            "textDocument.uri is not a file path".to_owned(),
        ));
    };
    let (reply, rx) = tokio::sync::oneshot::channel();
    if work_tx
        .send(crate::daemon::Work::Query(
            crate::daemon::Query::SemanticTokens(absolute),
            reply,
        ))
        .is_err()
    {
        return Step::End;
    }
    match rx.blocking_recv() {
        Ok(crate::daemon::QueryResult::SemanticTokens(tokens)) => {
            match semantic_tokens_response(id, tokens) {
                Ok(response) => Step::Reply(response),
                Err(e) => {
                    warn!(error = %e, "could not encode semantic tokens");
                    Step::Reply(lsp_server::Response::new_err(
                        id,
                        lsp_server::ErrorCode::InternalError as i32,
                        "could not encode semantic tokens".to_owned(),
                    ))
                }
            }
        }
        Err(_) => Step::End,
    }
}

fn semantic_tokens_response(
    id: lsp_server::RequestId,
    tokens: Option<Vec<lsp_types::SemanticToken>>,
) -> Result<lsp_server::Response, serde_json::Error> {
    let result: <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result =
        tokens.map(|data| {
            lsp_types::SemanticTokens {
                result_id: None,
                data,
            }
            .to()
        });
    serde_json::to_value(result).map(|result| lsp_server::Response {
        id,
        result: result.wrap_some(),
        error: None,
    })
}
```

`SemanticTokensFullRequest::Result` is `Option<SemanticTokensResult>`. No `DiskFile` is JSON `null`. A present file with no iso literals is `Some` of empty `data` (`lsp_semantic_tokens_for_file` already returns that). `Work` / `Query` / `QueryResult` are `pub(crate)` on `daemon`. No new trait.

`Running` + request in `step`:

```rust
        (Session::Running, lsp_server::Message::Request(request))
            if request_is::<lsp_types::request::Shutdown>(&request) =>
        { /* as lsp-socket.md */ }
        (Session::Running, lsp_server::Message::Request(request))
            if request_is::<lsp_types::request::SemanticTokensFullRequest>(&request) =>
        {
            let id = request.id.clone();
            match extract_request::<lsp_types::request::SemanticTokensFullRequest>(&request) {
                Some(Ok(params)) => query_semantic_tokens(work_tx, id, params),
                Some(Err(e)) => {
                    warn!(error = %e, "semantic tokens params");
                    Step::Reply(lsp_server::Response::new_err(
                        id,
                        lsp_server::ErrorCode::InvalidParams as i32,
                        "invalid semantic tokens params".to_owned(),
                    ))
                }
                None => Step::Reply(lsp_server::Response::new_err(
                    id,
                    lsp_server::ErrorCode::InternalError as i32,
                    "semantic tokens method mismatch".to_owned(),
                )),
            }
        }
        (Session::Running, lsp_server::Message::Request(request)) => { /* MethodNotFound as today */ }
```

`request_is` already matched the method, so `extract_request` `None` is a bug. `InvalidParams` on JSON failure. `Initialize` / `Shutdown` do not go through `Work`.

`accept_loop` / `session` take `UnboundedSender<Work>` instead of `UnboundedSender<IsographEvent>`. Notifications still `work_tx.send(Work::Event(event))`. `serve` passes that sender as today.

### `initialize` legend

Origin: lsp-socket.md empty `ServerCapabilities`. Delta: advertise full semantic tokens so a client will send the request.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
    serde_json::to_value(lsp_types::InitializeResult {
        capabilities: lsp_types::ServerCapabilities {
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
        },
        server_info: lsp_types::ServerInfo {
            name: "isograph".to_owned(),
            version: None,
        }
        .wrap_some(),
    })
```

Origin of the options struct: isograph `crates/isograph_lsp/src/server.rs` `initialize`. Delta: no hover, completion, or other providers.

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
isograph_lsp = { path = "../isograph_lsp" }
url = { workspace = true }
```

`url` is already a workspace dependency.

## Tests

`lsp_socket.rs` (on top of lsp-socket.md):

- initialize result `capabilities.semanticTokensProvider.legend.tokenTypes[15]` is `keyword`
- initialize, `isograph/diskChanged` of `/tmp/proj/src/Home.ts` with the one-literal contents, then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts`: `result.data` non-empty, first token type 15, length 10
- same handshake, `full` for a URI that was never interned: `result` is JSON `null`
- `full` with a non-file URI: `InvalidParams`, no query (event_rx / work side has no `Query`)

`cli.rs` is optional here. A protocol test in `lsp_socket.rs` is enough. e2e-semantic-tokens.md's hidden verb and query port are not this.

## Call sites

- `serve` -> `Work` channel -> `accept_loop` / SIGTERM (`Work::Event(Quit)`) / `run_event_loop`
- `Running` + `SemanticTokensFullRequest` -> `Work::Query` -> `answer` -> `lsp_semantic_tokens_for_file` -> oneshot -> `Message::Response`
- `Running` + notification -> `Work::Event` -> `handle` as lsp-socket.md
