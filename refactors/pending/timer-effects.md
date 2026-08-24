# Timer effects

Requires event-loop.md (landed). Independent of lsp-tokens.md. lsp-diagnostics.md is a consumer of this.

Figaro's two halves: `timer_effect_and_guard` returns a `TimerGuard` the node holds and a `TimerEffect` the handler returns. The effect loop schedules the sleep. Dropping the guard cancels it (`select!` takes the cancel branch; the event is not sent). A sleep that already posted cannot be unsent; `handle` of that firing matches nothing.

That code lives in `freddie` (`timer.rs`, `drop_guard.rs`, `always_equal.rs`) and pulls `bind` for `TimerTrigger`. i2 uses the same two halves, from a crate that does not pull `bind`. `TimerFired` is generic over a payload. The payload is readable only through `trigger_if_matching`.

Two shippable changes. Freddie first, then isograph bumps the git rev and wires `schedule_timer`.

Origin: figaro `figaro_model/src/effect.rs` `FigaroEffect::Timer`, `figaro_model/src/events.rs` `FigaroEvent::Timer`, `figaro/src/daemon.rs` `schedule_timer`, `freddie/crates/freddie/src/timer.rs`. Origin of reset by overwrite: freddie `docs-website/docs/architecture/timers.md`. Delta: crate `freddie_timer`; `TimerFired<P>` with private fields; `trigger_if_matching` instead of bind `TimerTrigger`; i2 payload `HelloWorldDebounced` (deleted when lsp-diagnostics.md lands).

## What the user does

Same verbs. `isograph send` of HelloWorld still logs `hello world`. The log arrives after the timer fires (`Duration::ZERO` in this slice). A second HelloWorld before fire logs once.

## Types

Most important first.

```rust
// external freddie/crates/freddie_timer/src/timer.rs
pub struct TimerFired<P> {
    id: TimerId,
    payload: P,
}

impl<P> TimerFired<P> {
    pub const fn new(id: TimerId, payload: P) -> Self {
        Self { id, payload }
    }

    pub fn trigger_if_matching<'a>(&'a self, guard: Option<&TimerGuard>) -> Option<&'a P> {
        guard
            .is_some_and(|guard| guard.id == self.id)
            .then_some(&self.payload)
    }
}
```

No getter for `payload`. No getter for `id`. Construction is `new`. Matching is `trigger_if_matching`. `TimerGuard.id` is visible here because both types live in this crate.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    HelloWorldDebounced,
}

fn hello_world_debounced(id: freddie_timer::TimerId) -> IsographEvent {
    IsographEvent::Timer(freddie_timer::TimerFired::new(
        id,
        TimerPayload::HelloWorldDebounced,
    ))
}
```

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    Client(ClientConnectionEvent),
    Timer(freddie_timer::TimerFired<TimerPayload>),
}
```

`TimerFired<P>` derives `Clone` when `P: Clone`, `Copy` when `P: Copy`, `Debug` when `P: Debug`. `Timer` is not on `InternalEvent`. The wire is unchanged.

```rust
// from crates/isograph_cli/src/effect.rs
use freddie_timer::TimerEffect;

#[derive(Debug)]
pub enum IsographEffect {
    LogHelloWorld,
    Kill,
    SendLspResponse(Box<SendLspResponseEffect>),
    /// Arm a timer. The effect loop schedules it; it fires its event after the delay unless the
    /// guard held by the state that asked for it drops first.
    Timer(TimerEffect<IsographEvent>),
}
```

Origin of the variant: figaro `FigaroEffect::Timer(TimerEffect<FigaroEvent>)`. Delta: the event type is `IsographEvent`; the crate is `freddie_timer`.

```rust
// from crates/isograph_cli/src/state.rs
pub struct State<THostLanguage: HostLanguage> {
    pub db: IsographState<THostLanguage>,
    pub live_sessions: LiveSessions,
    hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

Arming is figaro's pair:

```rust
// from crates/isograph_cli/src/state.rs
        let (guard, effect) =
            freddie_timer::timer_effect_and_guard(Duration::ZERO, hello_world_debounced);
        state.hello_world_debounce = guard.wrap_some();
        IsographEffect::Timer(effect).wrap_vec()
