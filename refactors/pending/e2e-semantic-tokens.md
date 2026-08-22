# E2E: send a file, query semantic tokens

Requires file-semantic-tokens.md and the daemon answering LSP requests (lsp-socket.md; domain requests are `MethodNotFound` there). `textDocument/semanticTokens/full` is call/response on that socket. This is not a second query port, not a hidden `isograph semantic-tokens` verb, and not `Work::Query`.

Origin of send: send-events.md / lsp-socket.md. Origin of tokens: file-semantic-tokens.md `lsp_semantic_tokens_for_file`. Delta: a client of the LSP socket sends `textDocument/semanticTokens/full` and prints the `data` array. `handle` stays ingest-only.

The types and tests below still describe a hidden `isograph semantic-tokens` verb and `Work::Query`. That is not this slice. Rewrite them when this file is next in the discussion.

## What the user does

```
$ isograph start
$ isograph send --file frame.json
$ isograph semantic-tokens --path /Users/x/app/src/Home.ts
[{"delta_line":0,"delta_start":24,"length":11,"token_type":15,"token_modifiers_bitset":0}, ...]
```

`frame.json` is a `DiskChanged` `Present` whose `path` is that same absolute path and whose contents are `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. The first token is `entrypoint`: `token_type` 15 is keyword in `LEGEND_TOKEN_TYPES`. Exit 0. Not in `--help`. Does not start the daemon. The daemon not running is the same error as `isograph send`.

`isograph semantic-tokens --path /Users/x/app/src/missing.ts` with the daemon up and no `DiskFile` for that path prints `null` and exits 0.

## Types

Most important first.

`Work` is what the event loop recvs. Origin: event-loop.md `IsographEvent`. Delta: `Query`.

```rust
// from crates/isograph_cli/src/daemon.rs
enum Work {
    Event(IsographEvent),
    Query(Query, tokio::sync::oneshot::Sender<QueryResult>),
}
```

```rust
// from crates/isograph_cli/src/query.rs
#[derive(Debug, serde::Deserialize)]
#[serde(tag = "kind", content = "value")]
enum Query {
    SemanticTokens(SemanticTokensQuery),
}

#[derive(Debug, serde::Deserialize)]
struct SemanticTokensQuery {
    pub path: RelativePathToSourceFile,
}

#[derive(Debug, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
enum QueryResult {
    SemanticTokens(SemanticTokensResult),
}

#[derive(Debug, serde::Serialize)]
struct SemanticTokensResult {
    pub tokens: Option<Vec<EncodedToken>>,
}

#[derive(Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
struct EncodedToken {
    pub delta_line: u32,
    pub delta_start: u32,
    pub length: u32,
    pub token_type: u32,
    pub token_modifiers_bitset: u32,
}
```

`None` is no `DiskFile`. `Some(vec![])` is a present file with no iso tokens.

`lsp_types::SemanticToken` is mapped field-for-field to `EncodedToken` so the e2e asserts named JSON keys. Do not print lsp-types' serde shape.

```rust
// from crates/isograph_cli/src/lib.rs
    /// Print encoded semantic tokens for one interned path. Not for typing: tests and CI.
    #[command(hide = true)]
    SemanticTokens(SemanticTokensArgs),
```

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(clap::Args, Debug)]
struct SemanticTokensArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// Absolute path of the DiskFile. `handle` converts it to `RelativePathToSourceFile` against the config directory.
    #[arg(long)]
    pub path: std::path::PathBuf,
}
```

`run()` gains `Some(CliVerb::SemanticTokens(args)) => query::run(args.reference())`.

## Change 1: the worker answers queries

`run_event_loop` recvs `Work`. `Event` is `handle` then effects, as today. `Query` does not go through `handle`.

```rust
// from crates/isograph_cli/src/daemon.rs
pub(crate) async fn run_event_loop(
    mut state: IsographState,
    mut work_rx: UnboundedReceiver<Work>,
    effect_tx: UnboundedSender<IsographEffect>,
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

fn answer(state: &IsographState, query: Query) -> QueryResult {
    match query {
        Query::SemanticTokens(SemanticTokensQuery { path }) => {
            QueryResult::SemanticTokens(SemanticTokensResult {
                tokens: lsp_semantic_tokens_for_file::<TypeScriptHostLanguage>(state, path)
                    .map(|tokens| {
                        tokens
                            .into_iter()
                            .map(|t| EncodedToken {
                                delta_line: t.delta_line,
                                delta_start: t.delta_start,
                                length: t.length,
                                token_type: t.token_type,
                                token_modifiers_bitset: t.token_modifiers_bitset,
                            })
                            .collect()
                    }),
            })
        }
    }
}
```

