# Event loop and dispatch

Requires config-discovery.md.

An event loop (`run_event_loop`) recvs `IsographEvent`s and calls `handle`. An effects loop (`run_effect_loop`) recvs `IsographEffect`s and calls `perform`. `handle` turns `HelloWorld` into `LogHelloWorld` and `Quit` into `Kill`. `perform` logs `hello world` for `LogHelloWorld`. `perform` returns `ControlFlow::Break` for `Kill`, which ends the effect loop. `std::sync::mpsc` carries both. The event thread is `run`. The effects thread is `std::thread::spawn`. A unix SIGTERM thread sends `Quit` and drops its sender, so the event loop's next `recv()?` ends. Origin of the two loops and of SIGTERM → Quit → Kill: figaro `src/daemon.rs`. Delta: `std::sync::mpsc` and `std::thread` in place of tokio tasks; `signal-hook` in place of `tokio::signal`; the SIGTERM thread drops the event sender after `Quit` in place of `select!` cancelling the event loop; `HelloWorld` / `LogHelloWorld` beside `Quit` / `Kill`.

## What the user does

```
$ isograph start
/Users/x/app/isograph.config.json started (pid 12345)
$ isograph status
/Users/x/app/isograph.config.json is running (pid 12345)
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json"}}
{"timestamp":"...","level":"INFO","fields":{"message":"hello world"}}
$ isograph stop
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"SIGTERM: quitting"}}
{"timestamp":"...","level":"INFO","fields":{"message":"kill: exiting"}}
```

`isograph stop` without `--force` is SIGTERM (unix). `run` sends `HelloWorld` at boot. SIGTERM sends `Quit`. `handle` returns `Kill`. The process returns from `run_daemon`.

A test sends `HelloWorld` into `run_event_loop`. The effects channel receives `LogHelloWorld`. A test sends `Quit`. The effects channel receives `Kill`. A test sends `Kill` into `run_effect_loop`. The loop returns.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Debug)]
pub enum IsographEvent {
    HelloWorld,
    Quit,
}
```

Origin of `Quit`: `docs-website/docs/design-docs/event-model.md` `Quit(Quit)`. Delta: unit variant.

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug, PartialEq, Eq)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
}
```

Origin of `Kill`: `docs-website/docs/design-docs/event-model.md`. Delta: beside `LogHelloWorld`.

```rust
// from crates/isograph_cli/src/state.rs
use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use prelude::Postfix;

pub struct IsographState;

impl IsographState {
    pub fn handle(&mut self, event: IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
            IsographEvent::Quit => IsographEffect::Kill.wrap_vec(),
        }
    }
}
```

Origin of `handle`: figaro `Figaro::handle(&mut self, event: &FigaroEvent)`. Delta: `event: IsographEvent`, owned.

```rust
// from crates/isograph_cli/src/daemon.rs
use std::ops::ControlFlow;

use crate::effect::IsographEffect;

pub fn perform(effect: IsographEffect) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
    }
}
```

Origin of `perform` returning `ControlFlow`: figaro `perform_effect` in `src/daemon.rs`. `Kill` breaks rather than `process::exit`, so `run` returns and destructors run.

## Change 1: two loops

`lib.rs` today: `mod discover;` and `run_daemon` parks.

```rust
// from crates/isograph_cli/src/lib.rs (before)
mod discover;
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
mod daemon;
mod discover;
mod effect;
mod event;
mod state;
```

```rust
// from crates/isograph_cli/src/lib.rs (before)
    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, _, _config)) => {
                tracing::info!(config = %path.display(), "isograph daemon up");
                loop {
                    std::thread::park();
                }
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
```

```rust
// from crates/isograph_cli/src/lib.rs (after)
    fn run_daemon(id: &ConfigFlag, _: &NoArgs) {
        match discover::config_and_instance(id.config.as_deref()) {
            Ok((path, _, _config)) => {
                tracing::info!(config = %path.display(), "isograph daemon up");
                crate::daemon::run();
            }
            Err(e) => {
                tracing::error!(error = %e, "the config went away between naming this daemon and starting it");
            }
        }
    }
```

