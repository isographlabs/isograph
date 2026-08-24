# Timer effects

Requires event-loop.md (landed). Independent of lsp-sessions.md. lsp-diagnostics.md's debounce is a consumer of this.

A timer is two linked halves built by one call: a guard the owning node holds, and an effect a handler returns. The effect loop schedules the effect. Dropping the guard cancels it. Every timer fires the same event, `Timer(TimerFired)`. What tells them apart is which node still holds that guard.

Origin: figaro `figaro_model/src/effect.rs` `FigaroEffect::Timer`, `figaro_model/src/events.rs` `FigaroEvent::Timer`, `figaro/src/daemon.rs` `schedule_timer` / `perform_effect` taking `event_tx`, `freddie::timer_effect_and_guard`. Origin of matching by the guard: freddie `docs-website/docs/architecture/timers.md`. Delta: `IsographEvent` / `IsographEffect`; `handle` matches `TimerFired` rather than bind dispatch; `Timer` is not `Internal` (not on the `isograph/event` wire); this slice adds no guard field, so every firing is unmatched and `handle` of it is no effects.

One shippable change.

## What the user does

Same verbs. A timer whose guard was dropped never posts its event. `isograph send` cannot inject a firing: `Timer` is not an `InternalEvent`.

## Types

Most important first.

```rust
// from crates/isograph_cli/src/effect.rs
use freddie::TimerEffect;

use crate::event::IsographEvent;

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

Origin of the variant: figaro `FigaroEffect::Timer(TimerEffect<FigaroEvent>)`. Delta: the event type is `IsographEvent`.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    Timer(freddie::TimerFired),
}
```

Origin of the variant: figaro `FigaroEvent::Timer(TimerFired)`. Delta: a third variant next to `Lsp` and `Internal`, not a wire kind. `InternalEvent` is unchanged and stays the `isograph/event` params. `freddie::TimerFired` is not `Clone` (`TimerId` is `Copy`); drop `Clone` from the derive and implement it by copying the id.

```rust
// from crates/isograph_cli/src/event.rs
impl Clone for IsographEvent {
    fn clone(&self) -> Self {
        match self {
            Self::Lsp(lsp) => lsp.clone().to(),
            Self::Internal(internal) => internal.clone().to(),
            Self::Timer(freddie::TimerFired(id)) => freddie::TimerFired(*id).to(),
        }
    }
}

pub fn timer_fired(id: freddie::TimerId) -> IsographEvent {
    freddie::TimerFired(id).to()
}
```

`timer_fired` is the closure `timer_effect_and_guard` takes. Figaro's tests name that function `fired`.

The level that wants the timer holds the guard as a field. Dropping the field cancels it. `TimerGuard::trigger()` matches that firing and no other. This slice adds no field; `handle` of `TimerFired` is no effects. A consumer stores the guard, returns `IsographEffect::Timer`, and matches with `trigger()`.

## Change 1: the `freddie` dep, the types, `handle` of `TimerFired`

```toml
# from crates/isograph_cli/Cargo.toml (before)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
```

```toml
# from crates/isograph_cli/Cargo.toml (after)
freddie = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "2cf07439ce57fd55079b89128b2f22603ef8d1a8" }
```

The three git revs stay equal.

`event.rs` and `effect.rs` as in Types. `InternalEvent` does not gain `Timer`.

```rust
// from crates/isograph_cli/src/state.rs (before)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
    }
```

```rust
// from crates/isograph_cli/src/state.rs (after)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(state, internal),
        IsographEvent::Timer(fired) => handle_timer(state, fired),
    }
```

```rust
// from crates/isograph_cli/src/state.rs
fn handle_timer<THostLanguage: HostLanguage>(
    _state: &mut IsographState<THostLanguage>,
    _fired: freddie::TimerFired,
) -> Vec<IsographEffect> {
    Vec::new()
}
```

## Change 2: the effect loop schedules the timer

Origin of `schedule_timer` and of `perform` taking `event_tx`: figaro `src/daemon.rs`. Delta: `IsographEvent` / `IsographEffect`; i2's `perform` gains `event_tx` (figaro's `perform_effect` already had it).

```rust
// from crates/isograph_cli/src/daemon.rs (before)
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx.clone()) => {}
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
    tokio::select! {
        () = run_event_loop(state, event_rx, effect_tx) => {}
        () = run_effect_loop(effect_rx, event_tx.clone()) => {}
        () = crate::lsp_socket::accept_loop(listener, event_tx.clone()) => {}
```