`handle` is the free function from extract-iso-literals-from-file.md. `TypeScriptHostLanguage` is the process host. `isograph_cli` depends on `isograph_compiler`, `isograph_extract_typescript`, `isograph_lsp`.

The event socket `on_message` still emits `IsographEvent`. The listen callback sends `Work::Event(event)`.

## Change 2: query socket and the verb

A second bind `127.0.0.1:0`. Port file is the lock with extension `query`. Origin of the event port file: `discover::port_file`. Delta: `query_port_file`.

```rust
// from crates/isograph_cli/src/discover.rs
pub fn query_port_file(lock: &Path) -> PathBuf {
    lock.with_extension("query")
}
```

`run_daemon` unlinks the query port file next to the event port file, before `load_config`. `serve` binds, writes the assigned port, then the event-socket port write as today. After `select!`, unlink both.

The query listener is ours (tungstenite), not `freddie_event_socket`. One connection: read one text frame, parse `Query`, send `Work::Query` with a oneshot, write the `QueryResult` JSON, close. A frame that is not `Query` is logged and the connection closes with no write. The event socket stays ingest-only and still has no second enum.

`query::run` is `send::run` with a reply. Same lock, same `HOME`. Reads `query_port_file`. Connects, sends `{"kind":"SemanticTokens","value":{"path":...}}`, reads one text frame, prints it to stdout, exits 0. Errors: `Discover`, `NotRunning`, `Unnamed`, `Lock`, `NoPort`, `ReadPort`, `BadPort`, `Connect`, `Write`, plus `ReadReply` (the daemon wrote nothing or not `QueryResult` JSON). `thiserror` on the enum. `#[expect(clippy::print_stdout)]` on the success print. `#[expect(clippy::print_stderr)]` on the error print, same as send.

`--path` is sent as given. It is the intern key. The e2e uses one canonical path in both the `DiskChanged` frame and `--path`.

## Tests

### `discover.rs`

`query_port_file_is_the_lock_with_a_query_extension`: `/tmp/isograph-abcd.lock` → `/tmp/isograph-abcd.query`.

### `daemon.rs`

`sending_a_semantic_tokens_query_replies_none_when_the_path_is_absent`: `run_event_loop` with a oneshot `Query::SemanticTokens` for a path that was never interned. The reply is `SemanticTokens { tokens: None }`. No effects.

`sending_a_semantic_tokens_query_replies_the_tokens_for_a_present_file`: intern via `handle` a `Present` of

```
export const Home = iso(`entrypoint Query.HomeRoute`)
```

then query that path. `tokens` is `Some`. First `token_type` is 15, `length` is 11, `delta_line` is 0.

### `cli.rs`

Fixture contents: `export const Home = iso(\`entrypoint Query.HomeRoute\`)`. Path: `daemon.dir.path().join("src/Home.ts")` created only as a string key (no need to write the file to disk). Canonicalize after `create_dir_all` of `src`.

`semantic_tokens_of_a_present_iso_literal`: `Daemon::start`, poll `isograph daemon up`, send `DiskChanged` `Present` of that path and contents, `isograph semantic-tokens --path <canonical>`. Exit 0. stdout JSON: `kind` is `SemanticTokens`, `value.tokens[0].token_type` is 15, `length` is 11, `delta_line` is 0.

`semantic_tokens_append_without_touching_the_literal_is_the_same_json`: after the previous send, send `Present` of contents + `"\nconst y = 1;\n"`. Query again. stdout JSON equals the first query.

`semantic_tokens_prepend_shifts_delta_line_and_keeps_keyword`: send `Present` of `"const x = 1;\n"` + original contents. Query. First `token_type` is 15, `length` is 11, `delta_line` is 1, `delta_start` equals the first query's `delta_start`.

`semantic_tokens_of_a_missing_path_prints_null_tokens`: start, poll up, query a canonical path that was never sent. stdout `value.tokens` is JSON `null`. Exit 0.

`semantic_tokens_with_the_daemon_stopped_fails`: write a config, do not start, run the verb. Exit 1. stderr contains `not running`.

`semantic_tokens_is_not_in_help`: `--help` contains `start`, does not contain `semantic-tokens`.

`send_of_disk_changed_present_then_absent_exits_0` and `Daemon::start` stay. They do not call this verb.
