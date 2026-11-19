# The daemon compiles continuously; `isograph errors` shows the errors

The compiler is not a separate process. It is work the daemon does while it runs. Every applied `DiskChanged` arms a compile debounce. When the debounce fires, `handle` computes the errors of every source file through the same memos the LSP uses, as read by the compiler's own `ClientId`, which never has an open buffer, so it reads the disk view. The source files are the paths `source_file_paths(db, client)` lists from `source_path_map`; `handle` classified each path once, with `ProjectGlobs::membership`, when it applied the `DiskChanged` or the `didOpen`. Nothing is stored beyond memo results. After the errors, the same compile fire generates artifacts (refactors/pending/artifact-generation.md).

`isograph errors` is a CLI verb. It starts the daemon if needed, connects to the LSP port like `isograph send`, performs the LSP initialize handshake, sends the custom request `isograph/errors`, prints one line per error, prints a count, and exits 1 if any error exists and 0 otherwise. `handle` answers `isograph/errors` synchronously from the memos, as the requesting connection's `ClientId` reads them, so the command never waits on the debounce. The CLI connection opens no buffers, so its answer is the disk view. `isograph errors --watch` keeps the connection open and prints the errors each time they change, through a server-to-client notification that `handle` returns after each compile.

Three prefactors make this correct. The watcher's boot scan is posted onto the event channel before the LSP port accepts, so a request on a new connection is handled after every boot `DiskChanged`. `ClientId`s are minted by `handle` from a counter on `IsographState`, so the compiler's id comes from the same source as every connection's id. The client side of the LSP handshake and the spawning of `isograph start` move to shared functions.

Builds on: `file_contents` and the client-keyed iso literal memos, `iso_literal_parse_errors_in_file`, `iso_literal_validation_errors_in_file`, `line_index`, the diagnostics debounce (refactors/past/lsp-diagnostics.md), `freddie_timer`, `LiveSessions`, and the path maps `source_path_map` and `schema_path_map` with their listing memos `source_file_paths` (`crates/isograph_compiler/src/database.rs`) and `schema_file_paths` (`crates/graphql_network_protocol/src/schema_files.rs`).

Depends on, for change 9 only: `parsed_schema_file` from refactors/past/graphql-schema-parser/integration.md.

## What the user does

The project has `src/Home.ts` containing `` export const Home = iso(` `); `` and the daemon is stopped. The user runs `isograph errors` in the project root. The command starts the daemon, waits for it to answer, and prints:

```text
src/Home.ts:1:26: Expected a declaration. An isograph literal cannot be empty.
1 error
```

It exits 1. The daemon keeps running. The user fixes the literal to `` iso(`entrypoint Query.HomeRoute`) `` and saves. About 100ms later the daemon has recompiled. The user runs `isograph errors` again. It prints `0 errors` and exits 0. A second run with no edit between answers from the memos without reparsing anything.

Each line is `path:line:column: message`. `path` is relative to the directory the user ran the command in (`../other/a.ts` when the file is outside it). `line` and `column` are 1-based. `column` counts UTF-16 code units from the line start, plus one; this is the LSP column plus one, it equals the character column for text without characters outside the Basic Multilingual Plane, and it is the column VS Code and other LSP editors jump to for `path:line:column`. Lines are sorted by path, then by start position. The last line is `N errors`, or `1 error`.

Errors are what the files on disk say. Unsaved edits in an editor never appear: `isograph errors` is its own connection with its own `ClientId` and opens no buffers, and the compile fire reads as `compiler_client`, which has no buffers either. A file the config's `source_files` globs exclude, or that the host language skips, is not in `source_path_map` and has no lines. Schema files (`schema` globs) have no lines of their own until change 9; an iso literal that selects a field the schema does not define is a validation error on the source file's line.

`isograph errors` with the daemon already running dials it and does not restart it. `isograph errors` with no config, a broken config, or a daemon that cannot start prints the reason to stderr and exits 2. Exit 2 means the command could not obtain the errors; exit 1 means it obtained them and there is at least one.

`isograph errors --watch` prints the current errors and the count, then stays connected. Each time a compile produces a different list, it prints a blank line, then the new list and its count. A compile whose list is equal to the last one printed prints nothing. Ctrl-C ends it. If the daemon stops, it prints `the daemon closed the connection` to stderr and exits 2.

`isograph logs` shows `compiled errors=N` after each compile.

## Design decisions this doc fixes

### `handle` mints every `ClientId`

`accept_loop` runs on its own task and currently mints ids from a local `next` counter. The id moves onto `IsographState` as `client_ids: ClientIdSource`. `IsographState::new` mints the compiler's id first and stores it as `compiler_client`. Every connection's id comes from the same `ClientIdSource`, so no connection can receive the compiler's id.

The session thread learns its id through a channel. After the initialize handshake, the session sends `ClientConnectionEvent::Started(ClientStartedEvent { session, assign })`, where `assign` is a `crossbeam::channel::Sender<ClientId>`, and blocks on the matching receiver. `handle` of `Started` mints the id, inserts the `LiveSession` under it, and returns `IsographEffect::AssignClientId(AssignClientIdEffect { assign, client })`. `perform` sends the id. The session thread wakes, and only then reads LSP messages from its connection and tags each with that id.

Consequences:

- Every `LspRequest`, `LspNotification`, and `LspResponse` of a connection is posted after `handle` processed that connection's `Started`, because the session forwards nothing until it holds the id. `handle` therefore never sees a message from a connection whose `Started` it has not processed. The `live_sessions` checks in `dispatch_lsp` stay; they cover messages after `Gone`.
- `Gone(client)` is posted only by a session that received an id, after its message loop ends. A connection that drops before or during initialize, or whose `InitializeParams` do not parse, posts nothing. Today it posts `Gone` of an id `handle` never saw.
- A client that disconnects while its session waits for the id still gets the id. The message loop then ends at once and the session posts `Gone`.
- Messages a client writes while its session waits stay in the socket and the reader thread's `bounded(0)` channel. None is lost.
- If the effect loop stops before performing `AssignClientId` (a `Kill` is ahead of it), the process exits. The blocked session thread exits with it.
- Ids are unique by construction, so `Started` of an id already in `live_sessions` cannot happen. The test `client_started_twice_for_the_same_id_keeps_the_second_session` is deleted.

The alternative of the session minting its own id and `handle` remapping it needs a second id space and a table in `handle`. The alternative of `handle` sending on the channel itself is I/O inside `handle`. The effect is the shape `run_event_loop` already supports.

### A request on a new connection is answered after the boot scan

`serve` writes the port file, then `crate::watch::start` scans the config directory and sends the boot batch on `watch_tx`, then `select!` starts `accept_loop` and `run_filesystem_watcher`. `run_filesystem_watcher` posts the boot `DiskChanged`s from inside `select!`. A session thread runs on its own OS thread, so its `Started` and its first request can reach `event_tx` while the boot batch is still being posted. `isograph errors` against a fresh daemon could then report zero errors for a project full of errors.

Change 1 removes this. `start` returns the boot batch instead of sending it, and `serve` calls `crate::watch::apply` on it before `select!`. `apply` posts every boot `DiskChanged` onto `event_tx` synchronously. `accept_loop` is not polled until `select!`, so every session event is behind every boot `DiskChanged` on the one FIFO channel. Connections the kernel queued in the listen backlog during the scan are accepted after it.

### `isograph/errors` computes the requesting client's errors

`errors_response` is one more arm of the existing request dispatch chain over `&IsographDb`. It receives the requesting connection's `ClientId`, like every other request handler, and returns `project_errors` as that client reads the files: the source files `source_file_paths(db, client)` lists, each read through `file_contents(db, client, path)`. A connection with no open buffers, which is every `isograph errors` connection, reads disk. An editor connection that sent the request would see its own buffers. There is no second dispatch chain, and the compiler's `ClientId` is not stored in the database as a singleton for the handler to read; `compiler_client` stays a field on `IsographState` that only the compile fire reads.

### `isograph/errors` does not wait

The request handler computes the errors in `handle` from the memos. It does not read a stored result and does not wait for the compile debounce. Because of change 1, every boot `DiskChanged` has been handled before the request is. If the debounce has fired, the computation is memo hits. If it has not, the request does the work and the later fire is memo hits. Either way the answer reflects every `DiskChanged` handled so far. Nothing polls.

## Changes

Each change is one commit, prefactors first. Each lists its tests. Each amends the design docs named in Docs this doc amends, in the same commit.

1. The watcher returns the boot scan, and `serve` posts it before the LSP port accepts. Landed.
2. `handle` mints every `ClientId` from `IsographState`. Landed.
3. The daemon compiles after every applied `DiskChanged`. Landed.
4. Shared LSP client handshake and `isograph start` spawning.
5. The `isograph/errors` request.
6. The `isograph errors` verb.
7. `isograph errors --watch`.
8. Validation errors are compiler errors. Landed.
9. Schema parse errors are compiler errors (after schema integration change 2 lands).

## Change 1: the boot scan is posted before the LSP port accepts (landed)

### `StartedWatcher`

```rust
// from crates/isograph_cli/src/watch.rs
/// A running watcher and the files that matched when it started.
pub struct StartedWatcher {
    pub watcher: Watcher,
    pub boot: Vec<SourceFileEvent>,
}
```

### `start_if_watching` and `start`

Before:

```rust
// from crates/isograph_cli/src/watch.rs
pub fn start_if_watching<TProfile: CompilationProfile>(
    filesystem: Filesystem,
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    globs: ProjectGlobs,
) -> Result<Option<Watcher>, WatchError> {
    match filesystem {
        Filesystem::Injected => None.wrap_ok(),
        Filesystem::Watch => {
            start::<TProfile>(watch_tx, config_path, globs).map(|watcher| watcher.wrap_some())
        }
    }
}
```

```rust
// from crates/isograph_cli/src/watch.rs (tail of start)
    info!(n = boot.len(), "scan finished");
    if !boot.is_empty() {
        let _ = watch_tx.send(boot);
    }
    Watcher {
        _debouncer: debouncer,
    }
    .wrap_ok()
}
```