```

Overwrite assigns a new guard onto the field. That drops the old one, so `select!` cancels that sleep.

## Change 1: `freddie_timer`

New crate `crates/freddie_timer`. Workspace member. Copies `always_equal.rs`, `drop_guard.rs`, `timer.rs` from `crates/freddie/src/`. `crates/freddie` depends on it and re-exports `TimerGuard`, `TimerEffect`, `timer_effect_and_guard`, `TimerFired`, `TimerId`. Figaro and mercury keep importing from `freddie`. The `TimerFired` type change is a compile break in both; the call sites are below.

```toml
# external freddie/crates/freddie_timer/Cargo.toml
[package]
name = "freddie_timer"
version.workspace = true
edition.workspace = true
license.workspace = true
repository.workspace = true

[dependencies]
tokio = { version = "1", features = ["sync"] }
bind = { path = "../bind", version = "0.0.1", default-features = false, optional = true }

[features]
testing = []
bind = ["dep:bind"]

[lints]
workspace = true
```

i2 does not enable `bind`. `crates/freddie` does (`features = ["bind"]`, and `testing` on its `testing` feature).

`timer.rs` copied from `crates/freddie/src/timer.rs`. Delta: `TimerFired<P>` as in Types; `TimerFired::new`; `trigger_if_matching`; `#[derive(Clone, Copy, Debug)]` on `TimerFired` with `P: Clone + Copy + Debug` via the bounds on the impls / derive. The `testing` `PartialEq` that always returns true stays, on `TimerFired<P>`.

`TimerTrigger` and `impl EventTrigger for TimerTrigger` move behind `#[cfg(feature = "bind")]`. `type Event = TimerFired<()>` so a firing with unit payload still matches `TimerGuard::trigger()`.

Figaro and mercury arm with `|id| Event::Timer(TimerFired(id))` today. That becomes `|id| Event::Timer(TimerFired::new(id, ()))`. Handlers that take `_ev: &TimerFired` take `_ev: &TimerFired<()>`. Tests that reconstruct a firing or read `TimerFired(id)` off an effect do the same.

Figaro (`figaro_model`, path dep on `freddie`): `events.rs` `FigaroEvent::Timer(TimerFired)`; `model/mod.rs` overlay; `model/windows.rs` settle; `model/typing/mod.rs` `arm_window`; handlers in `overlay.rs` / `windows.rs` / `typing/mod.rs`; `tests/transitions.rs` `fired` / `timer_id`. Figaro still depends on `freddie`, which re-exports. It does not add a `freddie_timer` dep.

Mercury (`mercury_model` in the freddie repo): the same `TimerFired(id)` / `&TimerFired` sites. Lands in the same change 1 commit as the crate.

`timer_effect_and_guard` is unchanged: `(delay, impl FnOnce(TimerId) -> E)`.

```rust
// external freddie/crates/freddie/src/lib.rs
pub use freddie_timer::{
    AlwaysEqual, DropGuard, TimerEffect, TimerFired, TimerGuard, TimerId, drop_guard,
    timer_effect_and_guard,
};

#[cfg(feature = "bind")]
pub use freddie_timer::TimerTrigger;
```

`crates/freddie` deletes `always_equal.rs`, `drop_guard.rs`, `timer.rs` and the `pub mod` lines. `sequence.rs` keeps `use crate::TimerGuard` via the re-export. `crates/freddie/Cargo.toml` drops direct `tokio`; `freddie_timer` is the tokio user. `bind` stays on `freddie` only to enable `freddie_timer/bind`.

```toml
# external freddie/crates/freddie/Cargo.toml (after)
[dependencies]
bind = { path = "../bind", version = "0.0.1", default-features = false }
freddie_keys = { path = "../freddie_keys", version = "0.0.1" }
freddie_timer = { path = "../freddie_timer", version = "0.0.1", features = ["bind"] }

[features]
testing = ["freddie_timer/testing"]
```

Workspace `members` gains `"crates/freddie_timer"`.

## Change 2: isograph uses `freddie_timer`

```toml
# from crates/isograph_cli/Cargo.toml (before)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
```

```toml
# from crates/isograph_cli/Cargo.toml (after)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
freddie_timer = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
```

The rev is the commit that landed change 1, all three equal. No `bind` dep. No `freddie` dep.

`empty()` fills `hello_world_debounce: None`.

```rust
// from crates/isograph_cli/src/state.rs (before)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(&mut state.db, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::Client(ClientConnectionEvent::Started(started)) => {
```

```rust
// from crates/isograph_cli/src/state.rs (after)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(&mut state.db, lsp),
        IsographEvent::Internal(InternalEvent::HelloWorld) => handle_hello_world(state),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::Timer(fired) => handle_timer_fired(state, fired),
        IsographEvent::Client(ClientConnectionEvent::Started(started)) => {
```

