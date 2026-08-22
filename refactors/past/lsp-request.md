# Answering LSP requests from the worker

Requires lsp-socket.md. `lsp_semantic_tokens_for_file` is already in `isograph_lsp`. Independent of filesystem-watcher.md.

lsp-socket.md's session writes `Response` for `Initialize` and `Shutdown`. Domain requests are `MethodNotFound`. The session cannot read `IsographState`. `handle` stays ingest-only.

A domain request is forwarded as `Work::Request` plus a oneshot. The worker runs isograph's request dispatch against `&state` and `send`s the `Response`. The session thread `blocking_recv`s and writes it.

The first handler is `on_request_sync::<SemanticTokensFullRequest>(on_semantic_token_full_request)`, same call as isograph `server.rs` `dispatch_request`. Do not add a `Query` enum, a second port, or `isograph semantic-tokens`.

Origin of the channel: event-loop.md `IsographEvent`. Origin of dispatch: isograph `crates/isograph_lsp/src/lsp_request_dispatch.rs` and `server.rs` `dispatch_request`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Delta: the dispatcher runs on the worker, not on `LspState`; extract is `Result`, no `expect` / `catch_unwind`; URI to path has no `expect`; postfix constructors.

One shippable change.

## What the user does

The daemon is up. Send interned a file. An LSP client that has `initialize`d asks for tokens.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts` returns `data` whose first `tokenType` is 15 (keyword, `entrypoint`). A URI with no `DiskFile` returns JSON `null`.

`isograph send` is unchanged. `Initialize` / `Shutdown` / `Exit` stay on the session, as `Connection::initialize` and shutdown are outside `dispatch_request` in isograph.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/daemon.rs
enum Work {
    Event(IsographEvent),
    Request(lsp_server::Request, tokio::sync::oneshot::Sender<lsp_server::Response>),
}
```

`Work` is in-process. No `Serialize`. The session does not interpret domain methods. The worker does, through dispatch.

```rust
// from crates/isograph_cli/src/lsp_dispatch.rs
use std::ops::ControlFlow;

use lsp_server::ExtractError;
use lsp_types::request::Request;
use prelude::Postfix;
use tracing::warn;

pub struct LspRequestDispatch<'state, TState> {
    request: lsp_server::Request,
    state: &'state TState,
}

impl<'state, TState> LspRequestDispatch<'state, TState> {
    pub fn new(request: lsp_server::Request, state: &'state TState) -> Self {
        Self { request, state }
    }

    pub fn on_request_sync<TRequest: Request>(
        self,
        handler: fn(&TState, TRequest::Params) -> Result<TRequest::Result, lsp_server::ResponseError>,
    ) -> ControlFlow<lsp_server::Response, Self> {
        if self.request.method != TRequest::METHOD {
            return ControlFlow::Continue(self);
        }
        let id = self.request.id.clone();
        match self.request.extract(TRequest::METHOD) {
            Ok(params) => {
                let response = match handler(self.state, params) {
                    Ok(result) => match serde_json::to_value(result) {
                        Ok(result) => lsp_server::Response {
                            id,
                            result: result.wrap_some(),
                            error: None,
                        },
                        Err(e) => {
                            warn!(error = %e, "could not encode request result");
                            lsp_server::Response::new_err(
                                id,
                                lsp_server::ErrorCode::InternalError as i32,
                                "could not encode request result".to_owned(),
                            )
                        }
                    },
                    Err(error) => lsp_server::Response {
                        id,
                        result: None,
                        error: error.wrap_some(),
                    },
                };
                ControlFlow::Break(response)
            }
            Err(ExtractError::MethodMismatch(request)) => ControlFlow::Continue(Self {
                request,
                state: self.state,
            }),
            Err(ExtractError::JsonError { method, error }) => {
                warn!(method = method.as_str(), error = %error, "request params");
                ControlFlow::Break(lsp_server::Response::new_err(
                    id,
                    lsp_server::ErrorCode::InvalidParams as i32,
                    "invalid request params".to_owned(),
                ))
            }
        }
    }

    pub fn request(self) -> lsp_server::Request {
        self.request
    }
}
```

Origin: isograph `lsp_request_dispatch.rs` `LSPRequestDispatch::on_request_sync` and `convert_to_lsp_response`. Delta: no `LSPRuntimeError`, no `catch_unwind`, no `expect`, no `"default-lsp-id"`; `JsonError` is `InvalidParams` with the real `id`; postfix.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn dispatch_request<THostLanguage: isograph_compiler::HostLanguage>(
    request: lsp_server::Request,
    state: &isograph_compiler::IsographState<THostLanguage>,
) -> lsp_server::Response {
    let get_response = || {
        let request = crate::lsp_dispatch::LspRequestDispatch::new(request, state)
            .on_request_sync::<lsp_types::request::SemanticTokensFullRequest>(
                on_semantic_token_full_request::<THostLanguage>,
            )?
            .request();
        ControlFlow::Continue(request)
    };
    match get_response() {
        ControlFlow::Break(response) => response,
        ControlFlow::Continue(request) => lsp_server::Response::new_err(
            request.id,
            lsp_server::ErrorCode::MethodNotFound as i32,
            format!("No handler registered for method '{}'", request.method),
        ),
    }
}

