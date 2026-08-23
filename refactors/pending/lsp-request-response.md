# LSP request/response as event and effect

Requires lsp-port.md (landed). Independent of lsp-dispatch.md, lsp-tokens.md, lsp-sessions.md, lsp-outstanding.md, filesystem-watcher.md.

Today `run_session` writes `MethodNotFound` on `connection.sender`. Domain requests need `IsographState`, which the session thread does not own. This slice posts the request as an event and writes the reply as an effect. `handle` of that event is still `MethodNotFound`. That is a reader: every unknown request goes through it.

The session does not block. It posts the event and keeps pumping. The effect carries a clone of `connection.sender`. The effect loop writes `Message::Response`. No writer map. No client id. No outstanding set. Those are lsp-outstanding.md.

Origin of MethodNotFound text: isograph `server.rs` unhandled arm. Origin of `handle` / effects: landed event loop. Delta: `IsographEvent::LspRequest`, `IsographEffect::SendLspResponse`.

One shippable change. Existing CLI send tests stay green. Existing `lsp_socket` MethodNotFound tests stay green (the bytes do not change).

`docs-website/docs/design-docs/event-model.md` Inner: `handle` matches `LspRequest` and returns `SendLspResponse`. It still does not touch the socket. The effect loop writes.

## What the user does

Same as today. `isograph send` still initialize + `isograph/event`. An unknown request after initialize is still `MethodNotFound`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
pub(crate) struct LspRequest {
    pub request: lsp_server::Request,
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, derive_more::From)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
    #[serde(skip)]
    #[from]
    LspRequest(LspRequest),
}
```

`--file` JSON is unchanged. `LspRequest` is not a wire kind. No `PartialEq` / `Eq` on `IsographEvent`. `Sender` does not implement them. Do not add a tests-only impl. Tests use `matches!` or compare `DiskChanged` fields.

`reply` is `connection.sender.clone()`. Request id is `request.id`. Do not duplicate it as a field.

`request.params` is `serde_json::Value`. The event does not have a per-method typed enum. Dispatch (`extract::<TRequest::Params>`) is what types them.

```rust
// from crates/isograph_cli/src/effect.rs
pub(crate) struct SendLspResponse {
    pub reply: crossbeam::channel::Sender<lsp_server::Message>,
    pub response: lsp_server::Response,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(SendLspResponse),
}
```

No `PartialEq` / `Eq`. Nothing in production compares effects. `Sender` does not implement them. Do not add a tests-only impl. Existing `assert_eq` on `LogHelloWorld` / `Kill` become `matches!`. Empty effects: `effects.is_empty()`.

### `handle`

```rust
// from crates/isograph_cli/src/state.rs
        IsographEvent::LspRequest(request) => method_not_found(request).wrap_vec(),

fn method_not_found(request: crate::event::LspRequest) -> IsographEffect {
    let id = request.request.id.clone();
    IsographEffect::SendLspResponse(crate::effect::SendLspResponse {
        reply: request.reply,
        response: lsp_server::Response {
            id,
            result: None,
            error: lsp_server::ResponseError {
                code: lsp_server::ErrorCode::MethodNotFound as i32,
                data: None,
                message: format!(
                    "No handler registered for method '{}'",
                    request.request.method
                ),
            }
            .wrap_some(),
        },
    })
}
```

Dispatch (`on_request_sync`) is lsp-dispatch.md. This slice is one function.

`run_event_loop` is unchanged except it already calls `handle` and sends every effect. `SendLspResponse` goes through that same path.

### `perform`

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::SendLspResponse(respond) => {
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
```

`bounded(0)` on the copied writer: this send waits until the IO thread takes the message. Same as today's session-thread send. The effect loop is a current-thread task; it yields only at `.await`. This send is blocking. Acceptable this slice: MethodNotFound is small.

### Session

```rust
// from crates/isograph_cli/src/lsp_socket.rs
            lsp_server::Message::Request(request) => {
                let _ = event_tx.send(
                    crate::event::LspRequest {
                        request,
                        reply: connection.sender.clone(),
                    }
                    .to(),
                );
            }
```

The rest of `run_session` is unchanged (`initialize`, `isograph/event`, ignore other notifications). No `LspClientId`. No `LspClientGone`.

```toml
# from crates/isograph_cli/Cargo.toml
derive_more = { workspace = true }
```

`listen_for_events` in tests must still use a multi-thread runtime: `TcpStream` is blocking.

## Tests

`lsp_socket.rs`: existing MethodNotFound / shutdown / ServerNotInitialized / HelloWorld tests stay. `ServerNotInitialized` is still `Connection::initialize`.

Add: initialize, hover, `MethodNotFound`, then `isograph/event` HelloWorld on the same connection arrives.

Add: `handle` of `LspRequest`. `reply` is a `crossbeam` channel the test owns. One effect. Match `SendLspResponse`. Compare `response.id` to the request id, `result` is `None`, `error.code` is `MethodNotFound`, `error.message` is `No handler registered for method 'textDocument/hover'`. `perform` that effect; `reply`'s receiver gets `Message::Response` with that same id, code, and message.

`state.rs` existing in-process tests stay. They construct `HelloWorld` / `Quit` / `DiskChanged` only.

`daemon.rs` existing tests stay. They do not send `LspRequest`.

`cli.rs` unchanged.

## Call sites

- `run_session` Request -> `IsographEvent::LspRequest` -> `handle` -> `SendLspResponse` -> `perform` -> `reply.send`
- `run_session` `isograph/event` -> `event_tx` -> `handle` (unchanged)
- lsp-dispatch.md -> replaces `method_not_found` with `LSPRequestDispatch`
- lsp-outstanding.md -> client id, `LspClientGone`, outstanding set
