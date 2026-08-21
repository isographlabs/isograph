# Event loop and dispatch

Requires config-discovery.md.

An event loop (`run_event_loop`) recvs `IsographEvent`s and calls `handle`. An effects loop (`run_effect_loop`) recvs `IsographEffect`s and calls `perform`. `handle` turns `HelloWorld` into `LogHelloWorld`. `perform` writes that to the log. `std::sync::mpsc` carries both. The event thread is `run`. The effects thread is `std::thread::spawn`. Origin of the two loops: figaro `src/daemon.rs`. Delta: `std::sync::mpsc` and `std::thread` in place of tokio tasks; `HelloWorld` / `LogHelloWorld`.

## What the user does

```
$ isograph start
/Users/x/app/isograph.config.json started (pid 12345)
$ isograph status
/Users/x/app/isograph.config.json is running (pid 12345)
$ isograph logs
{"timestamp":"...","level":"INFO","fields":{"message":"isograph daemon up","config":"/Users/x/app/isograph.config.json"}}
$ isograph stop
```

A test sends `HelloWorld` into `run_event_loop`. The effects loop receives `LogHelloWorld`. `perform` logs `hello world`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Debug)]
pub enum IsographEvent {
    HelloWorld,
}
```

```rust
// from crates/isograph_cli/src/effect.rs
#[derive(Debug, PartialEq, Eq)]
pub enum IsographEffect {
    LogHelloWorld,
}
```

```rust
// from crates/isograph_cli/src/state.rs
use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use prelude::Postfix;

pub struct IsographState;

impl IsographState {
    pub fn handle(&mut self, event: &IsographEvent) -> Vec<IsographEffect> {
        match event {
            IsographEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        }
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
use crate::effect::IsographEffect;

pub fn perform(effect: IsographEffect) {
    match effect {
        IsographEffect::LogHelloWorld => tracing::info!("hello world"),
    }
}
```

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

```rust
// from crates/isograph_cli/src/daemon.rs
use std::sync::mpsc::{Receiver, RecvError, SendError, Sender, channel};

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::IsographState;

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
    let _hold_events = event_tx;
    std::thread::spawn(move || {
        run_effect_loop(effect_rx).unwrap_or_else(|e| {
            tracing::error!(error = %e, "effect loop ended");
        });
    });
    run_event_loop(event_rx, effect_tx).unwrap_or_else(|e| {
        tracing::error!(error = %e, "event loop ended");
    });
}

fn run_event_loop(
    event_rx: Receiver<IsographEvent>,
    effect_tx: Sender<IsographEffect>,
) -> Result<(), EventLoopError> {
    let mut state = IsographState;
    loop {
        let event = event_rx.recv()?;
        let effects = state.handle(&event);
        for effect in effects {
            effect_tx.send(effect)?;
        }
    }
}

fn run_effect_loop(effect_rx: Receiver<IsographEffect>) -> Result<(), RecvError> {
    loop {
        let effect = effect_rx.recv()?;
        perform(effect);
    }
}

pub fn perform(effect: IsographEffect) {
    match effect {
        IsographEffect::LogHelloWorld => tracing::info!("hello world"),
    }
}
```

`_hold_events` keeps the event channel open while the daemon runs. `run_event_loop` holds `effect_tx`, which keeps the effect channel open. `recv()?` and `send()?` end the loop when the other end hangs up.

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
        let effects = state.handle(&IsographEvent::HelloWorld);
        assert_eq!(effects, IsographEffect::LogHelloWorld.wrap_vec());
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
#[cfg(test)]
mod tests {
    use super::run_event_loop;
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
}
```

`crates/ts_graphql_react_isograph_cli/tests/cli.rs` still:

- start then status reports running
- the log contains `isograph daemon up` and the config path
- stop then status reports not running
- a second start adopts the running daemon