After:

```rust
// from crates/isograph_cli/src/watch.rs
pub fn start_if_watching<TProfile: CompilationProfile>(
    filesystem: Filesystem,
    watch_tx: UnboundedSender<Vec<SourceFileEvent>>,
    config_path: &Path,
    globs: ProjectGlobs,
) -> Result<Option<StartedWatcher>, WatchError> {
    match filesystem {
        Filesystem::Injected => None.wrap_ok(),
        Filesystem::Watch => {
            start::<TProfile>(watch_tx, config_path, globs).map(|started| started.wrap_some())
        }
    }
}
```

`start` returns `Result<StartedWatcher, WatchError>`. Its body is unchanged through the construction of `boot`. Its tail:

```rust
// from crates/isograph_cli/src/watch.rs (tail of start)
    info!(n = boot.len(), "scan finished");
    StartedWatcher {
        watcher: Watcher {
            _debouncer: debouncer,
        },
        boot,
    }
    .wrap_ok()
}
```

Delta: the boot batch is returned, not sent on `watch_tx`. The `is_empty` check is gone; an empty batch is applied as nothing. `watch_tx` still goes to the debouncer callback, so notify batches still arrive on it.

### `serve`

Before:

```rust
// from crates/isograph_cli/src/daemon.rs
    let _hold_watch = watch_tx.clone();
    let _watcher = match crate::watch::start_if_watching::<TProfile>(
        filesystem,
        watch_tx,
        config_path.reference(),
        globs.clone(),
    ) {
        Ok(watcher) => watcher,
        Err(e) => {
            tracing::error!(error = %e, "could not start the watcher");
            return;
        }
    };
```

After:

```rust
// from crates/isograph_cli/src/daemon.rs
    let _hold_watch = watch_tx.clone();
    let started = match crate::watch::start_if_watching::<TProfile>(
        filesystem,
        watch_tx,
        config_path.reference(),
        globs.clone(),
    ) {
        Ok(started) => started,
        Err(e) => {
            tracing::error!(error = %e, "could not start the watcher");
            return;
        }
    };
    let (_watcher, boot) = match started {
        Some(crate::watch::StartedWatcher { watcher, boot }) => (watcher.wrap_some(), boot),
        None => (None, Vec::new()),
    };
    // accept_loop is first polled inside select!, so every session's events queue behind these.
    crate::watch::apply::<TProfile>(
        event_tx.reference(),
        config_directory.reference(),
        boot,
        globs.reference(),
    );
```

`config_directory` is already computed above this block.

### Tests

`crates/isograph_cli/src/watch.rs`:

- `boot_interns_a_file_written_before_start` reads `started.boot` instead of `watch_rx.try_recv()`, ingests it, and asserts the contents as today.
- `empty_source_files_boot_posts_nothing` asserts `started.boot.is_empty()` and `watch_rx.try_recv().is_err()`.
- `notify_interns_a_file_written_after_start` drops its `let _ = watch_rx.try_recv();`.
- New `boot_is_not_sent_on_the_watch_channel`: a matching file exists before `start`; `started.boot` has one `CreateOrModify` of it; `watch_rx.try_recv()` is `Err(TryRecvError::Empty)`.

`crates/ts_graphql_react_isograph_cli/tests/cli.rs`:

- New `semantic_tokens_right_after_a_watch_start_see_the_boot_scan`: write `src/Home.ts` with `CONTENTS`, `Daemon::start_watch("{\"source_files\":[\"src/**\"]}")`, `LspClient::connect` with no `settle()`, request `semanticTokens/full` of its URI. The first token is the `entrypoint` keyword (`KEYWORD`, length 10). Before this change the request can race the boot scan and return null.

## Change 2: `handle` mints every `ClientId` (landed)

### `ClientIdSource`

```rust
// from crates/isograph_cli/src/state.rs
/// Hands out each `ClientId` once.
#[derive(Debug, Default)]
pub(crate) struct ClientIdSource(u64);

impl ClientIdSource {
    pub(crate) fn mint(&mut self) -> ClientId {
        let client = ClientId(self.0);
        self.0 += 1;
        client
    }
}
```

The field is private, so the only way to obtain the next id is `mint`.

### `IsographState`

Before:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
    client_ids: ClientIdSource,
}

impl<TProfile: CompilationProfile> IsographState<TProfile> {
    pub(crate) fn new(globs: ProjectGlobs) -> Self {
        Self {
            db: IsographDb::default(),
            live_sessions: LiveSessions::default(),
            globs,
            diagnostics_debounce: None,
            client_ids: ClientIdSource::default(),
        }
    }
}
```

`client_ids` is private, so a struct literal outside `state.rs` does not compile and every `IsographState` is built by `new`. Callers that used a struct literal call `IsographState::new(globs)` and then `intern_config_directory(&mut state.db, ...)` where they interned before: `serve` (`daemon.rs`), `empty_state` (`daemon.rs` tests), `listen_and_reply` (`lsp_socket.rs` tests), `Harness::with_schema` (`watch.rs` tests), `empty` (`state.rs` tests).

### `ClientStartedEvent`

Before:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct ClientStartedEvent {
    pub client: ClientId,
    pub session: LiveSession,
}
```

After:

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct ClientStartedEvent {
    pub session: LiveSession,
    /// The session's thread waits on the other end for the `ClientId` `handle` mints.
    pub assign: crossbeam::channel::Sender<ClientId>,
}
```

### `AssignClientIdEffect`

```rust
// from crates/isograph_cli/src/effect.rs
/// The `ClientId` `handle` minted for one connection, and the channel its session waits on.
#[derive(Debug)]
pub struct AssignClientIdEffect {
    pub assign: crossbeam::channel::Sender<ClientId>,
    pub client: ClientId,
}

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    PublishDiagnostics(Box<PublishDiagnosticsEffect>),
    AssignClientId(AssignClientIdEffect),
    Timer(TimerEffect<TimerPayload>),
}
```

### `handle_client_connection`

Before:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Started(started) => {
            state
                .live_sessions
                .0
                .insert(started.client, started.session);
            no_effects()
        }
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        ClientConnectionEvent::Started(started) => {
            let client = state.client_ids.mint();
            state.live_sessions.0.insert(client, started.session);
            IsographEffect::AssignClientId(crate::effect::AssignClientIdEffect {
                assign: started.assign,
                client,
            })
            .wrap_vec()
        }
```

The `Gone` arm is unchanged.

### `perform`

New arm in `perform` (`crates/isograph_cli/src/daemon.rs`), between `PublishDiagnostics` and `Timer`:

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::AssignClientId(assign) => {
            let _ = assign.assign.send(assign.client);
            ControlFlow::Continue(())
        }
```

A send fails only when the session thread is gone; the thread is blocked on the receiver until the send, so this does not happen in practice and is ignored like the other sends in `perform`.

### `accept_loop`, `session`, `start_session`, `forward_messages`

Before:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let mut next = 1u64;
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let client = isograph_compiler::ClientId(next);
                next += 1;
                let event_tx = event_tx.clone();
                let std_stream = match stream.into_std() {
                    Ok(std_stream) => std_stream,
                    Err(e) => {
                        debug!(error = %e, %peer, "could not take the lsp stream");
                        continue;
                    }
                };
                if let Err(e) = std_stream.set_nonblocking(false) {
                    debug!(error = %e, %peer, "could not set the lsp stream blocking");
                    continue;
                }
                std::thread::spawn(move || session(std_stream, event_tx, client));
            }
            Err(e) => debug!(error = %e, "accept failed"),
        }
    }
}

fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: isograph_compiler::ClientId,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    run_session(&connection, event_tx.clone(), client);
    let _ = event_tx.send(crate::event::ClientConnectionEvent::Gone(client).to());
    drop(connection);
    let _ = io_threads.join();
}
```

After:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
pub(crate) async fn accept_loop(
    listener: tokio::net::TcpListener,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    loop {
        match listener.accept().await {
            Ok((stream, peer)) => {
                let event_tx = event_tx.clone();
                let std_stream = match stream.into_std() {
                    Ok(std_stream) => std_stream,
                    Err(e) => {
                        debug!(error = %e, %peer, "could not take the lsp stream");
                        continue;
                    }
                };
                if let Err(e) = std_stream.set_nonblocking(false) {
                    debug!(error = %e, %peer, "could not set the lsp stream blocking");
                    continue;
                }
                std::thread::spawn(move || session(std_stream, event_tx));
            }
            Err(e) => debug!(error = %e, "accept failed"),
        }
    }
}