`handle_internal` loses the `HelloWorld` arm. HelloWorld writes the guard on `State`.

```rust
// from crates/isograph_cli/src/state.rs
fn handle_hello_world<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) =
        freddie_timer::timer_effect_and_guard(Duration::ZERO, hello_world_debounced);
    state.hello_world_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

fn handle_timer_fired<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.hello_world_debounce.as_ref()) {
        Some(&TimerPayload::HelloWorldDebounced) => {
            state.hello_world_debounce = None;
            IsographEffect::LogHelloWorld.wrap_vec()
        }
        None => Vec::new(),
    }
}
```

`state.rs` gains `use std::time::Duration` and `use crate::event::{TimerPayload, hello_world_debounced}`.

Origin of `schedule_timer` and of `perform` taking `event_tx`: figaro `src/daemon.rs`. Delta: `IsographEvent` / `IsographEffect`; crate `freddie_timer`; i2's `perform` gains `event_tx`.

```rust
// from crates/isograph_cli/src/daemon.rs (before)
        () = run_effect_loop(effect_rx) => {}
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
        () = run_effect_loop(effect_rx, event_tx.clone()) => {}
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
pub(crate) async fn run_effect_loop(
    mut effect_rx: UnboundedReceiver<IsographEffect>,
    event_tx: UnboundedSender<IsographEvent>,
) {
    while let Some(effect) = effect_rx.recv().await {
        tracing::debug!(?effect, "effect");
        if perform(effect, event_tx.reference()).is_break() {
            break;
        }
    }
}

pub fn perform(
    effect: IsographEffect,
    event_tx: &UnboundedSender<IsographEvent>,
) -> ControlFlow<()> {
    match effect {
        IsographEffect::LogHelloWorld => {
            tracing::info!("hello world");
            ControlFlow::Continue(())
        }
        IsographEffect::Kill => {
            tracing::info!("kill: exiting");
            ControlFlow::Break(())
        }
        IsographEffect::SendLspResponse(respond) => {
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
        IsographEffect::Timer(timer) => schedule_timer(timer, event_tx),
    }
}

/// Schedule `timer`: fire its event after its delay, unless the guard the model kept drops first.
///
/// Fire-and-forget on the runtime, so a pending sleep runs off the effect loop.
fn schedule_timer(
    timer: TimerEffect<IsographEvent>,
    event_tx: &UnboundedSender<IsographEvent>,
) {
    let TimerEffect {
        delay,
        event,
        cancel: AlwaysEqual(mut cancel),
    } = timer;
    // If the guard dropped before the loop reached this, the receiver is closed and the timer is
    // already cancelled, so there is nothing to spawn.
    if matches!(cancel.try_recv(), Err(TryRecvError::Closed)) {
        return;
    }
    let event_tx = event_tx.clone();
    tokio::spawn(async move {
        tokio::select! {
            () = tokio::time::sleep(delay) => { let _ = event_tx.send(event); }
            _ = cancel => {}
        }
    });
}
```

`daemon.rs` gains `use freddie_timer::{AlwaysEqual, TimerEffect};` and `use tokio::sync::oneshot::error::TryRecvError;`.

Callers of `run_effect_loop` and `perform` pass `event_tx`:

- `serve`'s `select!`: `event_tx.clone()`
- `lsp_socket.rs` `listen_and_reply`: clone `loop_tx` before the forwarder spawn, pass that clone
- `kill_ends_the_effect_loop`: an `event_tx` the test does not read
- `lsp_request_returns_method_not_found`: an `event_tx` the test does not read

```rust
// from crates/isograph_cli/src/state.rs
        let (event_tx, _event_rx) = tokio::sync::mpsc::unbounded_channel();
        let _ = crate::daemon::perform(
            crate::effect::IsographEffect::SendLspResponse(send),
            event_tx.reference(),
        );
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
    #[tokio::test]
    async fn kill_ends_the_effect_loop() {
        let (effect_tx, effect_rx) = unbounded_channel();
        let (event_tx, _event_rx) = unbounded_channel();
        effect_tx
            .send(IsographEffect::Kill)
            .expect("the test sends Kill");
        run_effect_loop(effect_rx, event_tx).await;
        let _hold = effect_tx;
    }
```

`lsp_socket.rs` `listen_and_reply`, next to `let (loop_tx, loop_rx) = unbounded_channel();`: `let effect_event_tx = loop_tx.clone();`, and `run_effect_loop(effect_rx, effect_event_tx)`.

