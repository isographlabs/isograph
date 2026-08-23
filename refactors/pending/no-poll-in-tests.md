# Tests await a wake

Requires send-events.md (landed). Independent of lsp-tokens.md. `isograph send` does not poll; that has landed.

A poll is a loop that retries a predicate until it holds or a deadline fires (`cli.rs` `poll`, the `watch_rx.try_recv` loop, `lsp_socket.rs` `sleep(SETTLE)` then `try_recv`). Tests do not do that. The consumer parks on the thing that will wake it.

Origin of the wake: figaro `src/daemon.rs` `event_rx.recv().await`, and freddie `AGENTS.md` "No polling; wake on events." Origin of following a log file: freddie `crates/freddie_cli/src/client.rs` `follow`. Origin of waiting for a lock release: freddie `watch_for_free` / `await_free_at`. Delta: isograph tests park the same way. Figaro `tests/external.rs` sleeps `SETTLE` then `try_recv`; `lsp_socket.rs` copied that. This slice deletes it.

`isograph send` still writes `isograph/event` and exits. The wait is the test, parked on a log line, a channel, or a lock.

One shippable change.

## What the user does

Same verbs. `cargo test` still drives the binary. A test that used to spin on `log_text()` or sleep 250ms now blocks until the record, the event, or the lock arrives.

## Types

Most important first.

### In-process: the channel

`lsp_socket.rs` tests are already `#[tokio::test]`. After `notify`, `event_rx.recv().await`. Delete `const SETTLE` except the absence case below. Delete `tokio::time::sleep(SETTLE)` after bind: the listener is bound before `accept_loop` is spawned, so the kernel queues `connect`.

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

Every `sleep(SETTLE)` then `try_recv().expect("an event arrived")` becomes `recv().await.expect("an event arrived")`. Same for `listen_and_reply` tests that then wait for HelloWorld.

Absence of an event has no edge. `timeout` then `recv` is the last resort freddie names, justified only there:

```rust
// from crates/isograph_cli/src/lsp_socket.rs
    let arrived = tokio::time::timeout(Duration::from_millis(250), event_rx.recv()).await;
    assert!(arrived.is_err(), "nothing was dispatched");
```

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

A test that needs a daemon fact already in the log follows the file until that line. Origin: freddie `follow`. Delta: stop when the line contains the needle; do not print. The follow is a cursor on `Daemon`, opened once after the start verb, so a second `disk present` cannot match the first. Capture the position before send; a seek-to-end after send can miss the line.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
struct Daemon {
    dir: tempfile::TempDir,
    log: LogFollow,
}

struct LogFollow {
    reader: std::io::BufReader<std::fs::File>,
    line: String,
}

/// How long to wait before looking for more, once a read has come up empty.
///
/// A poll, and the exception the "never poll" rule allows: no platform reports a regular file
/// growing through a readiness primitive. `epoll` and `kqueue` both call a regular file always
/// ready and return zero bytes, and `tail -F` polls for the same reason.
const IDLE: Duration = Duration::from_millis(200);

const DEADLINE: Duration = Duration::from_secs(10);

impl LogFollow {
    fn open(path: &std::path::Path) -> Self {
        let deadline = std::time::Instant::now() + DEADLINE;
        let file = loop {
            match std::fs::File::open(path) {
                Ok(file) => break file,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                    assert!(std::time::Instant::now() < deadline, "deadline passed");
                    std::thread::sleep(IDLE);
                }
                Err(error) => panic!("opening the log: {error}"),
            }
        };
        Self {
            reader: std::io::BufReader::new(file),
            line: String::new(),
        }
    }

    fn await_containing(&mut self, needle: &str) {
        let deadline = std::time::Instant::now() + DEADLINE;
        loop {
            match self.reader.read_line(&mut self.line) {
                Ok(0) => {
                    assert!(std::time::Instant::now() < deadline, "deadline passed");
                    std::thread::sleep(IDLE);
                }
                Ok(_) if self.line.ends_with('\n') => {
                    let hit = self.line.contains(needle);
                    self.line.clear();
                    if hit {
                        return;
                    }
                }
                Ok(_) => {
                    assert!(std::time::Instant::now() < deadline, "deadline passed");
                    std::thread::sleep(IDLE);
                }
                Err(error) => panic!("reading the log: {error}"),
            }
        }
    }
}
```

`log_path` is the one `.log` under the private HOME that `log_text` already finds. Extract it; `log_text` still concatenates for assertions that read the whole file.

Delete `poll`. Delete `settle`.

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

`watch_interns_matching_files_and_skips_the_rest`: after start, `daemon.log.await_containing("Home.ts")` (the intern log names the absolute path). Write `Other.ts`, `daemon.log.await_containing("Other.ts")`. Write `skip.rs` and `Home.test.ts`. Absence has no edge: `timeout` 250ms, then `log_text()` does not contain those paths. Delete `Duration::from_millis(500)`.

`send_of_disk_changed_present_then_absent_exits_0`, `send_of_folder_removed_exits_0`: after present send, `daemon.log.await_containing("disk present")`. After absent / folder removed, `await_containing` `disk absent` / `disk folder removed`. Those tests today only assert send exits 0; they grow the await so the intern is the fact, not the client return.

`send_of_a_present_iso_literal_returns_entrypoint_as_keyword` and the other token tests: after Present send, `daemon.log.await_containing("disk present")`, then one `semantic_tokens_full`. After Absent, `await_containing("disk absent")`, then `full` is JSON `null`. Prefix: send, await `disk present`, read; send prefix, await `disk present`, read; assert `delta_line == 1`.

`send_of_not_json_fails`, `send_of_unknown_kind_fails`: only the daemon-up wait moves into `start`.

lsp-proxy.md tests already park: `Message::read` and `child.wait()`. No change beyond not calling `poll`. The production port-file loop in `lsp_stdio.rs` is unchanged.

## Call sites

- `run_event_loop` -> `event_rx.recv().await` (already)
- `lsp_socket.rs` tests -> `event_rx.recv().await`
- `watch.rs` notify test -> `watch_rx.blocking_recv()`
- `Daemon::start` / `start_watch` -> `await_log_containing` `isograph daemon up`
- send HelloWorld -> `await_log_containing` `hello world`
- `STOP` -> `await_log_containing` `kill: exiting`
- `handle_disk_changed` -> `disk present` / `disk absent` / `disk folder removed` -> `await_log_containing`
- e2e token tests send Present -> `daemon.log.await_containing("disk present")` -> `semantic_tokens_full`
