# Tests await a wake

Requires send-events.md (landed). Independent of lsp-tokens.md. `isograph send` does not poll; that has landed.

A poll is a loop that retries a predicate until it holds or a deadline fires (`cli.rs` `poll`, the `watch_rx.try_recv` loop, `lsp_socket.rs` `sleep(SETTLE)` then `try_recv`). Tests do not do that. The consumer parks on the thing that will wake it.

Origin of the wake: figaro `src/daemon.rs` `event_rx.recv().await`, and `AGENTS.md` "Wake on events." Origin of the follow cursor: freddie `crates/freddie_cli/src/client.rs` `follow`. Origin of waiting for a lock release: freddie `watch_for_free` / `await_free_at`. Origin of the log wake: `notify` on the log file. Delta: isograph parks; it does not copy freddie's idle sleep at EOF. Figaro `tests/external.rs` sleeps `SETTLE` then `try_recv`; `lsp_socket.rs` copied that. This slice deletes it.

`isograph send` still writes `isograph/event` and exits. The wait is the test, parked on a log line, a channel, or a lock.

One shippable change.

## What the user does

Same verbs. `cargo test` still drives the binary. A test that used to spin on `log_text()` now blocks until the record, the event, or the lock arrives.

## Types

Most important first.

### In-process: the channel

`lsp_socket.rs` tests are already `#[tokio::test]`. After `notify`, `event_rx.recv().await`. Delete `const SETTLE` and every `tokio::time::sleep`. The listener is bound before `accept_loop` is spawned, so the kernel queues `connect`.

```rust
// from crates/isograph_cli/src/lsp_socket.rs
    #[tokio::test(flavor = "multi_thread")]
    async fn notify_hello_world_arrives_as_an_event() {
        let (port, mut event_rx) = listen_for_events().await;
        notify(connect(port), Internal::HelloWorld).expect("notify returns");
        assert!(matches!(
            posted_internal(event_rx.recv().await.expect("an event arrived")),
            Internal::HelloWorld
        ));
    }
```

Every `sleep(SETTLE)` then `try_recv().expect("an event arrived")` becomes `recv().await.expect("an event arrived")`. Same for `listen_and_reply` tests that then wait for HelloWorld. Delete `const SETTLE` and every `tokio::time::sleep`.

Absence has no edge. Send HelloWorld after the dropped frame, `recv().await`, assert that event is HelloWorld. If the dropped frame had posted, it would have arrived first.

`watch.rs` `notify_interns_a_file_written_after_start` is sync. `UnboundedReceiver::blocking_recv` parks on the same channel the notify thread sends on.

```rust
// from crates/isograph_cli/src/watch.rs
        let _ = watch_rx.try_recv();
        h.write("src/a.in", "a");
        let events = watch_rx
            .blocking_recv()
            .expect("notify delivered src/a.in");
        h.ingest(events);
```

Delete the 10s `try_recv` loop. `boot_interns_a_file_written_before_start` stays `try_recv` immediately: boot posts before `start` returns.

### CLI: the log file

A test that needs a daemon fact already in the log follows the file until that line. Origin of the follow cursor: freddie `follow`. Origin of the wake: `notify` (already a workspace dep; isograph_cli uses `notify-debouncer-full` for source files). Delta: stop when the line contains the needle; do not print; park on the watcher, never `sleep`. The follow is a cursor on `Daemon`, opened once after the start verb, so a second `disk present` cannot match the first. Capture the position before send; a seek-to-end after send can miss the line.

```toml
# from crates/ts_graphql_react_isograph_cli/Cargo.toml
notify = { workspace = true }
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
struct Daemon {
    dir: tempfile::TempDir,
    log: LogFollow,
}

struct LogFollow {
    reader: std::io::BufReader<std::fs::File>,
    line: String,
    rx: std::sync::mpsc::Receiver<Result<notify::Event, notify::Error>>,
    _watcher: notify::RecommendedWatcher,
}

impl LogFollow {
    fn open(path: &std::path::Path) -> Self {
        let parent = path.parent().expect("a log path has a parent");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut watcher =
            notify::recommended_watcher(tx).expect("a test can watch the log directory");
        watcher
            .watch(parent, notify::RecursiveMode::NonRecursive)
            .expect("a test can watch the log directory");
        let file = loop {
            match std::fs::File::open(path) {
                Ok(file) => break file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    let _ = rx.recv().expect("the log directory is watched");
                }
                Err(error) => panic!("opening the log: {error}"),
            }
        };
        watcher
            .watch(path, notify::RecursiveMode::NonRecursive)
            .expect("a test can watch the log file");
        Self {
            reader: std::io::BufReader::new(file),
            line: String::new(),
            rx,
            _watcher: watcher,
        }
    }

    fn await_containing(&mut self, needle: &str) {
        loop {
            match self.reader.read_line(&mut self.line) {
                Ok(0) => {
                    let _ = self.rx.recv().expect("the log file is watched");
                }
                Ok(_) if self.line.ends_with('\n') => {
                    let hit = self.line.contains(needle);
                    self.line.clear();
                    if hit {
                        return;
                    }
                }
                Ok(_) => {
                    let _ = self.rx.recv().expect("the log file is watched");
                }
                Err(error) => panic!("reading the log: {error}"),
            }
        }
    }
}
```

