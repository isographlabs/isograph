# E2E: `isograph send`, then `textDocument/semanticTokens/full`

Requires lsp-tokens.md. Independent of filesystem-watcher.md (the tests use `--filesystem injected`). Independent of lsp-sessions.md, lsp-proxy.md, lsp-diagnostics.md.

Origin of send: landed `cli.rs` / `isograph send`. Origin of tokens: lsp-tokens.md `semantic_tokens_response` / `lsp_semantic_tokens_for_file`. Origin of the socket helpers: `lsp_socket.rs` `connect` / `split` / `write_message` / `read_message` / `initialize`. Delta: `cli.rs` drives the built `isograph` binary. `Daemon::start` already passes `--filesystem injected`. Ingest is `isograph send` of `DiskChanged::File`. The query is a second TCP client of `{slug}.port` that sends `textDocument/semanticTokens/full`. No new verb. No production code.

`isograph send` writes `isograph/event` and exits. It does not wait for `handle`. A request on a second connection immediately after send can see JSON `null`. The tests poll `semanticTokens/full` until the intern is visible (10s deadline, same `poll` as today).

One shippable change.

## What the user does

The daemon is up with injected filesystem. There is no watcher. The `.ts` path is not created on disk.

```
$ isograph start --filesystem injected
$ printf '%s\n' '{"kind":"DiskChanged","value":{"File":{"path":"/tmp/proj/src/Home.ts","presence":{"Present":"export const Home = iso(`entrypoint Query.HomeRoute`)"}}}}' > /tmp/disk.json
$ isograph send --file /tmp/disk.json
```

Then `textDocument/semanticTokens/full` for `file:///tmp/proj/src/Home.ts`. First token is `entrypoint`: `token_type` 15, `length` 10, `delta_start` the UTF-16 length of `export const Home = iso(\``. A URI with no `DiskFile` returns JSON `null`. A present file with no iso returns `{ "data": [] }`.

## Types

Most important first.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
use std::io::{BufRead, BufReader, Write};
use std::net::{Ipv4Addr, TcpStream};
use std::path::PathBuf;
use std::process::{Command, Output};
use std::time::{Duration, Instant};

use lsp_server::{Message, Request, RequestId};
use lsp_types::notification::{Initialized, Notification};
use lsp_types::request::{Initialize, Request as LspRequest, SemanticTokensFullRequest};
use prelude::Postfix;
```

Existing `use` stays. `LspRequest` is in scope so `Initialize::METHOD` and `SemanticTokensFullRequest::METHOD` resolve.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
struct LspClient {
    writer: TcpStream,
    reader: BufReader<TcpStream>,
    next_id: i32,
}
```

`next_id` starts at 2. Initialize on that connection used 1.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
impl LspClient {
    fn connect(port: u16) -> Self {
        let stream =
            TcpStream::connect((Ipv4Addr::LOCALHOST, port)).expect("connecting to the daemon");
        let writer = stream.try_clone().expect("cloning the stream");
        let mut client = Self {
            writer,
            reader: BufReader::new(stream),
            next_id: 2,
        };
        client.initialize();
        client
    }

    fn initialize(&mut self) {
        let id = RequestId::from(1);
        write_message(
            &mut self.writer,
            Message::Request(Request {
                id: id.clone(),
                method: Initialize::METHOD.to_owned(),
                params: serde_json::json!({ "capabilities": {} }),
            }),
        );
        let response = read_response(&mut self.reader, id.reference());
        assert!(response.error.is_none(), "{response:?}");
        write_message(
            &mut self.writer,
            Message::Notification(lsp_server::Notification {
                method: Initialized::METHOD.to_owned(),
                params: serde_json::json!({}),
            }),
        );
    }

    fn semantic_tokens_full(&mut self, uri: &str) -> Option<Vec<lsp_types::SemanticToken>> {
        let id = RequestId::from(self.next_id);
        self.next_id += 1;
        write_message(
            &mut self.writer,
            Message::Request(Request {
                id: id.clone(),
                method: SemanticTokensFullRequest::METHOD.to_owned(),
                params: serde_json::json!({ "textDocument": { "uri": uri } }),
            }),
        );
        tokens_from_response(read_response(&mut self.reader, id.reference()))
    }
}
```

`initialize` is copied from `crates/isograph_cli/src/lsp_socket.rs` `initialize`. Delta: method on `LspClient`, `read_response` instead of `read_message` plus a `let else`.

`write_message` / `read_message` are copied from `lsp_socket.rs`. Delta: they live in `cli.rs`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn write_message(writer: &mut impl Write, message: Message) {
    message.write(writer).expect("writing an lsp message");
}

fn read_message(reader: &mut impl BufRead) -> Message {
    Message::read(reader)
        .expect("reading an lsp message")
        .expect("the connection stayed open")
}
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn read_response(reader: &mut impl BufRead, expected: &RequestId) -> lsp_server::Response {
    loop {
        let Message::Response(response) = read_message(reader) else {
            continue;
        };
        assert_eq!(&response.id, expected);
        return response;
    }
}

fn tokens_from_response(
    response: lsp_server::Response,
) -> Option<Vec<lsp_types::SemanticToken>> {
    assert!(response.error.is_none(), "{response:?}");
    let value = response.result?;
    if value.is_null() {
        return None;
    }
    match serde_json::from_value(value).expect("the result is SemanticTokensResult") {
        lsp_types::SemanticTokensResult::Tokens(tokens) => tokens.data.wrap_some(),
        lsp_types::SemanticTokensResult::Partial(_) => {
            panic!("the daemon does not send partial semantic tokens")
        }
    }
}
```

