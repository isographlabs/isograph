# `textDocument/semanticTokens/full`

Requires lsp-port.md. `lsp_semantic_tokens_for_file` is already in `isograph_lsp`. Independent of filesystem-watcher.md.

Ingest methods are still a map onto `IsographEvent`. `semanticTokens/full` is not `handle`. This is the first method that justifies isograph’s `on_request_sync` chain. An e2e client does `isograph/diskChanged` (request, waits) then `textDocument/semanticTokens/full`. Because ingest was a request, the tokens cannot be from before the file was interned.

Origin of dispatch: isograph `lsp_request_dispatch.rs` / `server.rs` `dispatch_request`. Origin of the method: `lsp_types::request::SemanticTokensFullRequest`. Origin of the handler: isograph `on_semantic_token_full_request`. Origin of tokens: `lsp_semantic_tokens_for_file`. Origin of initialize options: isograph `server.rs` `initialize`. Delta: extract is `Result`; URI to path has no `expect`; missing `DiskFile` is `Ok(None)`; `&IsographState` not `LspState`.

One shippable change.

## What the user does

The daemon is up.

```
$ printf '%s\n' '{"kind":"DiskChanged","value":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Send exits 0 only after `handle` interned the file. Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts` returns `data` whose first `tokenType` is 15 (`entrypoint`). A URI with no `DiskFile` returns JSON `null`.

## Types

```rust
// from crates/isograph_cli/src/daemon.rs
enum Work {
    Event(IsographEvent),
    Ingest(IsographEvent, tokio::sync::oneshot::Sender<()>),
    LspRequest(lsp_server::Request, tokio::sync::oneshot::Sender<lsp_server::Response>),
}
```

`ingest_request`’s `None` arm (not an ingest method) becomes `Work::LspRequest` instead of `MethodNotFound`. The worker:

```rust
            Work::LspRequest(request, reply) => {
                let _ = reply.send(dispatch_request(request, &state));
            }
```

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

Origin: isograph `LSPRequestDispatch`. Delta: no `LSPRuntimeError`; extract is `match`; postfix. Handler is `fn(&TState, ...)`: tokens do not mutate.

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

`file_path` is a function, not a trait. Missing cwd or `DiskFile`: `Ok(None)`. Present file with no iso: `Ok(Some(empty data))`. This handler does not call `handle`.

### `initialize` legend

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

`lib.rs`: `mod lsp_dispatch;`

## Tests

`lsp_socket.rs`:

- initialize legend `tokenTypes[15]` is `keyword`
- `isograph/diskChanged` of `/tmp/proj/src/Home.ts` with the one-literal contents (wait for `null`), then `semanticTokens/full` for `file:///tmp/proj/src/Home.ts`: first token type 15, length 10
- `full` for a URI that was never interned: `result` is JSON `null`
- `full` with a non-file URI: `InvalidParams`

`run_event_loop`: intern via `Work::Ingest` of `DiskChanged`, then `Work::LspRequest` of `SemanticTokensFullRequest`. Same token assertions. No effects from the tokens request.

## Call sites

- ingest `None` -> `Work::LspRequest` -> `dispatch_request` -> `on_semantic_token_full_request` -> `lsp_semantic_tokens_for_file`