fn on_semantic_token_full_request<THostLanguage: isograph_compiler::HostLanguage>(
    state: &isograph_compiler::IsographState<THostLanguage>,
    params: lsp_types::SemanticTokensParams,
) -> Result<
    <lsp_types::request::SemanticTokensFullRequest as lsp_types::request::Request>::Result,
    lsp_server::ResponseError,
> {
    let Some(absolute) = file_path(params.text_document.uri.reference()) else {
        return lsp_server::ResponseError {
            code: lsp_server::ErrorCode::InvalidParams as i32,
            message: "textDocument.uri is not a file path".to_owned(),
            data: None,
        }
        .wrap_err();
    };
    let tokens = semantic_tokens(state, absolute.reference());
    tokens
        .map(|data| {
            lsp_types::SemanticTokens {
                result_id: None,
                data,
            }
            .to()
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

Origin: isograph `dispatch_request` and `on_semantic_token_full_request`. Delta: `TState` is `&IsographState`; no `uri_is_project_file`; no `.expect` on `to_file_path`; missing `DiskFile` is `Ok(None)` (JSON `null`); present file with no iso is `Ok(Some(empty data))` from `lsp_semantic_tokens_for_file`. `file_path` is a function, not a trait.

`on_semantic_token_full_request` is generic over `THostLanguage`. `on_request_sync` takes `fn(&TState, Params) -> ...`. `TState` is `IsographState<THostLanguage>`. The call site turbofishs the handler. `run_event_loop` is already generic.

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
            Work::Request(request, reply) => {
                let _ = reply.send(crate::lsp_socket::dispatch_request(request, &state));
            }
        }
    }
}
```

Existing `run_event_loop` tests send `Work::Event(...)`. Add:

- intern config directory and `DiskChanged` `Present` of `/tmp/proj/src/Home.ts` with `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. `Work::Request` a `SemanticTokensFullRequest` for `file:///tmp/proj/src/Home.ts`. The oneshot `Response` has `result.data` first `token_type` 15, `length` 10. No effects from the request.
- same, URI with no `DiskFile`: `result` is JSON `null`. No effects.

### Session

Origin: lsp-socket.md `Running` + `Request` → `MethodNotFound`. Delta: not `Shutdown` → `Work::Request`, wait, write that `Response`. `step` takes `&UnboundedSender<Work>`. Notifications still `Work::Event` via `dispatch_notification`. One in-flight request per connection.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
        (Session::Running, lsp_server::Message::Request(request))
            if request.method == lsp_types::request::Shutdown::METHOD =>
        { /* as lsp-socket.md */ }
        (Session::Running, lsp_server::Message::Request(request)) => {
            let (reply, rx) = tokio::sync::oneshot::channel();
            if work_tx
                .send(crate::daemon::Work::Request(request, reply))
                .is_err()
            {
                return Step::End;
            }
            match rx.blocking_recv() {
                Ok(response) => Step::Reply(response),
                Err(_) => Step::End,
            }
        }
```

`accept_loop` / `session` take `UnboundedSender<Work>`. `dispatch_notification` handlers `send(Work::Event(...))`. `serve` uses that channel. SIGTERM sends `Work::Event(Quit)`.

### `initialize` legend

Origin: lsp-socket.md empty `ServerCapabilities`. Origin of the options: isograph `server.rs` `initialize`. Delta: only semantic tokens, no hover or completion.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
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
```

### Cargo

```toml
# from crates/isograph_cli/Cargo.toml
isograph_lsp = { path = "../isograph_lsp" }
url = { workspace = true }
```

## Tests

`lsp_dispatch.rs`: origin isograph `calls_first_matching_request_handler`. Hover then GotoDefinition; only the matching method runs. `expect` names the fixture.

`lsp_socket.rs`:

- initialize result legend `tokenTypes[15]` is `keyword`
- initialize, `isograph/diskChanged` of `/tmp/proj/src/Home.ts` with the one-literal contents, then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts`: `result.data` first token type 15, length 10
- `full` for a URI that was never interned: `result` is JSON `null`
- `full` with a non-file URI: `InvalidParams`

## Call sites

- `serve` -> `Work` channel -> `accept_loop` / SIGTERM / `run_event_loop`
- `Running` + domain request -> `Work::Request` -> `dispatch_request` -> `on_semantic_token_full_request` -> `lsp_semantic_tokens_for_file` -> oneshot -> `Message::Response`
- `Running` + notification -> `dispatch_notification` -> `Work::Event` -> `handle`
