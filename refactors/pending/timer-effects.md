# Timer effects

Requires event-loop.md (landed) and freddie-timer.md. Independent of lsp-tokens.md. lsp-diagnostics.md is a consumer of this.

Incorporation of `freddie_timer` into i2. HelloWorld arms a `Duration::ZERO` timer with payload `HelloWorldDebounced`. `handle` of the matching fire returns `LogHelloWorld`. lsp-diagnostics.md deletes that payload and the guard field.

Origin: figaro `src/daemon.rs` `schedule_timer` / `perform_effect` taking `event_tx`. Origin of the halves: freddie-timer.md. Delta: `IsographEvent::Timer` / `IsographEffect::Timer`; `perform` gains `event_tx`; payload `HelloWorldDebounced`.

One shippable change.

## What the user does

Same verbs. `isograph send` of HelloWorld still logs `hello world`. The log arrives after the timer fires (`Duration::ZERO` in this slice). A second HelloWorld before fire logs once.

## Types

The crate is freddie-timer.md. i2 does not enable `bind`. No `freddie` dep.

```rust
// from crates/isograph_cli/src/event.rs
#[derive(Clone, Copy, Debug)]
pub enum TimerPayload {
    HelloWorldDebounced,
}

#[derive(Clone, Debug, derive_more::From)]
pub enum IsographEvent {
    #[from]
    Lsp(LspEvent),
    #[from]
    Internal(InternalEvent),
    #[from]
    ClientConnection(ClientConnectionEvent),
    Timer(freddie_timer::TimerFired<TimerPayload>),
}
```

`Timer` is not on `InternalEvent`. The wire is unchanged. `TimerFired<TimerPayload>` is Copy.

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
        let (guard, effect) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::HelloWorldDebounced,
            IsographEvent::Timer,
        );
        state.hello_world_debounce = guard.wrap_some();
        IsographEffect::Timer(effect).wrap_vec()
```

Overwrite assigns a new guard onto the field. That drops the old one, so `select!` cancels that sleep.

## Change 1: isograph uses `freddie_timer`

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

The rev is the commit that landed freddie-timer.md, all three equal.

`empty()` fills `hello_world_debounce: None`.

```rust
// from crates/isograph_cli/src/state.rs (before)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(&mut state.db, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::ClientConnection(ClientConnectionEvent::Started(started)) => {
```

```rust
// from crates/isograph_cli/src/state.rs (after)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(&mut state.db, lsp),
        IsographEvent::Internal(InternalEvent::HelloWorld) => handle_hello_world(state),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::Timer(fired) => handle_timer_fired(state, fired),
        IsographEvent::ClientConnection(ClientConnectionEvent::Started(started)) => {
```

`handle_internal` loses the `HelloWorld` arm. HelloWorld writes the guard on `State`.

```rust
// from crates/isograph_cli/src/state.rs
fn handle_hello_world<THostLanguage: HostLanguage>(
    state: &mut State<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) = freddie_timer::timer_effect_and_guard(
        Duration::ZERO,
        TimerPayload::HelloWorldDebounced,
        IsographEvent::Timer,
    );
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

`state.rs` gains `use std::time::Duration` and `use crate::event::TimerPayload`.

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
            freddie_timer::timer_effect_and_guard(
                Duration::ZERO,
                TimerPayload::HelloWorldDebounced,
                IsographEvent::Timer,
            );
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
            freddie_timer::timer_effect_and_guard(
                Duration::ZERO,
                TimerPayload::HelloWorldDebounced,
                IsographEvent::Timer,
            );
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
            TimerPayload::HelloWorldDebounced,
            IsographEvent::Timer,
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

`Duration::ZERO` is the degenerate delay. The 60s delay is only to make a late fire lose to `Quit` if cancel failed. `daemon.rs` tests gain `use super::perform`, `use std::time::Duration`, `IsographEvent`, `TimerPayload`. `lsp_isograph_event_hello_world_returns_log_hello_world` becomes a timer, same as `hello_world_returns_a_timer`.

Keep `guard` alive until after `trigger_if_matching` in the fire test.

## Call sites

- `handle` of HelloWorld -> `Timer` effect plus guard
- `handle` of `TimerFired` -> `trigger_if_matching` -> `LogHelloWorld` or empty
- `perform` of `Timer` -> `schedule_timer` -> `event_tx.send` of the baked event, or cancel
- `serve` / `listen_and_reply` / `kill_ends_the_effect_loop` pass `event_tx` into `run_effect_loop`
- `lsp_request_returns_method_not_found` passes `event_tx` into `perform`

## Design doc

`docs-website/docs/design-docs/event-model.md` gains `Timer(TimerFired<TimerPayload>)` on `IsographEvent` and `Timer(TimerEffect<IsographEvent>)` on `IsographEffect`. It is not on `InternalEvent`. `handle` of `Timer` uses `trigger_if_matching`. `handle` does not sleep. Performing `Timer` is `schedule_timer`.
