# Event loop and dispatch

Requires config-discovery.md.

An event loop (`run_event_loop`) recvs `IsographEvent`s and calls `handle`. An effects loop (`run_effect_loop`) recvs `IsographEffect`s and calls `perform`. `handle` turns `HelloWorld` into `LogHelloWorld` and `Quit` into `Kill`. `perform` logs `hello world` for `LogHelloWorld`. `perform` returns `ControlFlow::Break` for `Kill`, which ends the effect loop. `tokio::sync::mpsc::unbounded_channel` carries both. `run` builds a current-thread runtime and `block_on(serve)`. `serve` `tokio::select!`s the two loops. A unix `tokio::signal` task sends `Quit`. Origin: figaro `src/daemon.rs`. Delta: no AppKit worker thread (`run` is already the daemon thread); `handle` takes `event: IsographEvent`, owned; `HelloWorld` / `LogHelloWorld` beside `Quit` / `Kill`; SIGTERM is `#[cfg(unix)]`.

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

`isograph stop` without `--force` is SIGTERM (unix). `serve` sends `HelloWorld` at boot. SIGTERM sends `Quit`. `handle` returns `Kill`. `select!` ends. The process returns from `run_daemon`.

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

Origin of `perform` returning `ControlFlow`: figaro `perform_effect` in `src/daemon.rs`. `Kill` breaks rather than `process::exit`, so `serve` returns and destructors run.

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
tokio = { workspace = true, features = ["rt", "macros", "signal", "sync", "time"] }
tracing = { workspace = true }
```

Origin of the tokio features: figaro `Cargo.toml` `["rt", "macros", "signal", "sync", "time"]`. The workspace already pins `tokio` 1.35.

```rust
// from crates/isograph_cli/src/daemon.rs
use std::ops::ControlFlow;

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::IsographState;

pub fn run() {
    let runtime = match tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(e) => {
            tracing::error!(error = %e, "could not start the tokio runtime");
            return;
        }
    };
    runtime.block_on(serve());
}

async fn serve() {
    let (event_tx, event_rx) = unbounded_channel::<IsographEvent>();
    let (effect_tx, effect_rx) = unbounded_channel::<IsographEffect>();
    let _ = event_tx.send(IsographEvent::HelloWorld);

    // `isograph stop` sends SIGTERM. Route it into the event channel as Quit, so the
    // model turns it into Kill, the effect loop breaks, and serve returns.
    //
    // A spawned task rather than a third `select!` arm, because an arm that completed
    // would drop the other two futures and skip the graceful path this exists to run.
    #[cfg(unix)]
    match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
        Ok(mut term) => {
            let event_tx = event_tx.clone();
            tokio::spawn(async move {
                if term.recv().await.is_some() {
                    tracing::info!("SIGTERM: quitting");
                    let _ = event_tx.send(IsographEvent::Quit);
                }
            });
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                "no SIGTERM handler; a terminated isograph will not run Kill"
            );
        }
    }

    // `select!` rather than `join!`: the effect loop ends on `Kill`, and the event
    // loop never does, because `_hold_events` holds a sender for as long as serve runs.
    let _hold_events = event_tx;
    let state = IsographState;
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
    }
}

pub(crate) async fn run_event_loop(
    mut state: IsographState,
    mut event_rx: UnboundedReceiver<IsographEvent>,
    effect_tx: UnboundedSender<IsographEffect>,
) {
    while let Some(event) = event_rx.recv().await {
        let effects = state.handle(event);
        for effect in effects {
            let _ = effect_tx.send(effect);
        }
    }
}

pub(crate) async fn run_effect_loop(mut effect_rx: UnboundedReceiver<IsographEffect>) {
    while let Some(effect) = effect_rx.recv().await {
        if perform(effect).is_break() {
            break;
        }
    }
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

Origin of `run` / `serve` / the two loops / SIGTERM / `select!`: figaro `src/daemon.rs`. Figaro builds the current-thread runtime on a worker because main is AppKit. `run` is already the daemon thread, so it builds the runtime here. Figaro `expect`s that `build()`. `run` logs and returns if `build()` fails.

`_hold_events` keeps the event channel open while `serve` runs. `run_event_loop` holds `effect_tx`, which keeps the effect channel open. A closed event channel ends `while let Some`. `Kill` breaks the effect loop. `select!` then cancels the event loop.

On Windows there is no SIGTERM task. The event loop stays selected until SIGKILL. `isograph stop` on Windows is `--force` (SIGKILL). Origin: `crates/ts_graphql_react_isograph_cli/tests/cli.rs` `STOP`.

`App::DaemonArgs` remains `NoArgs`. `ConfigFlag` remains.

## Tests

`run_event_loop` and `run_effect_loop` are `pub(crate)`.

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
    use crate::state::IsographState;
    use tokio::sync::mpsc::unbounded_channel;

    #[tokio::test]
    async fn sending_hello_world_emits_log_hello_world() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(IsographEvent::HelloWorld)
            .expect("the test sends HelloWorld");
        drop(event_tx);
        run_event_loop(IsographState, event_rx, effect_tx).await;
        let effect = effect_rx
            .recv()
            .await
            .expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::LogHelloWorld);
    }

    #[tokio::test]
    async fn sending_quit_emits_kill() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(IsographEvent::Quit)
            .expect("the test sends Quit");
        drop(event_tx);
        run_event_loop(IsographState, event_rx, effect_tx).await;
        let effect = effect_rx
            .recv()
            .await
            .expect("handle sent one effect");
        assert_eq!(effect, IsographEffect::Kill);
    }

    #[tokio::test]
    async fn kill_ends_the_effect_loop() {
        let (effect_tx, effect_rx) = unbounded_channel();
        effect_tx
            .send(IsographEffect::Kill)
            .expect("the test sends Kill");
        run_effect_loop(effect_rx).await;
        let _hold = effect_tx;
    }
}
```

`kill_ends_the_effect_loop` keeps the sender alive. The loop returns because `Kill` breaks, not because the channel closed.

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
