# Event loop and dispatch, empty events and effects

Requires config-discovery.md (landed).

The daemon stops parking. It recvs on a channel of `IsographEvent`, calls `handle`, and performs the `IsographEffect`s `handle` returned. Both enums have no variants. Nothing is sent. There is no socket, no CLI verb, no `--port`, and no domain state.

Not `bind`. Not pico. `std::sync::mpsc`. One thread: recv, dispatch, perform.

The event sender is held for the life of `run` so `recv` does not disconnect and exit. SIGTERM still ends the process.

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

Same verbs as today. The process is in `recv`, not `park`.

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

No variants. Neither derives `Deserialize`. No value of either type can be constructed.

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

The match is exhaustive. It diverges, so the return type `Vec<IsographEffect>` is satisfied without constructing a vec.

```rust
// from crates/isograph_cli/src/daemon.rs
pub fn perform(effect: IsographEffect) {
    match effect {}
}
```

## Change 1: recv, handle, perform

Before:

```rust
// from crates/isograph_cli/src/lib.rs
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

After:

```rust
// from crates/isograph_cli/src/lib.rs
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

```rust
// from crates/isograph_cli/src/daemon.rs
use std::sync::mpsc::channel;

use crate::effect::IsographEffect;
use crate::event::IsographEvent;
use crate::state::IsographState;

pub fn run() {
    let (event_tx, event_rx) = channel::<IsographEvent>();
    let mut state = IsographState;
    let _hold = event_tx;
    loop {
        let event = match event_rx.recv() {
            Ok(event) => event,
            Err(_) => break,
        };
        let effects = state.handle(&event);
        for effect in effects {
            perform(effect);
        }
    }
}

pub fn perform(effect: IsographEffect) {
    match effect {}
}
```

`_hold` is the event sender. Dropping it makes `recv` return `Err` and `run` return. Do not drop it.

`App::DaemonArgs` stays `NoArgs`. `ConfigFlag` stays.

`mod event;`, `mod effect;`, `mod state;`, `mod daemon;` in `lib.rs`.

No new crate dependencies.

## Tests

Existing e2e in `crates/ts_graphql_react_isograph_cli/tests/cli.rs` still pass: start, status running, log contains `isograph daemon up` and the config path, stop, second start adopts.

There is no `IsographEvent` or `IsographEffect` value. Do not add a constructing API for tests.