fn session(
    stream: std::net::TcpStream,
    event_tx: tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) {
    let (connection, io_threads) = match connection_from_stream(stream) {
        Ok(pair) => pair,
        Err(e) => {
            debug!(error = %e, "lsp transport");
            return;
        }
    };
    if let Some(client) = start_session(connection.reference(), event_tx.reference()) {
        forward_messages(connection.reference(), event_tx.reference(), client);
        let _ = event_tx.send(crate::event::ClientConnectionEvent::Gone(client).to());
    }
    drop(connection);
    let _ = io_threads.join();
}
```

`run_session` splits in two. `start_session` is `run_session` from its first line through the `Started` send. Delta: the `client` parameter is gone, `event_tx` is borrowed, each early `return;` is `return None;`, the `ClientStartedEvent` literal has `assign` instead of `client`, a failed `Started` send returns `None`, and the function ends by waiting for the id:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
/// Initializes, posts `Started`, and returns the `ClientId` `handle` assigned. `None` when
/// initialize fails or the daemon dropped the assignment.
fn start_session(
    connection: &lsp_server::Connection,
    event_tx: &tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
) -> Option<isograph_compiler::ClientId> {
    let capabilities = match serde_json::to_value(lsp_types::ServerCapabilities {
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
        document_formatting_provider: lsp_types::OneOf::Left(true).wrap_some(),
        hover_provider: lsp_types::HoverProviderCapability::Simple(true).wrap_some(),
        document_highlight_provider: lsp_types::OneOf::Left(true).wrap_some(),
        text_document_sync: lsp_types::TextDocumentSyncCapability::Options(
            lsp_types::TextDocumentSyncOptions {
                open_close: true.wrap_some(),
                change: lsp_types::TextDocumentSyncKind::FULL.wrap_some(),
                will_save_wait_until: true.wrap_some(),
                ..Default::default()
            },
        )
        .wrap_some(),
        ..Default::default()
    }) {
        Ok(value) => value,
        Err(e) => {
            debug!(error = %e, "server capabilities");
            return None;
        }
    };
    let params = match connection.initialize(capabilities) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "lsp initialize");
            return None;
        }
    };
    let params = match serde_json::from_value::<lsp_types::InitializeParams>(params) {
        Ok(params) => params,
        Err(e) => {
            debug!(error = %e, "initialize params");
            return None;
        }
    };
    let (assign, assigned) = crossbeam::channel::bounded(1);
    event_tx
        .send(
            crate::event::ClientConnectionEvent::Started(crate::event::ClientStartedEvent {
                session: crate::event::LiveSession {
                    sender: connection.sender.clone(),
                    capabilities: params.capabilities,
                    published_diagnostics: crate::diagnostics::PublishedDiagnostics::default(),
                    outstanding: std::collections::HashSet::new(),
                },
                assign,
            })
            .to(),
        )
        .ok()?;
    assigned.recv().ok()
}
```

`forward_messages` is the `for message in &connection.receiver` loop of `run_session`, verbatim:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
fn forward_messages(
    connection: &lsp_server::Connection,
    event_tx: &tokio::sync::mpsc::UnboundedSender<crate::event::IsographEvent>,
    client: isograph_compiler::ClientId,
) {
    for message in &connection.receiver {
        let event = match message {
            lsp_server::Message::Request(request) => {
                crate::event::LspEvent::Request(crate::event::LspRequest { client, request })
            }
            lsp_server::Message::Notification(notification) => {
                crate::event::LspEvent::Notification(crate::event::LspNotification {
                    client,
                    notification,
                })
            }
            lsp_server::Message::Response(response) => {
                crate::event::LspEvent::Response(crate::event::LspResponse { client, response })
            }
        };
        let _ = event_tx.send(event.to());
    }
}
```

### Tests

`crates/isograph_cli/src/state.rs`:

- The helpers that start a client return the minted id instead of taking one: `start_client(state, sender) -> ClientId`, `started(state) -> ClientId`, `start_a_client(state) -> ClientId`, `start_with_capabilities(state, capabilities) -> (ClientId, Receiver<lsp_server::Message>)`, `diagnostics_client(state) -> (ClientId, Receiver<lsp_server::Message>)`. Each reads the id from the one `AssignClientId` effect `handle` returned:

```rust
// from crates/isograph_cli/src/state.rs (tests)
fn assigned(effects: Vec<IsographEffect>) -> ClientId {
    let [IsographEffect::AssignClientId(assign)] = <[IsographEffect; 1]>::try_from(effects)
        .expect("Started returns exactly one effect")
    else {
        panic!("Started returns AssignClientId");
    };
    assign.client
}
```

- Every test that wrote `ClientId(1)` or `ClientId(2)` for a started client binds the returned id instead. Tests of a client that never started keep a literal id.
- `client_started_stores_the_session` asserts the session is stored under the assigned id.
- New `two_started_clients_get_different_ids`.
- New `performing_assign_client_id_sends_the_id_to_the_session`: perform the effect with a `bounded(1)` sender; the receiver yields the assigned id.
- Deleted: `client_started_twice_for_the_same_id_keeps_the_second_session`. Its state is unrepresentable.

`crates/isograph_cli/src/lsp_socket.rs`:

- `listen_for_events` tests play `handle`: a new helper `take_started_and_assign(event_rx, client) -> ClientStartedEvent` awaits `Started` with `event_rx.recv().await` and sends `client` on `started.assign`. Tests that expected `started.client` use the id they assigned. Events that must arrive are awaited with `recv().await`, not read with `try_recv` after `SETTLE`.
- New `a_session_forwards_nothing_until_assigned`: initialize, write a hover request, await `Started` without assigning, write `isograph/event` `HelloWorld`, assign `ClientId(7)`, then the next two events are the hover request and the notification, both tagged `ClientId(7)`, in that order.
- New `a_session_whose_assignment_is_dropped_posts_no_gone`: initialize, await `Started`, drop `started.assign` unsent, drop the connection; start a second connection, initialize, await its `Started`; no `Gone` arrived before it.
- `drop_before_initialize_is_gone_without_started` becomes `drop_before_initialize_posts_nothing`, asserted the same way: a second connection's `Started` is the first event.
- `initialize_params_that_are_not_initialize_params_are_gone_without_started` becomes `..._post_nothing`, asserted the same way.
- `two_connections_get_different_ids_and_dropping_one_posts_one_gone` assigns two different ids and asserts the dropped one's `Gone` carries its id. Uniqueness itself is `two_started_clients_get_different_ids`.

The `listen_and_reply` tests run `run_event_loop` and `run_effect_loop`, so they exercise the real assignment and pass unchanged.

## Change 3: the daemon compiles after every applied `DiskChanged` (landed)

### `CompilerErrorReport`

```rust
// from crates/isograph_cli/src/compile.rs
/// One compiler error, positioned for a terminal.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompilerErrorReport {
    /// Absolute: the config directory joined with the source path.
    pub path: PathBuf,
    /// LSP positions: 0-based lines, UTF-16 code unit columns.
    pub range: lsp_types::Range,
    pub message: String,
}
```

Change 5 adds `serde::Serialize, serde::Deserialize`.

### `CompilerError` and `compiler_errors_in_file`

New module `crates/isograph_compiler/src/compiler_errors.rs`, exported from `lib.rs` as `pub use compiler_errors::{CompilerError, compiler_errors_in_file};`.

Change 3 added `CompilerError` with the one variant `Parse(ParseError)`; change 8 adds `Validation(ValidationError)` and shows the memo. This memo is where validation (change 8) and schema (change 9) errors join. `lsp_diagnostics_for_file` reads `iso_literal_parse_errors_in_file` and `iso_literal_validation_errors_in_file` directly.

### `source_file_paths`

The compile lists source files through the existing memo over `source_path_map`. It does not filter `disk_file_map` by the globs a second time; `handle` classified every path when it applied the event that stored it.

```rust
// from crates/isograph_compiler/src/database.rs
/// Every source file path `client` reads: disk paths and `client`'s open paths, sorted.
#[memo]
pub fn source_file_paths<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> Vec<RelativePathToSourceFile> {
    let view = db.get_source_path_map();
    let map = view.tracked();
    map.disk
        .iter()
        .copied()
        .chain(
            map.open
                .iter()
                .filter(|key| key.client == client)
                .map(|key| key.path),
        )
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
```

It iterates the map, so it reads `tracked()` (AGENTS.md, pico tracked maps). For `compiler_client`, which has no `OpenFileKey`, the list is `source_path_map.disk`. "Sorted" is the `Ord` of the interned path, not the text order, so `project_errors` sorts its reports and `project_artifact_sources` sorts its paths by `lookup()`. `IsographDb::disk_paths` is removed; nothing reads it.

### `absolute_path`

Extracted from `uri_for_path` (`crates/isograph_cli/src/adapter.rs`).

Before:

```rust
// from crates/isograph_cli/src/adapter.rs
pub(crate) fn uri_for_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    path: RelativePathToSourceFile,
) -> Result<lsp_types::Uri, PathUriError> {
    let cwd = db
        .get_singleton::<CurrentWorkingDirectory>()
        .ok_or(PathUriError::MissingWorkingDirectory)?
        .dereference();
    let absolute = normalize_lexically(Path::new(cwd.lookup()).join(path.lookup()).as_path());
    let url =
        url::Url::from_file_path(absolute.as_path()).map_err(|()| PathUriError::NotAbsolute)?;
    url.as_str()
        .parse::<lsp_types::Uri>()
        .map_err(PathUriError::InvalidUri)
}
```

After:

```rust
// from crates/isograph_cli/src/adapter.rs
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[error("CurrentWorkingDirectory is not interned")]
pub(crate) struct MissingWorkingDirectory;

/// `path` joined onto the config directory, with `.` and `..` resolved lexically.
pub(crate) fn absolute_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    path: RelativePathToSourceFile,
) -> Result<PathBuf, MissingWorkingDirectory> {
    let cwd = db
        .get_singleton::<CurrentWorkingDirectory>()
        .ok_or(MissingWorkingDirectory)?
        .dereference();
    normalize_lexically(Path::new(cwd.lookup()).join(path.lookup()).as_path()).wrap_ok()
}

pub(crate) fn uri_for_path<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    path: RelativePathToSourceFile,
) -> Result<lsp_types::Uri, PathUriError> {
    let absolute = absolute_path(db, path)
        .map_err(|MissingWorkingDirectory| PathUriError::MissingWorkingDirectory)?;
    let url =
        url::Url::from_file_path(absolute.as_path()).map_err(|()| PathUriError::NotAbsolute)?;
    url.as_str()
        .parse::<lsp_types::Uri>()
        .map_err(PathUriError::InvalidUri)
}
```

`PathUriError` is unchanged.

### `IsographState`: `compiler_client`, `compile_debounce`, `compile_input`

After (relative to change 2):

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<TProfile: CompilationProfile> {
    pub db: IsographDb<TProfile>,
    pub live_sessions: LiveSessions,
    pub(crate) globs: ProjectGlobs,
    pub(crate) diagnostics_debounce: Option<freddie_timer::TimerGuard>,
    pub(crate) compile_debounce: Option<freddie_timer::TimerGuard>,
    client_ids: ClientIdSource,
    /// The compiler's reader of file text. No `LiveSession` and no `OpenFile` has it.
    compiler_client: ClientId,
}

impl<TProfile: CompilationProfile> IsographState<TProfile> {
    pub(crate) fn new(globs: ProjectGlobs) -> Self {
        let mut client_ids = ClientIdSource::default();
        let compiler_client = client_ids.mint();
        Self {
            db: IsographDb::default(),
            live_sessions: LiveSessions::default(),
            globs,
            diagnostics_debounce: None,
            compile_debounce: None,
            client_ids,
            compiler_client,
        }
    }

    pub(crate) fn compile_input(&self) -> crate::compile::CompileInput<'_, TProfile> {
        crate::compile::CompileInput {
            db: self.db.reference(),
            client: self.compiler_client,
        }
    }
}
```

