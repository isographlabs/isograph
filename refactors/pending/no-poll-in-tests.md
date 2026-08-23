# Tests do not poll

Requires send-events.md (landed). Independent of lsp-tokens.md. `isograph send` does not poll; that has landed. This slice is the tests.

A poll is a loop that retries a predicate until it holds or a deadline fires (`poll` in `cli.rs`, the `watch_rx.try_recv` loop in `watch.rs`). Tests do not do that. After a fact that is not yet visible, sleep `SETTLE` once, then assert. Origin of `SETTLE`: `crates/isograph_cli/src/lsp_socket.rs` `const SETTLE: Duration = Duration::from_millis(250)`. Delta: the same constant in `cli.rs` and in `watch.rs` tests; `std::thread::sleep`, not tokio.

`lsp_socket.rs` already sleeps `SETTLE` then asserts. No change there.

Production code is unchanged. Send still fails immediately on a missing port. The proxy's port-file wait is lsp-proxy.md and is not a test.

One shippable change.

## What the user does

Same verbs. CI `cargo test` still drives the binary. A test that used to spin for up to 10s now sleeps 250ms once and asserts.

## Types

Most important first.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
const SETTLE: Duration = Duration::from_millis(250);

fn settle() {
    std::thread::sleep(SETTLE);
}
```

Delete `poll`. Delete `DEADLINE`. Drop `use std::time::Instant`.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
        assert!(text.contains("started"), "{text}");
        assert!(text.contains(&path.display().to_string()), "{text}");
        settle();
        assert!(
            daemon.log_text().contains("isograph daemon up"),
            "{}",
            daemon.log_text()
        );
        daemon
```

Same two lines at the end of `start_watch`, after the start verb succeeds. `isograph start` returning still means the lock is held, not that listen has run. The wait is `settle` in the harness, not send.

## Tests

### `cli.rs`

Every `poll(|| daemon.log_text().contains("isograph daemon up").then_some(()))` goes away. `Daemon::start` / `start_watch` already settled.

`the_log_contains_the_config_path` after send:

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
    let sent = daemon.isograph(["send", "--file", frame.to_str().expect("utf-8")].reference());
    assert!(
        sent.status.success(),
        "stdout: {} stderr: {}",
        stdout(sent.reference()),
        stderr(sent.reference())
    );
    settle();
    assert!(
        daemon.log_text().contains("hello world"),
        "{}",
        daemon.log_text()
    );
    assert!(!frame.exists(), "send deletes --file");
```

Same after-send `hello world` assert in `stop_after_send_exits_without_force`, `two_sends_then_stop_exits_without_force`, `truncated_body_does_not_kill_the_daemon`.

After `STOP`:

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
    let stopped = daemon.isograph(STOP);
    assert!(
        stopped.status.success(),
        "stdout: {} stderr: {}",
        stdout(stopped.reference()),
        stderr(stopped.reference())
    );
    settle();
    assert!(!daemon.isograph(["status"].reference()).status.success());
    #[cfg(not(windows))]
    {
        let log = daemon.log_text();
        assert!(log.contains("SIGTERM: quitting"), "{log}");
        assert!(log.contains("kill: exiting"), "{log}");
    }
```

Same in `two_sends_then_stop_exits_without_force` and `stop_then_status_reports_not_running`.

`watch_of_empty_source_files_logs_scan_finished`: `start_watch` settled. Assert both strings once.

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs
fn watch_of_empty_source_files_logs_scan_finished() {
    let daemon = Daemon::start_watch("{\"source_files\":[]}\n");
    let log = daemon.log_text();
    assert!(log.contains("scan finished"), "{log}");
    assert!(log.contains("isograph daemon up"), "{log}");
}
```

`watch_interns_matching_files_and_skips_the_rest`: after the inline start, `settle()` then assert `disk present` and `Home.ts`. Write `Other.ts`, `settle()`, assert `Other.ts`. Write `skip.rs` and `Home.test.ts`, `settle()`, assert those paths are absent. Delete `Duration::from_millis(500)`.

`send_of_disk_changed_present_then_absent_exits_0`, `send_of_folder_removed_exits_0`, `send_of_not_json_fails`, `send_of_unknown_kind_fails`: only the daemon-up poll goes away.

### `watch.rs`

`notify_interns_a_file_written_after_start`. Origin of `SETTLE`: `lsp_socket.rs`. Delta: this module.

```rust
// from crates/isograph_cli/src/watch.rs
        let _ = watch_rx.try_recv();
        h.write("src/a.in", "a");
        std::thread::sleep(SETTLE);
        let events = watch_rx
            .try_recv()
            .expect("notify delivered src/a.in");
        h.ingest(events);
```

`SETTLE` is `Duration::from_millis(250)` in the `watch.rs` test module. Delete the 10s deadline loop. `boot_interns_a_file_written_before_start` stays `try_recv` immediately: boot posts before `start` returns.

### Pending docs this rule applies to

e2e-send-semantic-tokens.md: `Daemon::start` settled. After `isograph send`, `settle()` then one `semantic_tokens_full`. No `wait_until_up`. No `poll` of the tokens request. Prefix: send, settle, read; send prefix, settle, read; assert `delta_line == 1`. Absent: send present, settle, read `Some`; send absent, settle, read `None`.

lsp-proxy.md tests: `Message::read` stays on a thread. The parent waits with `rx.recv_timeout(Duration::from_secs(10))`, not `poll(|| rx.try_recv().ok())`. Nested start can exceed `SETTLE`. After drop stdin, `child.wait()`, not `poll` of `try_wait`. The production port-file loop in `lsp_stdio.rs` is unchanged.

## Call sites

- `Daemon::start` / `start_watch` -> `settle` -> log has `isograph daemon up`
- send HelloWorld -> `settle` -> log has `hello world`
- `STOP` -> `settle` -> `status` is not running
- write a watched file -> `settle` -> log / `watch_rx`
- e2e-send-semantic-tokens.md send DiskChanged -> `settle` -> `semantic_tokens_full`