`sending_hello_world_emits_log_hello_world` drives the event loop only, so it now receives `Timer` not `LogHelloWorld`. Assert `Timer`.

## Tests

Park on `event_rx.recv().await`. Absence of a firing has no edge: inject `Quit` after the cancel, `recv().await`, that event is `Quit`. If the timer had posted, it would have arrived first.

```rust
// from crates/isograph_cli/src/state.rs
    #[test]
    fn hello_world_returns_a_timer() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        assert!(matches!(effects.as_slice(), [IsographEffect::Timer(_)]));
        assert!(state.hello_world_debounce.is_some());
    }

    #[test]
    fn matching_hello_world_debounced_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let event = timer.event;
        let effects = handle(&mut state, event);
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
        assert!(state.hello_world_debounce.is_none());
    }

    #[test]
    fn stale_hello_world_debounced_returns_no_effects() {
        let mut state = empty();
        let effects = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        state.hello_world_debounce = None;
        let effects = handle(&mut state, timer.event);
        assert!(effects.is_empty());
    }

    #[test]
    fn a_second_hello_world_drops_the_first_guard() {
        let mut state = empty();
        let first = handle(&mut state, InternalEvent::HelloWorld.to());
        let _second = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(timer) = first
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, timer.event);
        assert!(effects.is_empty());
    }
```

The matching test does not `perform` the timer: it feeds `timer.event` to `handle` as if the sleep had won. The stale test drops the field first. The second HelloWorld test is overwrite.

```rust
// from crates/isograph_cli/src/daemon.rs
    #[tokio::test]
    async fn a_timer_effect_fires_its_event_after_a_zero_delay() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) =
            freddie_timer::timer_effect_and_guard(Duration::ZERO, hello_world_debounced);
        assert!(perform(IsographEffect::Timer(timer), event_tx.reference()).is_continue());
        let event = event_rx.recv().await.expect("the timer fired");
        let IsographEvent::Timer(fired) = event else {
            panic!("the timer posts TimerFired");
        };
        assert!(matches!(
            fired.trigger_if_matching(guard.reference().wrap_some()),
            Some(&TimerPayload::HelloWorldDebounced)
        ));
    }

    #[tokio::test]
    async fn dropping_the_guard_before_perform_does_not_fire() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) =
            freddie_timer::timer_effect_and_guard(Duration::ZERO, hello_world_debounced);
        drop(guard);
        let _ = perform(IsographEffect::Timer(timer), event_tx.reference());
        event_tx
            .send(InternalEvent::Quit.to())
            .expect("the test sends Quit");
        let event = event_rx.recv().await.expect("Quit arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::Quit)
        ));
    }

    #[tokio::test]
    async fn dropping_the_guard_after_perform_does_not_fire() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::from_secs(60),
            hello_world_debounced,
        );
        let _ = perform(IsographEffect::Timer(timer), event_tx.reference());
        drop(guard);
        event_tx
            .send(InternalEvent::Quit.to())
            .expect("the test sends Quit");
        let event = event_rx.recv().await.expect("Quit arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::Quit)
        ));
    }
```

`Duration::ZERO` is the degenerate delay. The 60s delay is only to make a late fire lose to `Quit` if cancel failed. `daemon.rs` tests gain `use super::perform`, `use std::time::Duration`, `IsographEvent`, `TimerPayload`, `hello_world_debounced`. `lsp_isograph_event_hello_world_returns_log_hello_world` becomes a timer, same as `hello_world_returns_a_timer`.

Keep `guard` alive until after `trigger_if_matching` in the fire test.

## Call sites

- `handle` of HelloWorld -> `Timer` effect plus guard
- `handle` of `TimerFired` -> `trigger_if_matching` -> `LogHelloWorld` or empty
- `perform` of `Timer` -> `schedule_timer` -> `event_tx.send` of the baked event, or cancel
- `serve` / `listen_and_reply` / `kill_ends_the_effect_loop` pass `event_tx` into `run_effect_loop`
- `lsp_request_returns_method_not_found` passes `event_tx` into `perform`

## Design doc

`docs-website/docs/design-docs/event-model.md` gains `Timer(TimerFired<TimerPayload>)` on `IsographEvent` and `Timer(TimerEffect<IsographEvent>)` on `IsographEffect`. It is not on `InternalEvent`. `handle` of `Timer` uses `trigger_if_matching`. `handle` does not sleep. Performing `Timer` is `schedule_timer`.