Artifact generation later added `artifact_folder` and `artifact_files` (refactors/pending/artifact-generation.md).

`compiler_client` is minted once and never inserted into `live_sessions`. `set_open_file` is called only by `didOpen` and `didChange` of a client in `live_sessions`, so no `OpenFile` has `compiler_client`, and `file_contents(db, compiler_client, path)` is always the `DiskFile`.

### `CompileInput`

```rust
// from crates/isograph_cli/src/compile.rs
/// What compile reads.
pub(crate) struct CompileInput<'a, TProfile: CompilationProfile> {
    pub db: &'a IsographDb<TProfile>,
    pub client: ClientId,
}
```

`compile_input` builds it with `client: compiler_client`. Change 5's `errors_response` and change 7's subscriber notifications build it with the requesting or subscribed connection's `ClientId`.

### `TimerPayload`

Before:

```rust
// from crates/isograph_cli/src/timer.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    DiagnosticsDebounce,
}
```

After:

```rust
// from crates/isograph_cli/src/timer.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    DiagnosticsDebounce,
    CompileDebounce,
}
```

### `crates/isograph_cli/src/compile.rs`

```rust
// from crates/isograph_cli/src/compile.rs
pub(crate) const COMPILE_DEBOUNCE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum ProjectErrorsError {
    #[error("{0}")]
    MissingWorkingDirectory(MissingWorkingDirectory),
    #[error("{} is a listed source file but has no text", .0.lookup())]
    ListedPathWithoutText(RelativePathToSourceFile),
}

/// Replaces the pending compile timer with one that fires after `COMPILE_DEBOUNCE`.
pub(crate) fn arm_compile_debounce<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let (guard, effect) =
        freddie_timer::timer_effect_and_guard(COMPILE_DEBOUNCE, TimerPayload::CompileDebounce);
    state.compile_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

/// Computes every source file's errors, so later reads are memo hits, and logs the count.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    no_effects()
}

/// Every error of every source file on disk as `input.client` reads it, sorted by path,
/// then by start position.
pub(crate) fn project_errors<TProfile: CompilationProfile>(
    input: &CompileInput<'_, TProfile>,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let per_file = source_file_paths(input.db, input.client)
        .iter()
        .copied()
        .map(|path| file_reports(input.db, input.client, path))
        .collect::<Result<Vec<_>, _>>()?;
    let mut reports: Vec<CompilerErrorReport> = per_file.into_iter().flatten().collect();
    reports.sort_by(|a, b| {
        a.path
            .cmp(b.path.reference())
            .then(a.range.start.cmp(b.range.start.reference()))
    });
    reports.wrap_ok()
}

fn file_reports<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Result<Vec<CompilerErrorReport>, ProjectErrorsError> {
    let errors = compiler_errors_in_file(db, client, path)
        .as_ref()
        .ok_or(ProjectErrorsError::ListedPathWithoutText(path))?;
    if errors.is_empty() {
        return Vec::new().wrap_ok();
    }
    let line_index = isograph_lsp::line_index(db, client, path)
        .as_ref()
        .ok_or(ProjectErrorsError::ListedPathWithoutText(path))?;
    let absolute =
        crate::adapter::absolute_path(db, path).map_err(ProjectErrorsError::MissingWorkingDirectory)?;
    errors
        .iter()
        .map(|error| CompilerErrorReport {
            path: absolute.clone(),
            range: line_index.range(error.location),
            message: error.item.to_string(),
        })
        .collect::<Vec<_>>()
        .wrap_ok()
}
```

`sort_by` is stable, so two errors with one start keep the memo's order. `lsp_types::Position` is `Ord` (line, then character); `Range` is not, so the key is `range.start`.

`ListedPathWithoutText` is a broken invariant: `source_file_paths(db, client)` and the per-path reads run in one `handle` call on one database, and every listed path is in `source_path_map.disk`, which `insert_disk_file` writes only beside a `DiskFile`, or is an `OpenFileKey` of `client`, which `set_open_file` writes only beside that client's `OpenFile`. Either way `file_contents(db, client, path)` is `Some`. The type system cannot express that a listed key's lookup hits. It is an error value, not a panic, and change 5 surfaces it as `LSPRuntimeError::UnexpectedError`.

### `handle_disk_changed`

Before:

```rust
// from crates/isograph_cli/src/state.rs
        InternalEvent::DiskChanged(change) => {
            handle_disk_changed(&mut state.db, state.globs.reference(), change)
        }
```

```rust
// from crates/isograph_cli/src/state.rs
fn handle_disk_changed<TProfile: CompilationProfile>(
    db: &mut IsographDb<TProfile>,
    globs: &ProjectGlobs,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(db, globs, change)
        .unwrap_or_else(|error| tracing::warn!(%error, "DiskChanged path"));
    no_effects()
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
        InternalEvent::DiskChanged(change) => handle_disk_changed(state, change),
```

```rust
// from crates/isograph_cli/src/state.rs
fn handle_disk_changed<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    change: DiskChanged,
) -> Vec<IsographEffect> {
    apply_disk_changed(&mut state.db, state.globs.reference(), change).map_or_else(
        |error| {
            tracing::warn!(%error, "DiskChanged path");
            no_effects()
        },
        |()| crate::compile::arm_compile_debounce(state),
    )
}
```

Same shape as `on_did_open`. A `DiskChanged` whose path is rejected arms nothing. `didOpen`, `didChange`, and `didClose` do not arm the compile debounce; `compiler_client` does not read buffers. The current `handle_disk_changed` also arms the diagnostics debounce, before the compile debounce.

### `handle_timer_fired`