```rust
// from crates/isograph_cli/src/daemon.rs (before)
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
        IsographEffect::SendLspResponse(respond) => {
            let _ = respond
                .reply
                .send(lsp_server::Message::Response(respond.response));
            ControlFlow::Continue(())
        }
    }
}
```

```rust
// from crates/isograph_cli/src/daemon.rs (after)
pub(crate) async fn run_effect_loop(
    mut effect_rx: UnboundedReceiver<IsographEffect>,
    event_tx: UnboundedSender<IsographEvent>,
) {
    while let Some(effect) = effect_rx.recv().await {
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
fn schedule_timer(timer: TimerEffect<IsographEvent>, event_tx: &UnboundedSender<IsographEvent>) {
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

`daemon.rs` gains `use freddie::{AlwaysEqual, TimerEffect};` and `use tokio::sync::oneshot::error::TryRecvError;`. `IsographEvent` is already imported.

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

## Tests

Park on `event_rx.recv().await`. Absence of a firing has no edge: inject `HelloWorld` after the cancel, `recv().await`, that event is `HelloWorld`. If the timer had posted, it would have arrived first.

```rust
// from crates/isograph_cli/src/daemon.rs
    #[tokio::test]
    async fn a_timer_effect_fires_its_event_after_a_zero_delay() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) =
            freddie::timer_effect_and_guard(Duration::ZERO, crate::event::timer_fired);
        let expected = match &timer.event {
            IsographEvent::Timer(freddie::TimerFired(id)) => *id,
            other => panic!("timer_effect_and_guard built TimerFired, got {other:?}"),
        };
        assert!(perform(IsographEffect::Timer(timer), event_tx.reference()).is_continue());
        let event = event_rx.recv().await.expect("the timer fired");
        let IsographEvent::Timer(freddie::TimerFired(id)) = event else {
            panic!("the timer posts TimerFired");
        };
        assert_eq!(id, expected);
        drop(guard);
    }

    #[tokio::test]
    async fn dropping_the_guard_before_perform_does_not_fire() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) =
            freddie::timer_effect_and_guard(Duration::ZERO, crate::event::timer_fired);
        drop(guard);
        let _ = perform(IsographEffect::Timer(timer), event_tx.reference());
        event_tx
            .send(InternalEvent::HelloWorld.to())
            .expect("the test sends HelloWorld");
        let event = event_rx.recv().await.expect("HelloWorld arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::HelloWorld)
        ));
    }

    #[tokio::test]
    async fn dropping_the_guard_after_perform_does_not_fire() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) =
            freddie::timer_effect_and_guard(Duration::from_secs(60), crate::event::timer_fired);
        let _ = perform(IsographEffect::Timer(timer), event_tx.reference());
        drop(guard);
        event_tx
            .send(InternalEvent::HelloWorld.to())
            .expect("the test sends HelloWorld");
        let event = event_rx.recv().await.expect("HelloWorld arrives");
        assert!(matches!(
            event,
            IsographEvent::Internal(InternalEvent::HelloWorld)
        ));
    }
```

`Duration::ZERO` is the degenerate delay. The 60s delay is only to make a late fire lose to `HelloWorld` if cancel failed. `daemon.rs` tests gain `use super::perform`, `use std::time::Duration`, and `IsographEvent` next to `InternalEvent`. `state.rs` tests gain `use std::time::Duration`.

```rust
// from crates/isograph_cli/src/state.rs
    #[test]
    fn timer_fired_with_no_guard_returns_no_effects() {
        let mut state = IsographState::<TypeScriptHostLanguage>::default();
        let (guard, timer) =
            freddie::timer_effect_and_guard(Duration::ZERO, crate::event::timer_fired);
        drop(guard);
        let event = timer.event;
        let effects = handle(&mut state, event);
        assert!(effects.is_empty());
    }
```

## Call sites

- `handle` of `TimerFired` (unmatched: empty)
- `perform` of `Timer` -> `schedule_timer` -> `event_tx.send` of the baked event
- `serve` / `listen_and_reply` / `kill_ends_the_effect_loop` pass `event_tx` into `run_effect_loop`
- `lsp_request_returns_method_not_found` passes `event_tx` into `perform`

## Design doc

`docs-website/docs/design-docs/event-model.md` gains `Timer(TimerFired)` on `IsographEvent` and `Timer(TimerEffect<IsographEvent>)` on `IsographEffect`. `TimerFired` is `freddie::TimerFired`. It is not on `InternalEvent`. The wire stays `InternalEvent`. `handle` of `Timer` is unmatched until a node holds that guard; unmatched is no effects. `handle` does not sleep. Performing `Timer` is `schedule_timer`.