Delta: `park` becomes `daemon::run()`. The `discover` match stays.

```toml
# from crates/isograph_cli/Cargo.toml (before)
[dependencies]
clap = { workspace = true }
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "af6b57df9732f42cde9bafc109bcf1b6a32f28e0" }
prelude = { path = "../prelude" }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = "2"
tracing = { workspace = true }
```

```toml
# from crates/isograph_cli/Cargo.toml (after)
[dependencies]
clap = { workspace = true }
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "af6b57df9732f42cde9bafc109bcf1b6a32f28e0" }
prelude = { path = "../prelude" }
serde = { workspace = true, features = ["derive"] }
serde_json = { workspace = true }
thiserror = "2"
tracing = { workspace = true }

[target.'cfg(unix)'.dependencies]
signal-hook = "0.3"
```

`std` has no SIGTERM API. Origin of a dedicated SIGTERM waiter: figaro `tokio::signal::unix::signal(SignalKind::terminate())`. Delta: `signal-hook` `Signals` on a `std::thread`, unix only.

```rust
// from crates/isograph_cli/src/daemon.rs
use std::ops::ControlFlow;
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, channel};

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::IsographState;
use prelude::Postfix;

#[derive(Debug, thiserror::Error)]
enum EventLoopError {
    #[error("{0}")]
    Recv(#[from] RecvError),
    #[error("{0}")]
    Send(#[from] SendError<IsographEffect>),
}

pub fn run() {
    let (event_tx, event_rx) = channel::<IsographEvent>();
    let (effect_tx, effect_rx) = channel::<IsographEffect>();
    let _ = event_tx.send(IsographEvent::HelloWorld);
    let _hold_events = spawn_sigterm_or_hold(event_tx);
    let effect_thread = std::thread::spawn(move || {
        run_effect_loop(effect_rx).unwrap_or_else(|e| {
            tracing::error!(error = %e, "effect loop ended");
        });
    });
    run_event_loop(event_rx, effect_tx).unwrap_or_else(|e| {
        tracing::error!(error = %e, "event loop ended");
    });
    if effect_thread.join().is_err() {
        tracing::error!("effect thread panicked");
    }
}

/// `None`: the SIGTERM thread owns `event_tx` and drops it after sending `Quit`.
/// `Some`: no such thread; the caller holds the sender so the event channel stays open.
fn spawn_sigterm_or_hold(event_tx: Sender<IsographEvent>) -> Option<Sender<IsographEvent>> {
    #[cfg(unix)]
    {
        let mut signals = match signal_hook::iterator::Signals::new([signal_hook::consts::SIGTERM])
        {
            Ok(signals) => signals,
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    "no SIGTERM handler; a terminated isograph will not run Kill"
                );
                return event_tx.wrap_some();
            }
        };
        std::thread::spawn(move || {
            if signals.forever().next().is_some() {
                tracing::info!("SIGTERM: quitting");
                let _ = event_tx.send(IsographEvent::Quit);
            }
        });
        None
    }
    #[cfg(not(unix))]
    {
        event_tx.wrap_some()
    }
}

pub(crate) fn run_event_loop(
    event_rx: Receiver<IsographEvent>,
    effect_tx: Sender<IsographEffect>,
) -> Result<(), EventLoopError> {
    let mut state = IsographState;
    loop {
        let event = event_rx.recv()?;
        let effects = state.handle(event);
        for effect in effects {
            effect_tx.send(effect)?;
        }
    }
}

fn run_effect_loop(effect_rx: Receiver<IsographEffect>) -> Result<(), RecvError> {
    loop {
        let effect = effect_rx.recv()?;
        if perform(effect).is_break() {
            break;
        }
    }
    ().wrap_ok()
}

pub fn perform(effect: IsographEffect) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
    }
}
```

`run` sends `HelloWorld` before moving `event_tx`. `spawn_sigterm_or_hold` registers SIGTERM on this thread, then either moves `event_tx` into a waiter thread (`None`) or returns it (`Some`). The waiter sends `Quit` and returns, which drops the last sender, which makes `run_event_loop`'s next `recv()?` return `RecvError`. `run` then joins the effect thread.

