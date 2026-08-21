# Event loop and dispatch

Requires config-discovery.md.

The daemon recvs `IsographEvent`s, calls `handle`, and sends the returned `IsographEffect`s to a performer loop. Both enums have zero variants. `std::sync::mpsc` carries events and effects. The event thread is `run_daemon`. The effect thread is `std::thread::spawn`. Origin of the two loops: figaro `src/daemon.rs`. Delta: `std::sync::mpsc` and `std::thread` in place of tokio tasks; empty enums.

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

## Types

Most important first.

```rust
// from crates/isograph_cli/src/event.rs
pub enum IsographEvent {}
```

```rust
// from crates/isograph_cli/src/effect.rs
pub enum IsographEffect {}
```

```rust
// from crates/isograph_cli/src/state.rs
use crate::effect::IsographEffect;
use crate::event::IsographEvent;

pub struct IsographState;

impl IsographState {
    pub fn handle(&mut self, event: &IsographEvent) -> Vec<IsographEffect> {
        match event {}
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs
use crate::effect::IsographEffect;

pub fn perform(effect: IsographEffect) {
    match effect {}
}
```

`handle` and `perform` match exhaustively. The empty match diverges.

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
use std::sync::mpsc::{Receiver, Sender, channel};

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::IsographState;

pub fn run() {
    let (event_tx, event_rx) = channel::<IsographEvent>();
    let (effect_tx, effect_rx) = channel::<IsographEffect>();
    let _hold_events = event_tx;
    std::thread::spawn(move || run_effect_loop(effect_rx));
    run_event_loop(event_rx, effect_tx);
}

fn run_event_loop(event_rx: Receiver<IsographEvent>, effect_tx: Sender<IsographEffect>) {
    let mut state = IsographState;
    loop {
        let Ok(event) = event_rx.recv() else {
            break;
        };
        let effects = state.handle(&event);
        for effect in effects {
            let Ok(()) = effect_tx.send(effect) else {
                break;
            };
        }
    }
}

fn run_effect_loop(effect_rx: Receiver<IsographEffect>) {
    loop {
        let Ok(effect) = effect_rx.recv() else {
            break;
        };
        perform(effect);
    }
}

pub fn perform(effect: IsographEffect) {
    match effect {}
}
```

`_hold_events` keeps the event channel open. `run_event_loop` holds `effect_tx`, which keeps the effect channel open. `run` returns when the event channel closes. The effect thread returns when `effect_tx` is dropped.

`App::DaemonArgs` remains `NoArgs`. `ConfigFlag` remains.

Origin of the two loops: figaro `src/daemon.rs` recvs events, calls `handle`, sends each effect; a second loop recvs effects and performs them. Delta: `std::sync::mpsc` and `std::thread` in place of tokio tasks; empty enums.

## Tests

`crates/ts_graphql_react_isograph_cli/tests/cli.rs`:

- start then status reports running
- the log contains `isograph daemon up` and the config path
- stop then status reports not running
- a second start adopts the running daemon