`log_path` is the one `.log` under the private HOME that `log_text` already finds. Extract it; `log_text` still concatenates for assertions that read the whole file. Directory events for other files are spurious wakes: `read_line` returns 0 and the next `recv` parks again.

Delete `poll`. Delete `settle`. Delete `DEADLINE`. Delete `IDLE`.

`Daemon::start` / `start_watch` open `LogFollow` after the start verb succeeds, then `log.await_containing("isograph daemon up")`. `isograph start` returning still means the lock is held, not that listen has run. Tests that wait on a later line call `daemon.log.await_containing(...)`. `await_containing` needs `&mut Daemon`; tests that today pass `daemon.reference()` into send helpers stay `&Daemon` for send.

### Intern is a log line

`handle_disk_changed` logs after it interns or removes, so send and the watcher share one wake. Origin: `watch.rs` `debug!(path = %path.display(), "disk present")` on the watcher apply path. Delta: the same record after `insert_disk_file` / `remove_disk_file` / `remove_disk_files_from_path`. Delete the watcher's copy; a line emitted before `handle` is not intern.

```rust
// from crates/isograph_cli/src/state.rs
        DiskChanged::File(change) => {
            let path = relative_path_to_source_file(state, &change.path);
            match change.presence {
                Presence::Present(contents) => {
                    state.insert_disk_file(path, contents);
                    tracing::debug!(path = %change.path.display(), "disk present");
                }
                Presence::Absent => {
                    state.remove_disk_file(path);
                    tracing::debug!(path = %change.path.display(), "disk absent");
                }
            }
        }
        DiskChanged::FolderRemoved(folder) => {
            let path = relative_path_to_source_file(state, &folder.path);
            state.remove_disk_files_from_path(path);
            tracing::debug!(path = %folder.path.display(), "disk folder removed");
        }
```

The file log already includes `debug` (watch e2e already matches `disk present`).

## Tests

### `cli.rs`

Every `poll(|| daemon.log_text().contains("isograph daemon up").then_some(()))` goes away. `start` / `start_watch` already awaited that line. Delete `settle()` from `start` / `start_watch` and from the e2e token tests.

After send HelloWorld: `daemon.log.await_containing("hello world")`. Same in `the_log_contains_the_config_path`, `stop_after_send_exits_without_force`, `two_sends_then_stop_exits_without_force`, `truncated_body_does_not_kill_the_daemon`.

After `STOP`: `daemon.log.await_containing("kill: exiting")`, then `assert!(!daemon.isograph(["status"].reference()).status.success())`. Unix also asserts `SIGTERM: quitting` in `log_text()`. Same in `two_sends_then_stop_exits_without_force` and `stop_then_status_reports_not_running`.

`watch_of_empty_source_files_logs_scan_finished`: `start_watch` awaited daemon up. `daemon.log.await_containing("scan finished")`.

`watch_interns_matching_files_and_skips_the_rest`: after start, `daemon.log.await_containing("Home.ts")` (the intern log names the absolute path). Write `Other.ts`, `daemon.log.await_containing("Other.ts")`. Write `skip.rs` and `Home.test.ts`. Write `src/Keep.ts` with an iso literal (matches the glob). `await_containing("Keep.ts")`. Then `log_text()` does not contain `skip.rs` or `Home.test.ts`. If those had interned, their lines would have arrived before `Keep.ts`. Delete `Duration::from_millis(500)`.

`send_of_disk_changed_present_then_absent_exits_0`, `send_of_folder_removed_exits_0`: after present send, `daemon.log.await_containing("disk present")`. After absent / folder removed, `await_containing` `disk absent` / `disk folder removed`. Those tests today only assert send exits 0; they grow the await so the intern is the fact, not the client return.

`send_of_a_present_iso_literal_returns_entrypoint_as_keyword` and the other token tests: after Present send, `daemon.log.await_containing("disk present")`, then one `semantic_tokens_full`. After Absent, `await_containing("disk absent")`, then `full` is JSON `null`. Prefix: send, await `disk present`, read; send prefix, await `disk present`, read; assert `delta_line == 1`.

`send_of_not_json_fails`, `send_of_unknown_kind_fails`: only the daemon-up wait moves into `start`.

lsp-proxy.md tests already park: `Message::read` and `child.wait()`. No change beyond not calling `poll`. The production port-file wait watches the port file (or its directory, for create) with the OS watcher, then reads. It does not poll and does not `sleep`.

## Call sites

- `run_event_loop` -> `event_rx.recv().await` (already)
- `lsp_socket.rs` tests -> `event_rx.recv().await`
- `watch.rs` notify test -> `watch_rx.blocking_recv()`
- `Daemon::start` / `start_watch` -> `await_log_containing` `isograph daemon up`
- send HelloWorld -> `await_log_containing` `hello world`
- `STOP` -> `await_log_containing` `kill: exiting`
- `handle_disk_changed` -> `disk present` / `disk absent` / `disk folder removed` -> `await_log_containing`
- e2e token tests send Present -> `daemon.log.await_containing("disk present")` -> `semantic_tokens_full`