On Windows, and if `Signals::new` fails, `_hold_events` holds the sender. The event loop stays blocked until SIGKILL. `isograph stop` on Windows is `--force` (SIGKILL). Origin: `crates/ts_graphql_react_isograph_cli/tests/cli.rs` `STOP`.

`recv()?` and `send()?` end a loop when the other end hangs up.

`App::DaemonArgs` remains `NoArgs`. `ConfigFlag` remains.

`isograph_cli` already depends on `thiserror`.

## Tests

`run_event_loop` is `pub(crate)`.

```rust
// from crates/isograph_cli/src/state.rs
#[cfg(test)]
mod tests {
    use super::IsographState;
    use crate::effect::IsographEffect;
    use crate::event::IsographEvent;
    use prelude::Postfix;

    #[test]
    fn hello_world_returns_log_hello_world() {
        let mut state = IsographState;
        let effects = state.handle(IsographEvent::HelloWorld);
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }

    #[test]
    fn quit_returns_kill() {
        let mut state = IsographState;
        let effects = state.handle(IsographEvent::Quit);
        assert_eq!(effects, IsographEffect::Kill.wrap_vec());
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
#[cfg(test)]
mod tests {
    use super::{run_effect_loop, run_event_loop};
    use crate::effect::IsographEffect;
    use crate::event::IsographEvent;
    use std::sync::mpsc::channel;

    #[test]
    fn sending_hello_world_emits_log_hello_world() {
        let (event_tx, event_rx) = channel();
        let (effect_tx, effect_rx) = channel();
        event_tx
            .send(IsographEvent::HelloWorld)
            .expect("the test sends HelloWorld");
        drop(event_tx);
        run_event_loop(event_rx, effect_tx)
            .expect_err("the test dropped the event sender after one event");
        let effect = effect_rx
            .recv()
            .expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::LogHelloWorld);
    }

    #[test]
    fn sending_quit_emits_kill() {
        let (event_tx, event_rx) = channel();
        let (effect_tx, effect_rx) = channel();
        event_tx
            .send(IsographEvent::Quit)
            .expect("the test sends Quit");
        drop(event_tx);
        run_event_loop(event_rx, effect_tx)
            .expect_err("the test dropped the event sender after one event");
        let effect = effect_rx
            .recv()
            .expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::Kill);
    }

    #[test]
    fn kill_ends_the_effect_loop() {
        let (effect_tx, effect_rx) = channel();
        let effect_thread = std::thread::spawn(move || run_effect_loop(effect_rx));
        effect_tx
            .send(IsographEffect::Kill)
            .expect("the test sends Kill");
        drop(effect_tx);
        effect_thread
            .join()
            .expect("the effect loop thread returns")
            .expect("Kill ends the effect loop without RecvError");
    }
}
```

## e2e

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (before)
    poll(|| {
        let log = daemon.log_text();
        (log.contains("isograph daemon up") && log.contains(path_in_log.reference())).then_some(())
    });
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (after)
    poll(|| {
        let log = daemon.log_text();
        (log.contains("isograph daemon up")
            && log.contains(path_in_log.reference())
            && log.contains("hello world"))
            .then_some(())
    });
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (before)
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
}
```

```rust
// from crates/ts_graphql_react_isograph_cli/tests/cli.rs (after)
    poll(|| (!daemon.isograph(["status"].reference()).status.success()).then_some(()));
    #[cfg(not(windows))]
    poll(|| {
        let log = daemon.log_text();
        (log.contains("SIGTERM: quitting") && log.contains("kill: exiting")).then_some(())
    });
}
```

Windows `STOP` is `["stop", "--force"]` (SIGKILL). Those two log lines are unix SIGTERM.

`crates/ts_graphql_react_isograph_cli/tests/cli.rs` still:

- start then status reports running
- the log contains `isograph daemon up`, the config path, and `hello world`
- stop then status reports not running; on unix the log contains `SIGTERM: quitting` and `kill: exiting`
- a second start adopts the running daemon