JSON `null` and an omitted `result` are `None` (no `DiskFile`). `{ "data": [] }` is `Some` empty (present file, no iso tokens).

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn wait_until_up(daemon: &Daemon) {
    poll(|| {
        daemon
            .log_text()
            .contains("isograph daemon up")
            .then_some(())
    });
}

fn source_path(daemon: &Daemon, relative: &str) -> std::path::PathBuf {
    daemon
        .dir
        .path()
        .canonicalize()
        .expect("the fixture exists")
        .join(relative)
}

fn file_uri(path: &std::path::Path) -> String {
    url::Url::from_file_path(path)
        .expect("the path is absolute")
        .to_string()
}

fn send_json(daemon: &Daemon, value: serde_json::Value) {
    let frame = write_frame(daemon.dir.path(), value.to_string().as_str());
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
}

fn present(path: &std::path::Path, contents: &str) -> serde_json::Value {
    serde_json::json!({
        "kind": "DiskChanged",
        "value": {
            "File": {
                "path": path,
                "presence": { "Present": contents }
            }
        }
    })
}

fn absent(path: &std::path::Path) -> serde_json::Value {
    serde_json::json!({
        "kind": "DiskChanged",
        "value": {
            "File": {
                "path": path,
                "presence": "Absent"
            }
        }
    })
}
```

`source_path` canonicalizes the temp dir, then joins. The `.ts` path is not created. `Daemon::start` already interned the canonical config directory; `handle` diffs `DiskChanged.path` against that.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
const CONTENTS: &str = "export const Home = iso(`entrypoint Query.HomeRoute`)";
const KEYWORD: u32 = 15;
```

Same fixture and `token_type` as `file_semantic_tokens.rs` `one_literal_encodes_entrypoint_as_keyword`.

### Cargo

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
[dev-dependencies]
lsp-server = { workspace = true }
lsp-types = { workspace = true }
prelude = { path = "../prelude" }
serde_json = { workspace = true }
tempfile = "3"
url = { workspace = true }
```

## Tests

`cli.rs`. `Daemon::start` (`--filesystem injected`). Do not call `start_watch`. Do not write the `.ts` path.

`wait_until_up`, then `isograph send` when the case has a file to intern, then `LspClient::connect(daemon_port)` and `file_uri` of the same absolute path as the frame. Keep that connection open. After send, `poll` `semantic_tokens_full` until the intern is visible.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
#[test]
fn send_of_a_present_iso_literal_returns_entrypoint_as_keyword() {
    let daemon = Daemon::start();
    wait_until_up(daemon.reference());
    let path = source_path(daemon.reference(), "src/Home.ts");
    assert!(!path.exists(), "injected send does not write the path");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let tokens = poll(|| client.semantic_tokens_full(uri.as_str()));
    assert!(!path.exists(), "injected send does not write the path");
    assert_eq!(tokens[0].delta_line, 0);
    assert_eq!(tokens[0].token_type, KEYWORD);
    assert_eq!(tokens[0].length, 10);
    assert_eq!(
        tokens[0].delta_start,
        "export const Home = iso(`".encode_utf16().count() as u32
    );
}

#[test]
fn send_of_a_path_never_interned_returns_null_tokens() {
    let daemon = Daemon::start();
    wait_until_up(daemon.reference());
    let path = source_path(daemon.reference(), "src/missing.ts");
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    assert!(client.semantic_tokens_full(uri.as_str()).is_none());
}

#[test]
fn send_of_a_present_file_with_no_iso_returns_empty_data() {
    let daemon = Daemon::start();
    wait_until_up(daemon.reference());
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(
        daemon.reference(),
        present(path.reference(), "export const x = 1;\n"),
    );
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let tokens = poll(|| client.semantic_tokens_full(uri.as_str()));
    assert!(tokens.is_empty());
}

#[test]
fn send_of_absent_after_present_returns_null_tokens() {
    let daemon = Daemon::start();
    wait_until_up(daemon.reference());
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let _present = poll(|| client.semantic_tokens_full(uri.as_str()));
    send_json(daemon.reference(), absent(path.reference()));
    poll(|| client.semantic_tokens_full(uri.as_str()).is_none().then_some(()));
}

#[test]
fn send_prefix_increments_delta_line_and_keeps_keyword() {
    let daemon = Daemon::start();
    wait_until_up(daemon.reference());
    let path = source_path(daemon.reference(), "src/Home.ts");
    send_json(daemon.reference(), present(path.reference(), CONTENTS));
    let mut client = LspClient::connect(daemon_port(daemon.reference()));
    let uri = file_uri(path.reference());
    let before = poll(|| client.semantic_tokens_full(uri.as_str()));
    send_json(
        daemon.reference(),
        present(
            path.reference(),
            &("const x = 1;\n".to_owned() + CONTENTS),
        ),
    );
    let after = poll(|| {
        let tokens = client.semantic_tokens_full(uri.as_str())?;
        (tokens[0].delta_line == 1).then_some(tokens)
    });
    assert_eq!(after[0].delta_start, before[0].delta_start);
    assert_eq!(after[0].token_type, KEYWORD);
    assert_eq!(after[0].length, 10);
}
```

`send_of_a_path_never_interned_returns_null_tokens`: no send of that path. Initialize has already succeeded, so the first `full` is the answer.

Prefix polls until `delta_line == 1` so a stale pre-prefix reply is not accepted.

Existing send tests stay. They still do not request tokens.

## Call sites

- `Daemon::start` -> `isograph start --filesystem injected`
- `send_json` -> `isograph send --file` -> `isograph/event` `Internal::DiskChanged`
- `LspClient::semantic_tokens_full` -> `textDocument/semanticTokens/full` -> `semantic_tokens_response` -> `lsp_semantic_tokens_for_file` -> `SendLspResponse`