Before:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_timer_fired<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.diagnostics_debounce.as_ref()) {
        Some(&TimerPayload::DiagnosticsDebounce) => {
            state.diagnostics_debounce = None;
            crate::diagnostics::publish_diagnostics(state)
        }
        None => no_effects(),
    }
}
```

After:

```rust
// from crates/isograph_cli/src/state.rs
fn handle_timer_fired<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    let payload = fired
        .trigger_if_matching(state.diagnostics_debounce.as_ref())
        .or_else(|| fired.trigger_if_matching(state.compile_debounce.as_ref()));
    match payload {
        Some(&TimerPayload::DiagnosticsDebounce) => {
            state.diagnostics_debounce = None;
            crate::diagnostics::publish_diagnostics(state)
        }
        Some(&TimerPayload::CompileDebounce) => {
            state.compile_debounce = None;
            crate::compile::compile(state)
        }
        None => no_effects(),
    }
}
```

A `TimerId` matches at most one guard. A stale firing of either timer matches neither guard and returns no effects.

### Tests

`crates/isograph_compiler/src/compiler_errors.rs`:

- `missing_file_has_no_errors`: `None`.
- `a_file_with_no_literals_has_no_errors`: `Some(vec![])`.
- `parse_errors_are_compiler_errors_with_the_same_spans`: for `` iso(`entrypoint`) ``, the result equals `iso_literal_parse_errors_in_file` mapped through `CompilerError::Parse`.
- `a_whitespace_only_literal_is_the_empty_literal_error`: exactly one `CompilerError::Parse(ParseError::Ast(AstError::EmptyLiteral))`.

`crates/isograph_compiler/src/database.rs`: the listing tests read `source_path_map` and `schema_path_map` (`a_source_disk_file_is_listed_as_source_only`, `remove_disk_files_from_path_removes_listed_descendants_only`, and the rest of that group). The three `disk_paths` tests are removed with `disk_paths`.

`crates/isograph_cli/src/compile.rs` (state built with `IsographState::new` and `intern_config_directory` of `/tmp/proj/isograph.config.json`, globs `src/**` and schema `schema/**`):

- `project_errors_of_a_broken_disk_file_has_its_absolute_path_range_and_message`: path `/tmp/proj/src/a.ts`; the ranges and messages equal `lsp_diagnostics_for_file(db, compiler_client, path)` of the same file.
- `project_errors_ignore_an_open_buffer`: a started client opens `src/a.ts` with `BROKEN` while disk is `CLEAN`; empty. `CLEAN` is `` iso(`entity Query`) iso(`entrypoint Query.HomeRoute`) iso(`field Query.HomeRoute`) ``, which also has no validation error.
- `project_errors_read_disk_when_a_client_has_a_clean_buffer`: disk `BROKEN`, buffer `CLEAN`; non-empty.
- `project_errors_skip_files_outside_the_source_globs`: `lib/a.ts` broken; empty.
- `project_errors_skip_schema_files`: `schema/a.graphql` containing `` iso(` `) ``; empty.
- `project_errors_are_sorted_by_path_then_start`: `src/b.ts` and `src/a.ts` broken, `src/a.ts` with two literals on lines 0 and 2; `a.ts` line 0, `a.ts` line 2, `b.ts`.
- `project_errors_of_no_disk_files_is_empty`.
- `project_errors_after_absent_is_empty`.
- `project_errors_of_a_parent_relative_path_is_normalized`: `../other/a.ts` under globs `../other/**` reports `/tmp/other/a.ts`.
- `project_errors_without_a_working_directory_is_missing_working_directory`.
- `the_compiler_client_is_not_a_started_clients_id`: start two clients; neither assigned id equals `compile_input().client`.

`crates/isograph_cli/src/state.rs`:

- Every test that asserted `effects.is_empty()` after an applied `DiskChanged` asserts that `only_timer(effects)` succeeds instead.
- `an_applied_disk_changed_arms_the_debounce` becomes `an_applied_disk_changed_arms_the_compile_and_diagnostics_debounces`: each of `Present`, `Absent`, and `FolderRemoved` returns two `Timer`s, one whose firing matches `compile_debounce` and one whose firing matches `diagnostics_debounce`.
- `firing_the_compile_debounce_returns_no_effects_and_clears_it`.
- `a_superseded_compile_firing_returns_no_effects`: two `DiskChanged`, fire the first timer; no effects; `compile_debounce` is still `Some`.
- `a_compile_firing_does_not_clear_the_diagnostics_debounce` and the converse.
- `did_open_does_not_arm_the_compile_debounce`.
- `a_rejected_disk_changed_path_arms_nothing`: a `Present` whose path `relative_path_to_source_file` rejects returns no effects. If no absolute path is rejected on the test platform, this test is omitted and the doc updated.

## Change 4: shared LSP client handshake and `isograph start` spawning

Prefactor for change 6. No behavior changes.

### `crates/isograph_cli/src/lsp_client.rs`

The handshake of `send::notify` and `wait_for_initialize_result` move here.

```rust
// from crates/isograph_cli/src/lsp_client.rs
#[derive(Debug, thiserror::Error)]
pub(crate) enum LspClientError {
    #[error("could not write the frame: {0}")]
    Write(io::Error),
    #[error("could not read the frame: {0}")]
    Read(io::Error),
    #[error("the lsp connection closed")]
    Closed,
    #[error("{0}")]
    Lsp(String),
}

/// Sends `initialize` as request 1, waits for its result, and sends `initialized`.
pub(crate) fn initialize(
    writer: &mut impl Write,
    reader: &mut impl BufRead,
) -> Result<(), LspClientError> {
    // JSON-RPC ids are per connection. This client has one outstanding request at a time.
    let id = lsp_server::RequestId::from(1);
    lsp_server::Message::Request(lsp_server::Request {
        id: id.clone(),
        method: Initialize::METHOD.to_owned(),
        params: serde_json::json!({ "capabilities": {} }),
    })
    .write(writer)
    .map_err(LspClientError::Write)?;
    read_response(reader, id.reference())?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: Initialized::METHOD.to_owned(),
        params: serde_json::json!({}),
    })
    .write(writer)
    .map_err(LspClientError::Write)?;
    ().wrap_ok()
}

/// The result of the response to `expected`, skipping requests and notifications before it.
pub(crate) fn read_response(
    reader: &mut impl BufRead,
    expected: &lsp_server::RequestId,
) -> Result<Option<serde_json::Value>, LspClientError> {
    loop {
        let message = lsp_server::Message::read(reader).map_err(LspClientError::Read)?;
        let Some(message) = message else {
            return LspClientError::Closed.wrap_err();
        };
        let lsp_server::Message::Response(response) = message else {
            continue;
        };
        if &response.id != expected {
            return LspClientError::Lsp(format!(
                "response id {} wanted {}",
                response.id, expected
            ))
            .wrap_err();
        }
        match response.error {
            None => return response.result.wrap_ok(),
            Some(error) => return LspClientError::Lsp(error.message).wrap_err(),
        }
    }
}
```

Origin: `send::notify` (the initialize and initialized writes) and `send::wait_for_initialize_result`. Delta: generic writer and reader instead of `TcpStream`; returns the result instead of `()`; the id mismatch message drops the word `initialize`; errors are `LspClientError` instead of `SendError`.

### `send.rs`

`SendError` loses `Read`, `Closed`, and `Lsp`, and gains `Client`:

```rust
// from crates/isograph_cli/src/send.rs
    #[error("{0}")]
    Client(crate::lsp_client::LspClientError),
```

`notify` after:

```rust
// from crates/isograph_cli/src/send.rs
pub(crate) fn notify(stream: std::net::TcpStream, event: InternalEvent) -> Result<(), SendError> {
    let mut writer = stream.try_clone().map_err(SendError::Clone)?;
    let mut reader = std::io::BufReader::new(stream);
    crate::lsp_client::initialize(&mut writer, &mut reader).map_err(SendError::Client)?;
    let params = serde_json::to_value(&event).map_err(SendError::Encode)?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: IsographEventNotification::METHOD.to_owned(),
        params,
    })
    .write(&mut writer)
    .map_err(SendError::Write)?;
    ().wrap_ok()
}
```

`wait_for_initialize_result` is deleted.

### `spawn_start`

Extracted from `lsp_stdio::start_daemon`.

```rust
// from crates/isograph_cli/src/start.rs
#[derive(Debug, thiserror::Error)]
pub(crate) enum SpawnStartError {
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("could not find this executable: {0}")]
    CurrentExe(io::Error),
    #[error("could not spawn isograph start: {0}")]
    Spawn(io::Error),
}

/// Runs `isograph start` for `config` with stdout discarded and stderr inherited.
pub(crate) fn spawn_start(config: Option<&Path>) -> Result<ExitStatus, SpawnStartError> {
    let _ = crate::discover::instance_for_config_path(config)?;
    let exe = std::env::current_exe().map_err(SpawnStartError::CurrentExe)?;
    let mut command = Command::new(exe);
    command
        .arg("start")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::inherit());
    if let Some(config) = config {
        command.arg("--config").arg(config);
    }
    command.status().map_err(SpawnStartError::Spawn)
}
```

Delta from `start_daemon`: takes `Option<&Path>` instead of `&LspArgs`; errors are `SpawnStartError`. `LspError` replaces `CurrentExe` and `Spawn` with `Start(#[from] crate::start::SpawnStartError)` and keeps `Discover` for `connect_pair`. `lsp_stdio::run` calls `crate::start::spawn_start(args.id.config.as_deref())`.

### Tests

The `send` tests in `lsp_socket.rs`, `send.rs`, and `cli.rs`, and the `lsp_*` tests in `cli.rs`, pass unchanged. New in `lsp_client.rs`: `read_response_skips_a_notification_before_the_response`; `read_response_of_another_id_is_an_error`; `read_response_at_eof_is_closed`; `read_response_of_an_error_response_is_its_message`; each over an in-memory `Cursor` of written `Message`s.

## Change 5: the `isograph/errors` request

### `IsographErrorsRequest`

```rust
// from crates/isograph_cli/src/compile.rs
#[derive(Debug)]
pub enum IsographErrorsRequest {}

impl lsp_types::request::Request for IsographErrorsRequest {
    type Params = ();
    type Result = Vec<CompilerErrorReport>;
    const METHOD: &'static str = "isograph/errors";
}
```

`CompilerErrorReport` gains `serde::Serialize, serde::Deserialize`. `path` serializes as a JSON string; every path here came from an interned `&str`, so it is UTF-8.

### `errors_response`

```rust
// from crates/isograph_cli/src/compile.rs
/// Every compiler error of every source file as `client` reads it.
pub(crate) fn errors_response<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    _params: (),
) -> LSPRuntimeResult<Vec<CompilerErrorReport>> {
    project_errors(CompileInput { db, client }.reference())
        .map_err(|error| LSPRuntimeError::UnexpectedError(error.to_string()))
}
```

Same signature as the adapter handlers (`hover_response`), so it registers on the existing chain over `&IsographDb`. The connection's own `ClientId` selects the view: `source_file_paths(db, client)` and `file_contents(db, client, path)` read that client's open buffers over disk. An `isograph errors` connection never sends `didOpen`, so it reads disk.

### `dispatch_lsp_request`

Before:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::Completion>(
                crate::adapter::completion_response::<TProfile>,
            )?
            .request();
        ControlFlow::Continue(request)
    };
```

After:

```rust
// from crates/isograph_cli/src/state.rs
            .on_request_sync::<lsp_types::request::Completion>(
                crate::adapter::completion_response::<TProfile>,
            )?
            .on_request_sync::<crate::compile::IsographErrorsRequest>(
                crate::compile::errors_response::<TProfile>,
            )?
            .request();
        ControlFlow::Continue(request)
    };
```

The rest of `dispatch_lsp_request` is unchanged: `before_compute`, the one chain, and `method_not_found` for a leftover.

### Tests

`crates/isograph_cli/src/state.rs` (through `handle` of an `LspRequest` from a started client, reading the `SendLspResponse` effect):

- `errors_request_of_a_broken_disk_file_returns_its_reports`: the result deserializes to `project_errors` of the same state.
- `errors_request_of_a_clean_project_returns_an_empty_array`.
- `errors_request_of_a_client_with_no_open_buffers_reads_disk`: disk `BROKEN`; another started client has `src/a.ts` open as `CLEAN`; the requester's result is non-empty.
- `errors_request_reads_the_requesters_open_buffer`: disk `CLEAN`; the requester has `src/a.ts` open as `BROKEN`; the result lists `src/a.ts`.
- `errors_request_lists_a_validation_error`: disk `` iso(`entity Query`) iso(`field Query.x { nope }`) ``; one report whose message is ``Field `Query.nope` is not defined.``.
- `errors_request_before_the_compile_debounce_fires_sees_the_disk_change`: `DiskChanged` of a broken file, no fire, request; the result is non-empty.
- `errors_request_after_absent_returns_an_empty_array`.
- `errors_request_without_a_working_directory_is_an_error_response`: `error.code` is `UnknownErrorCode`.
- `errors_request_from_a_client_that_never_started_returns_no_effects`.
- `an_unknown_request_is_still_method_not_found`.

`crates/isograph_cli/src/lsp_socket.rs`: `errors_request_over_the_socket_returns_the_reports` with `listen_and_reply`. On one connection, initialize, write an `isograph/event` notification of a broken `DiskChanged`, then the `isograph/errors` request. One connection's messages are posted in order, so the response lists the file. No `SETTLE`.

## Change 6: the `isograph errors` verb

### CLI

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(clap::Subcommand)]
enum CliVerb<TProfile: CompilationProfile> {
    /// start, restart, status, logs, stop, and the hidden daemon.
    #[command(flatten)]
    Lifecycle(freddie_cli::Verb<Isograph<TProfile>>),

    /// Encode one IsographEvent as an LSP notification to the running daemon. Not for typing: tests and CI.
    #[command(hide = true)]
    Send(SendArgs),

    /// Print the canonical isograph config path.
    ConfigPath(ConfigFlag),

    /// Speak LSP on stdio with the daemon for this config.
    Lsp(LspArgs),

    /// Print the compiler errors of this config's files on disk. Starts the daemon if it is not running.
    Errors(ErrorsArgs),
}

#[derive(clap::Args, Debug)]
struct ErrorsArgs {
    #[command(flatten)]
    pub id: ConfigFlag,
}
```

`run` gains `Some(CliVerb::Errors(args)) => errors::run(args.reference()),` and `mod errors;`.

### `crates/isograph_cli/src/errors.rs`

```rust
// from crates/isograph_cli/src/errors.rs
const FAILED: u8 = 2;

#[derive(Debug, thiserror::Error)]
enum ErrorsCommandError {
    #[error("{0}")]
    Start(#[from] crate::start::SpawnStartError),
    #[error("isograph start exited with {0}")]
    StartFailed(ExitStatus),
    #[error("{0}")]
    Discover(#[from] DiscoverError),
    #[error("{0}")]
    Port(#[from] crate::discover::PortError),
    #[error("could not clone the stream: {0}")]
    Clone(io::Error),
    #[error("{0}")]
    Client(#[from] crate::lsp_client::LspClientError),
    #[error("could not write the request: {0}")]
    Write(io::Error),
    #[error("the daemon answered isograph/errors with no result")]
    NoResult,
    #[error("the isograph/errors result is not a list of errors: {0}")]
    Decode(serde_json::Error),
    #[error("could not read the current directory: {0}")]
    CurrentDir(io::Error),
}

#[expect(clippy::print_stdout, clippy::print_stderr)]
pub fn run(args: &ErrorsArgs) -> ExitCode {
    match fetch(args) {
        Ok((reports, cwd)) => {
            report_lines(reports.as_slice(), cwd.as_path())
                .for_each(|line| println!("{line}"));
            println!("{}", count_line(reports.len()));
            if reports.is_empty() {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(e) => {
            eprintln!("{e}");
            ExitCode::from(FAILED)
        }
    }
}

/// Starts the daemon if needed, asks it for the errors, and reads the canonical current directory.
fn fetch(args: &ErrorsArgs) -> Result<(Vec<CompilerErrorReport>, PathBuf), ErrorsCommandError> {
    let (mut writer, mut reader) = connect_initialized(args)?;
    let id = lsp_server::RequestId::from(2);
    lsp_server::Message::Request(lsp_server::Request {
        id: id.clone(),
        method: IsographErrorsRequest::METHOD.to_owned(),
        params: serde_json::Value::Null,
    })
    .write(&mut writer)
    .map_err(ErrorsCommandError::Write)?;
    let result = crate::lsp_client::read_response(&mut reader, id.reference())?
        .ok_or(ErrorsCommandError::NoResult)?;
    let reports: Vec<CompilerErrorReport> =
        serde_json::from_value(result).map_err(ErrorsCommandError::Decode)?;
    (reports, canonical_cwd()?).wrap_ok()
}

/// Runs `isograph start`, dials the daemon's port, and completes the LSP handshake.
fn connect_initialized(
    args: &ErrorsArgs,
) -> Result<(TcpStream, BufReader<TcpStream>), ErrorsCommandError> {
    let status = crate::start::spawn_start(args.id.config.as_deref())?;
    if !status.success() {
        return ErrorsCommandError::StartFailed(status).wrap_err();
    }
    let (_, instance) = crate::discover::instance_for_config_path(args.id.config.as_deref())?;
    let stream =
        crate::discover::connect_to_daemon(&crate::discover::port_file(instance.lock_file()))?;
    let mut writer = stream.try_clone().map_err(ErrorsCommandError::Clone)?;
    let mut reader = BufReader::new(stream);
    crate::lsp_client::initialize(&mut writer, &mut reader)?;
    (writer, reader).wrap_ok()
}

fn canonical_cwd() -> Result<PathBuf, ErrorsCommandError> {
    std::env::current_dir()
        .and_then(|cwd| cwd.canonicalize())
        .map_err(ErrorsCommandError::CurrentDir)
}

/// `path:line:column: message`, 1-based, `path` relative to `cwd`.
fn report_lines<'a>(
    reports: &'a [CompilerErrorReport],
    cwd: &'a Path,
) -> impl Iterator<Item = String> + 'a {
    reports.iter().map(move |report| {
        let path = pathdiff::diff_paths(report.path.as_path(), cwd)
            .unwrap_or_else(|| report.path.clone());
        format!(
            "{}:{}:{}: {}",
            path.display(),
            report.range.start.line + 1,
            report.range.start.character + 1,
            report.message
        )
    })
}

fn count_line(count: usize) -> String {
    match count {
        1 => "1 error".to_owned(),
        n => format!("{n} errors"),
    }
}
```

`spawn_start` validates the config before spawning, so a missing config exits 2 without starting anything. `isograph start` adopts a running daemon, so this does not restart one. `isograph start` returns after the port accepts connections (`start::wait_until_listening`), so `connect_initialized` does not race the port file. The current directory is canonicalized because the daemon's paths are canonical (`/private/var/...` on macOS) and `pathdiff` compares components lexically. `pathdiff::diff_paths` of two absolute paths is always `Some`; the `unwrap_or_else` covers a relative `report.path`, which `absolute_path` does not produce.

`run` is the one place that turns the outcome into an exit code: 0 for an empty list, 1 for a non-empty list, 2 for any `ErrorsCommandError`.

### Tests

`crates/isograph_cli/src/errors.rs`:

- `report_line_is_one_based`: range start `(0, 0)` prints `:1:1:`.
- `report_line_of_a_path_under_cwd_is_relative`: `/p/src/a.ts` from `/p` prints `src/a.ts`.
- `report_line_of_a_path_outside_cwd_climbs`: `/p/other/a.ts` from `/p/src` prints `../other/a.ts`.
- `report_line_of_the_cwd_itself_as_a_parent_is_the_file_name`: `/p/a.ts` from `/p` prints `a.ts`.
- `count_line_is_singular_for_one_and_plural_otherwise`: 0, 1, 2.

`crates/ts_graphql_react_isograph_cli/tests/cli.rs` (new helper `daemon_with_config(config: &str) -> Daemon` writes the config and does not start; the config is `{"source_files":["src/**"]}`):

- `errors_with_the_daemon_stopped_starts_it_and_prints_the_error`: `src/a.ts` is `` export const a = iso(` `); ``; `isograph errors` exits 1; stdout is two lines; the first starts with `src/a.ts:1:` and ends with `: Expected a declaration. An isograph literal cannot be empty.`; the second is `1 error`; `isograph status` then succeeds.
- `errors_of_a_clean_project_prints_0_errors_and_exits_0`.
- `errors_of_an_empty_project_prints_0_errors_and_exits_0`: no files.
- `errors_with_the_daemon_running_dials_it`: `Daemon::start_watch(config)`, `isograph errors` twice with equal stdout; `isograph status` stdout equals before.
- `errors_are_sorted_by_path_then_line`: `src/b.ts` and `src/a.ts` broken, `a.ts` with two broken literals on lines 1 and 3; each broken literal is `` iso(` `) ``, which has exactly one error; four lines in order `a.ts:1`, `a.ts:3`, `b.ts:1`, then `3 errors`.
- `errors_of_a_file_outside_the_source_globs_is_not_printed`.
- `errors_of_a_bad_selection_prints_the_validation_message`: config `{"source_files":["src/**"],"schema":["schema/**"]}`, `schema/schema.graphql` is `type Query { a: String }`, `src/a.ts` is `` export const x = iso(`field Query.x { nope }`); ``; exit 1; the first line starts with `src/a.ts:1:` and ends with ``: Field `Query.nope` is not defined.``.
- `errors_with_no_config_exits_2`.
- `errors_is_in_help`.

## Change 7: `isograph errors --watch`

### Notifications

```rust
// from crates/isograph_cli/src/compile.rs
/// Client to daemon: send `isograph/errorsChanged` now and after each compile that changes the errors.
#[derive(Debug)]
pub enum WatchErrorsNotification {}

impl lsp_types::notification::Notification for WatchErrorsNotification {
    type Params = ();
    const METHOD: &'static str = "isograph/watchErrors";
}

/// Daemon to client: the compiler's current errors.
#[derive(Debug)]
pub enum ErrorsChangedNotification {}

impl lsp_types::notification::Notification for ErrorsChangedNotification {
    type Params = Vec<CompilerErrorReport>;
    const METHOD: &'static str = "isograph/errorsChanged";
}
```

### `ErrorsSubscription` on `LiveSession`

```rust
// from crates/isograph_cli/src/compile.rs
/// Whether a session watches the errors, and the list it was last sent.
#[derive(Clone, Debug, Default, PartialEq)]
pub enum ErrorsSubscription {
    #[default]
    Unsubscribed,
    Subscribed(Vec<CompilerErrorReport>),
}
```

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug)]
pub struct LiveSession {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub capabilities: lsp_types::ClientCapabilities,
    pub published_diagnostics: crate::diagnostics::PublishedDiagnostics,
    pub errors_subscription: crate::compile::ErrorsSubscription,
    pub outstanding: std::collections::HashSet<lsp_server::RequestId>,
}
```

Like `PublishedDiagnostics`, it is outbound I/O state on the session. `Gone` drops it with the session. `start_session` builds it as `ErrorsSubscription::Unsubscribed`.

### `PublishErrorsEffect`

```rust
// from crates/isograph_cli/src/effect.rs
/// One `isograph/errorsChanged` notification for one session.
#[derive(Debug)]
pub struct PublishErrorsEffect {
    pub sender: crossbeam::channel::Sender<lsp_server::Message>,
    pub notification: lsp_server::Notification,
}
```

`IsographEffect` gains `PublishErrors(Box<PublishErrorsEffect>)` after `PublishDiagnostics`. `perform`:

```rust
// from crates/isograph_cli/src/daemon.rs
        IsographEffect::PublishErrors(publish) => {
            let PublishErrorsEffect {
                sender,
                notification,
            } = *publish;
            sender
                .send(lsp_server::Message::Notification(notification))
                .unwrap_or_else(|_| tracing::debug!("errorsChanged to a closed session"));
            ControlFlow::Continue(())
        }
```

### Subscribing, and notifying after compile

```rust
// from crates/isograph_cli/src/compile.rs
pub(crate) fn on_watch_errors<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
    client: ClientId,
    _params: (),
) -> Vec<IsographEffect> {
    let Ok(reports) = project_errors(
        CompileInput {
            db: state.db.reference(),
            client,
        }
        .reference(),
    )
    .map_err(|error| tracing::error!(%error, "watchErrors")) else {
        return no_effects();
    };
    let Some(session) = state.live_sessions.0.get_mut(&client) else {
        return no_effects();
    };
    send_errors(session, reports).into_iter().collect()
}

/// Computes every source file's errors and logs the count, sends each subscribed session
/// whose errors differ from its last list, then generates the artifacts.
pub(crate) fn compile<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    project_errors(state.compile_input().reference()).map_or_else(
        |error| tracing::error!(%error, "compile"),
        |reports| tracing::info!(errors = reports.len(), "compiled"),
    );
    notify_subscribers(state)
        .into_iter()
        .chain(generate_artifacts(state))
        .collect()
}

/// One `errorsChanged` per subscribed session whose errors, as that session's client reads
/// them, differ from the list it was last sent.
fn notify_subscribers<TProfile: CompilationProfile>(
    state: &mut IsographState<TProfile>,
) -> Vec<IsographEffect> {
    let db = state.db.reference();
    state
        .live_sessions
        .0
        .iter_mut()
        .filter_map(|(client, session)| notify_subscriber(db, client.dereference(), session))
        .collect()
}

fn notify_subscriber<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    session: &mut LiveSession,
) -> Option<IsographEffect> {
    let ErrorsSubscription::Subscribed(sent) = session.errors_subscription.reference() else {
        return None;
    };
    let reports = project_errors(CompileInput { db, client }.reference())
        .map_err(|error| tracing::error!(%error, "errorsChanged"))
        .ok()?;
    if sent.as_slice() == reports.as_slice() {
        return None;
    }
    send_errors(session, reports)
}

/// Records `reports` as sent and returns the notification that sends them.
fn send_errors(session: &mut LiveSession, reports: Vec<CompilerErrorReport>) -> Option<IsographEffect> {
    let params = serde_json::to_value(reports.reference())
        .map_err(|error| tracing::warn!(%error, "errorsChanged"))
        .ok()?;
    session.errors_subscription = ErrorsSubscription::Subscribed(reports);
    IsographEffect::PublishErrors(
        PublishErrorsEffect {
            sender: session.sender.clone(),
            notification: lsp_server::Notification {
                method: ErrorsChangedNotification::METHOD.to_owned(),
                params,
            },
        }
        .boxed(),
    )
    .wrap_some()
}
```

`compile` already takes `&mut IsographState`. `notify_subscribers` borrows `state.db` and `state.live_sessions` as two disjoint fields. Each subscriber's errors are computed as its own client reads them, the same view `isograph/errors` answers with, and compared with the list that session was last sent. The `--watch` connection opens no buffers, so its errors change only when the disk changes, and every applied `DiskChanged` arms the compile debounce, so the subscriber is notified of every change without polling. A subscriber that has open buffers is not notified on `didChange`, which does not arm the compile debounce; its next notification follows the next compile fire. A second `watchErrors` from a subscribed session resends the current list. The `compiled errors=N` log line stays the `compiler_client` count.

`dispatch_lsp_notification` registers `.on_notification_sync::<crate::compile::WatchErrorsNotification>(crate::compile::on_watch_errors::<TProfile>)?` after `DidCloseTextDocument`.

### CLI

```rust
// from crates/isograph_cli/src/lib.rs
#[derive(clap::Args, Debug)]
struct ErrorsArgs {
    #[command(flatten)]
    pub id: ConfigFlag,

    /// Stay connected and print the errors each time they change.
    #[arg(long)]
    pub watch: bool,
}
```

`watch` is a `bool` because a clap flag is one (Open decisions, `--watch`). `errors::run` branches on it once:

```rust
// from crates/isograph_cli/src/errors.rs
pub fn run(args: &ErrorsArgs) -> ExitCode {
    if args.watch {
        run_watch(args)
    } else {
        run_once(args)
    }
}
```

`run_once` is change 6's `run`. `run_watch` and its helpers:

```rust
// from crates/isograph_cli/src/errors.rs
/// Whether a printed block is the first of the session.
#[derive(Clone, Copy, Debug)]
enum ErrorsBlock {
    First,
    Later,
}

#[expect(clippy::print_stdout, clippy::print_stderr)]
fn run_watch(args: &ErrorsArgs) -> ExitCode {
    let error = subscribe(args).map_or_else(
        |error| error,
        |(reader, cwd)| {
            print_changes(reader, |reports, block| {
                if let ErrorsBlock::Later = block {
                    println!();
                }
                report_lines(reports, cwd.as_path()).for_each(|line| println!("{line}"));
                println!("{}", count_line(reports.len()));
            })
        },
    );
    eprintln!("{error}");
    ExitCode::from(FAILED)
}

/// Starts the daemon if needed, sends `isograph/watchErrors`, and reads the canonical current directory.
fn subscribe(args: &ErrorsArgs) -> Result<(BufReader<TcpStream>, PathBuf), ErrorsCommandError> {
    let (mut writer, reader) = connect_initialized(args)?;
    lsp_server::Message::Notification(lsp_server::Notification {
        method: WatchErrorsNotification::METHOD.to_owned(),
        params: serde_json::Value::Null,
    })
    .write(&mut writer)
    .map_err(ErrorsCommandError::Write)?;
    (reader, canonical_cwd()?).wrap_ok()
}

/// Calls `print` for each `isograph/errorsChanged` until reading fails, and returns that failure.
fn print_changes(
    mut reader: BufReader<TcpStream>,
    mut print: impl FnMut(&[CompilerErrorReport], ErrorsBlock),
) -> ErrorsCommandError {
    let mut block = ErrorsBlock::First;
    loop {
        match read_errors_changed(&mut reader) {
            Ok(reports) => {
                print(reports.as_slice(), block);
                block = ErrorsBlock::Later;
            }
            Err(error) => return error,
        }
    }
}

/// The params of the next `isograph/errorsChanged`, skipping every other message.
fn read_errors_changed(
    reader: &mut impl BufRead,
) -> Result<Vec<CompilerErrorReport>, ErrorsCommandError> {
    loop {
        let message = lsp_server::Message::read(reader)
            .map_err(|e| ErrorsCommandError::Client(crate::lsp_client::LspClientError::Read(e)))?
            .ok_or(ErrorsCommandError::Closed)?;
        let lsp_server::Message::Notification(notification) = message else {
            continue;
        };
        if notification.method != ErrorsChangedNotification::METHOD {
            continue;
        }
        return serde_json::from_value(notification.params).map_err(ErrorsCommandError::Decode);
    }
}
```

`ErrorsCommandError` gains `#[error("the daemon closed the connection")] Closed`. The process ends on Ctrl-C (default SIGINT) or when the daemon closes the connection. The loop is a state machine over a blocking socket read; each iteration blocks in `Message::read` until the daemon writes. Nothing polls.

`print_changes` returns `ErrorsCommandError`, not a `Result`: it ends only when reading fails, so its return type is that failure.

### Tests

`crates/isograph_cli/src/state.rs` (through `handle`):

- `watch_errors_sends_the_current_list_once`: one `PublishErrors` effect whose params deserialize to `project_errors`; the session is `Subscribed` with that list.
- `watch_errors_of_a_clean_project_sends_an_empty_list`.
- `a_compile_that_changes_the_errors_notifies_each_subscriber`: two subscribed sessions and one unsubscribed; `DiskChanged` of a broken file, fire; two effects, to the two subscribers' senders.
- `a_compile_with_equal_errors_notifies_nobody`: subscribed, then a `DiskChanged` of an unrelated clean file, fire; no effects.
- `a_compile_with_no_subscribers_returns_no_effects`: no `PublishErrors` effect; with no `output_directory` the fire returns no effects.
- `a_subscriber_is_notified_of_its_own_view`: a subscribed session has `src/a.ts` open as `BROKEN` while disk is `CLEAN`; the list it is sent at subscribe time is non-empty, while `project_errors` of `compile_input()` is empty.
- `a_second_watch_errors_resends_the_current_list`.
- `gone_drops_the_subscription`: subscribe, `Gone`, `DiskChanged`, fire; no effects.
- `watch_errors_from_a_client_that_never_started_returns_no_effects`.

`crates/isograph_cli/src/errors.rs`: `read_errors_changed_skips_other_messages`; `read_errors_changed_at_eof_is_closed`; both over a `Cursor`.

`crates/ts_graphql_react_isograph_cli/tests/cli.rs` (new `Daemon::start_injected_with(config: &str)`, config `{"source_files":["src/**"]}`):

- `errors_watch_prints_each_change`: spawn `isograph errors --watch` with piped stdout; read lines until `0 errors`; `send_json` a `present` of `src/a.ts` with `` iso(` `) ``; read until the next count line: a blank line, one `src/a.ts:1:` line, `1 error`; `send_json` an `absent` of it; read: a blank line, `0 errors`. Reads block on the child's stdout; Rust's stdout is line-buffered, so each `println!` arrives. Kill the child at the end.
- `errors_watch_exits_2_when_the_daemon_stops`: spawn, read until `0 errors`, `isograph stop`, the child exits with 2 and stderr contains `the daemon closed the connection`.

## Change 8: validation errors are compiler errors (landed)

`iso_literal_validation_errors_in_file` (`crates/isograph_compiler/src/validation.rs`) returns file-absolute spans, like `iso_literal_parse_errors_in_file`:

```rust
// from crates/isograph_compiler/src/validation.rs
#[memo]
pub fn iso_literal_validation_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ValidationError>>>
```

Before:

```rust
// from crates/isograph_compiler/src/compiler_errors.rs
/// An error the compiler reports for a source file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompilerError {
    #[error("{0}")]
    Parse(ParseError),
}

/// Every compiler error of `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn compiler_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<CompilerError>>> {
    let parse_errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    parse_errors
        .iter()
        .map(|error| error.clone().map(CompilerError::Parse))
        .collect::<Vec<_>>()
        .wrap_some()
}
```

After:

```rust
// from crates/isograph_compiler/src/compiler_errors.rs
/// An error the compiler reports for a source file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CompilerError {
    #[error("{0}")]
    Parse(ParseError),
    #[error("{0}")]
    Validation(ValidationError),
}

/// Every compiler error of `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn compiler_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<CompilerError>>> {
    let parse_errors = iso_literal_parse_errors_in_file(db, client, path).as_ref()?;
    let validation_errors = iso_literal_validation_errors_in_file(db, client, path).as_ref()?;
    parse_errors
        .iter()
        .map(|error| error.clone().map(CompilerError::Parse))
        .chain(
            validation_errors
                .iter()
                .map(|error| error.clone().map(CompilerError::Validation)),
        )
        .collect::<Vec<_>>()
        .wrap_some()
}
```

The parse errors come first, then the validation errors, the same order as `lsp_diagnostics_for_file`. `project_errors` sorts by path and start and its sort is stable, so the editor and the compile list the same messages at the same ranges. The compile step's `errors=N` count includes validation errors, and so will `isograph errors` (changes 5 and 6).

Validation reads schema files through `entity_named` / `selectable_named`, which read `file_contents` of each path `schema_file_paths` lists. A schema file change is a `DiskChanged`, which arms the compile debounce, so the compiler's validation errors follow schema edits. `compiler_errors_in_file` of a source file depends on the schema reads validation made, so it re-invokes when they change.

### Tests

`crates/isograph_compiler/src/compiler_errors.rs`:

- `validation_errors_are_compiler_errors_with_the_same_spans`: for `` iso(`entity Query`) iso(`field Query.x { nope }`) ``, the result equals `iso_literal_validation_errors_in_file` mapped through `CompilerError::Validation`.
- `parse_errors_precede_validation_errors`: `` iso(`entrypoint Query.Missing`) iso(` `) `` is `[Parse(_), Validation(_)]`.
- `a_file_whose_declarations_resolve_has_no_errors`: `` iso(`entity Query`) iso(`field Query.x`) iso(`entrypoint Query.x`) `` is `Some(vec![])`.

`crates/isograph_cli/src/compile.rs`:

- `project_errors_include_a_validation_error_with_its_absolute_path_range_and_message`: one report, `/tmp/proj/src/a.ts`, the range of `nope`, ``Field `Query.nope` is not defined.``.
- `project_errors_follow_a_schema_change`: `schema/schema.graphql` is `type Query { a: String }` and `src/a.ts` selects `Query.nope`; one report; a `Present` of the schema that adds `nope` makes `project_errors` empty.
- `project_errors_of_a_clean_pet_demo_like_project_is_empty`: a schema with `Query.pets: [Pet!]!` and `Pet { id firstName lastName best_friend }`, `src/Pet.ts` with `field Pet.fullName` and `field Pet.bestFriend to Pet`, and `src/HomeRoute.ts` with `field Query.HomeRoute @component` selecting `pets { id fullName bestFriend { firstName } }` and `entrypoint Query.HomeRoute`; empty.

Change 6 adds the e2e `errors_of_a_bad_selection_prints_the_validation_message`.

## Change 9: schema parse errors are compiler errors

Ships after refactors/past/graphql-schema-parser/integration.md change 2 (`parsed_schema_file`). `CompilerError` gains `SchemaParse(<the schema parser's error type>)`. `project_errors` also iterates `schema_file_paths(db, input.client)` (`schema_path_map`, the client's view) and skips a path that `source_file_paths(db, input.client)` also lists (a path in both is reported once, through `compiler_errors_in_file`), reading `parsed_schema_file(db, input.client, path)` errors through a new memo `schema_file_errors(db, client, path) -> Option<Vec<WithSpan<CompilerError>>>`. Classification stays in `handle`: neither listing re-reads the globs. The exact types are written when integration change 2 lands; this section is updated then. Same command, same output format.

## Panics

No new panic. `ProjectErrorsError::ListedPathWithoutText` is the one broken invariant this doc adds (a path `source_file_paths(db, client)` listed has no text for `client`), and it is an error value that `isograph/errors` returns as `LSPRuntimeError::UnexpectedError` and `compile` logs.

## Shared state

No new `Arc`, `Mutex`, atomic, `static`, or `thread_local!`. The `ClientId` counter moves off `accept_loop` onto `IsographState`. The session thread receives its id through a `crossbeam::channel::bounded(1)` whose sender travels in the event and the effect.

## Wake on events

The session thread blocks on `recv` of its id. `isograph errors` blocks on the socket read of its response. `isograph errors --watch` blocks on the socket read of the next notification. The daemon notifies a watcher when a compile fires, and a compile fires only after a `DiskChanged`. `isograph errors` on a stopped daemon runs `isograph start`, whose `wait_until_listening` (`crates/isograph_cli/src/start.rs`) sleeps 10ms between connect attempts. That loop predates this doc and is not changed here.

## Known risks

- `isograph/errors` and the compile fire run in `handle`, on the event loop. A cold compile of a large project blocks every other event for its duration. After boot the compile debounce warms the memos, so later requests are mostly memo hits.
- The compiler client doubles the derived nodes of extraction per disk file (one set for the compiler, one per LSP client that reads disk). `parsed_iso_literal` is keyed by text and is shared. i2 never runs pico garbage collection, so these live for the process.
- Every `isograph errors` run is a new connection with a new `ClientId`, and the per-file memos (`file_contents`, extraction, `compiler_errors_in_file`, `line_index`) are keyed by client. The first request of each run re-derives those nodes for every source file, even though the text is the disk text `compiler_client` already read; only `parsed_iso_literal` (keyed by text) is shared. Each run's nodes stay after it disconnects. This is the cost of answering with the requester's view.
- A notify batch that arrives during the boot scan is applied after the boot batch; this is today's order too.

## Open decisions

Each has a held position. Implementation follows the held position unless the user picks otherwise.

1. Output format beyond `path:line:column: message`. Held: plain lines plus a final count line (`1 error`, `N errors`); no colors, no severity column, no summary other than the count. `--watch` prints a blank line between blocks and does not clear the screen.
2. JSON output (`--json`). Held: later. The response is already `Vec<CompilerErrorReport>` as JSON, so `--json` would print it verbatim.
3. Whether the errors include schema files' parse errors. Held: yes, once schema integration change 2 lands, through the same command (change 9).
4. Exit code when the errors cannot be obtained. Held: 2, distinct from 1 (errors exist). clap's own usage errors also exit 2.
5. Column unit. Held: UTF-16 code units plus one, the LSP column. Alternatives: Unicode scalar values plus one (rustc's convention), or bytes plus one (gcc's). They differ only on lines with non-ASCII text before the error.
6. `--watch` is `watch: bool` on `ErrorsArgs` (AGENTS.md Booleans). The two cases are print once and exit, and print on every change until disconnected. Held: `bool`, because clap derives a flag from a `bool` field (the same as `LspArgs.stdio`), and `errors::run` branches on it exactly once. The alternative is an `ErrorsMode` value enum, which would make the CLI `--mode watch`.

## Docs this doc amends

docs-website/docs/design-docs/event-model.md, in the change that makes each statement true:

- Change 1, Outer, Watcher bullet: append "The boot scan's `DiskChanged`s are posted before the LSP port accepts its first connection, so every session's events follow them."
- Change 2, Inner: "`ClientConnectionEvent::Started` carries the session and a channel; `handle` mints the `ClientId` from `IsographState.client_ids`, stores the session under it, and returns `AssignClientId`. The session forwards nothing until it receives the id." Effect: the `IsographEffect` listing gains `AssignClientId(AssignClientIdEffect)`.
- Change 3, Inner: the `IsographState` field list gains `compile_debounce`, `client_ids`, and `compiler_client`; add "An applied `DiskChanged` arms the compile debounce. `handle` of that `TimerFired` computes every source file's errors as `compiler_client` reads them." State: replace "Artifact generation will read through `file_contents` as its own client with no `OpenFile`; how it gets its `ClientId` is not designed yet." with "The compiler reads through `file_contents` as its own client, `compiler_client`, which `IsographState::new` mints first. No `LiveSession` and no `OpenFile` has it, so it reads disk. Artifact generation will be an effect `handle` returns after compile."
- Change 5, Outer, LSP port bullet: add "`isograph/errors` is a request arm. Its result is every compiler error of every source file as the requesting client reads it; a connection with no open buffers reads disk."
- Change 6, Outer: add "`isograph errors` starts the daemon if needed, sends `isograph/errors`, prints the result, and exits 0, 1, or 2."
- Change 7, Effect: the listing gains `PublishErrors(Box<PublishErrorsEffect>)`. Outer, LSP port bullet: add "`isograph/watchErrors` subscribes a session; after each compile fire, `handle` computes each subscriber's errors as its client reads them and returns `isograph/errorsChanged` to each subscriber whose errors differ from what it was last sent."

docs-website/docs/design-docs/pico.md:

- Change 2, The client is an input: "The session loop in `lsp_socket.rs` tags each incoming message with its connection's `ClientId`" becomes "... with the `ClientId` `handle` assigned to its connection when it processed that connection's `Started`".
- Change 3, Source: "The file-based compiler will be another client with its own `ClientId`." becomes "The compiler is another client, `IsographState.compiler_client`."
