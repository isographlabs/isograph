# Timer effects

Requires event-loop.md (landed). Independent of lsp-tokens.md. lsp-diagnostics.md is a consumer of this.

Figaro's two halves: `timer_effect_and_guard` returns a `TimerGuard` the node holds and a `TimerEffect` the handler returns. The effect loop schedules the sleep. Dropping the guard cancels it (`select!` takes the cancel branch; the event is not sent). A sleep that already posted cannot be unsent; `handle` of that firing matches nothing.

That code lives in `freddie` (`timer.rs`, `drop_guard.rs`, `always_equal.rs`) and pulls `bind` for `TimerTrigger`. i2 uses the same two halves, from a crate that does not pull `bind`. `TimerFired` is generic over a payload. The payload is readable only through `trigger_if_matching`.

Two shippable changes. Freddie first, then isograph bumps the git rev and wires `schedule_timer`.

Origin: figaro `figaro_model/src/effect.rs` `FigaroEffect::Timer`, `figaro_model/src/events.rs` `FigaroEvent::Timer`, `figaro/src/daemon.rs` `schedule_timer`, `freddie/crates/freddie/src/timer.rs`. Origin of reset by overwrite: freddie `docs-website/docs/architecture/timers.md`. Delta: crate `freddie_timer` (Types); figaro and mercury arm with `(), Event::Timer`; i2 payload `HelloWorldDebounced` (deleted when lsp-diagnostics.md lands).

## What the user does

Same verbs. `isograph send` of HelloWorld still logs `hello world`. The log arrives after the timer fires (`Duration::ZERO` in this slice). A second HelloWorld before fire logs once.

## Types

The crate is `freddie_timer`. Two halves from one call. `timer_effect_and_guard` mints the id, builds the oneshot pair, puts the sender in the guard and the receiver in the effect, wraps `payload` into `TimerFired`, and hands `TimerFired` to `event` so the effect carries whatever the consumer's event enum is.

Copy: `TimerId`, `TimerTrigger`, `TimerFired<P>` when `P: Copy`. Not Copy: `DropGuard` (owns `oneshot::Sender`), `TimerGuard` (owns `DropGuard`), `AlwaysEqual<T>` (owns the cancel receiver), `TimerEffect<E>` (owns delay, event, cancel receiver). None of the non-Copy types are `Clone`. The guard and the effect are each used once. Overwrite of a guard field is drop of the old one.

`schedule_timer` is not in this crate. The consumer's effect loop performs `TimerEffect`.

### `timer_effect_and_guard`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
pub fn timer_effect_and_guard<P, E>(
    delay: Duration,
    payload: P,
    event: impl FnOnce(TimerFired<P>) -> E,
) -> (TimerGuard, TimerEffect<E>) {
    let (guard, receiver) = drop_guard();
    let id = TimerId::mint();
    (
        TimerGuard { id, guard },
        TimerEffect {
            delay,
            event: event(TimerFired { id, payload }),
            cancel: AlwaysEqual(receiver),
        },
    )
}
```

Figaro: `timer_effect_and_guard(OVERLAY_DWELL, (), FigaroEvent::Timer)`. i2: `timer_effect_and_guard(Duration::ZERO, TimerPayload::HelloWorldDebounced, IsographEvent::Timer)`. The enum variant is `fn(TimerFired<P>) -> Event`.

`TimerFired` is not constructed outside this function. There is no public `new`.

### `TimerGuard`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[must_use = "dropping the guard cancels the timer immediately"]
#[derive(Debug)]
pub struct TimerGuard {
    id: TimerId,
    #[expect(dead_code)]
    guard: DropGuard,
}
```

Not Copy. Not Clone. Drop closes the oneshot, which completes the cancel branch of `schedule_timer`. `id` is private.

```rust
// external freddie/crates/freddie_timer/src/timer.rs
impl TimerGuard {
    #[must_use]
    #[cfg(feature = "bind")]
    pub const fn trigger(&self) -> TimerTrigger {
        TimerTrigger(self.id)
    }
}
```

Figaro bind uses `trigger()`. i2 does not enable `bind`.

### `TimerEffect`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[cfg_attr(feature = "testing", derive(PartialEq, Eq))]
#[derive(Debug)]
pub struct TimerEffect<E> {
    pub delay: Duration,
    pub event: E,
    pub cancel: AlwaysEqual<oneshot::Receiver<()>>,
}
```

Not Copy. Not Clone. Used once: the effect loop moves it into `schedule_timer`. Under `testing`, equality is delay and event; `AlwaysEqual` makes the receiver compare equal.

### `TimerFired`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[derive(Clone, Copy, Debug)]
pub struct TimerFired<P = ()> {
    id: TimerId,
    payload: P,
}

impl<P> TimerFired<P> {
    pub fn trigger_if_matching<'a>(&'a self, guard: Option<&TimerGuard>) -> Option<&'a P> {
        guard
            .is_some_and(|guard| guard.id == self.id)
            .then_some(&self.payload)
    }
}
```

Copy when `P: Copy`. `()` is Copy, so figaro's `TimerFired` is Copy. `id` and `payload` are private. The only read of `payload` is this method. Default `P = ()` so figaro writes `TimerFired` and `FigaroEvent::Timer(TimerFired)`.

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[cfg(feature = "testing")]
impl<P> PartialEq for TimerFired<P> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(feature = "testing")]
impl<P> Eq for TimerFired<P> {}
```

A test that rebuilds an expected `Timer` effect cannot know the minted id. Equality ignores it and the payload. A test that cares about a firing uses the `event` on the effect that set the timer, or `trigger_if_matching`.

### `TimerId`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TimerId(u64);

impl TimerId {
    fn mint() -> Self {
        static NEXT: AtomicU64 = AtomicU64::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }
}
```

Copy. The field is private. The only mint is inside `timer_effect_and_guard`.

### `DropGuard`

```rust
// external freddie/crates/freddie_timer/src/drop_guard.rs
#[must_use = "dropping the guard cancels immediately"]
pub struct DropGuard(#[expect(dead_code)] oneshot::Sender<()>);

impl std::fmt::Debug for DropGuard {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DropGuard")
    }
}

pub fn drop_guard() -> (DropGuard, oneshot::Receiver<()>) {
    let (sender, receiver) = oneshot::channel();
    (DropGuard(sender), receiver)
}
```

Not Copy. Not Clone. Timer's cancel mechanism; not timer-specific besides that.

### `AlwaysEqual`

```rust
// external freddie/crates/freddie_timer/src/always_equal.rs
pub struct AlwaysEqual<T>(pub T);

impl<T> std::fmt::Debug for AlwaysEqual<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("AlwaysEqual")
    }
}

#[cfg(feature = "testing")]
impl<T> PartialEq for AlwaysEqual<T> {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

#[cfg(feature = "testing")]
impl<T> Eq for AlwaysEqual<T> {}
```

Not Copy. Not Clone unless `T` is and we add those derives; we do not. The cancel receiver is not data.

### `TimerTrigger`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[cfg(feature = "bind")]
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct TimerTrigger(TimerId);

#[cfg(feature = "bind")]
impl EventTrigger for TimerTrigger {
    type Event = TimerFired;
    fn is_matching(&self, ev: &TimerFired) -> bool {
        self.0 == ev.id
    }
}
```

Copy. `TimerFired` here is `TimerFired<()>` via the default. Figaro bind: `|path| path.overlay_timer().map(TimerGuard::trigger)`. i2 does not compile this.

### Crate root

```rust
// external freddie/crates/freddie_timer/src/lib.rs
pub mod always_equal;
pub mod drop_guard;
pub mod timer;

pub use always_equal::AlwaysEqual;
pub use drop_guard::{DropGuard, drop_guard};
pub use timer::{TimerEffect, TimerFired, TimerGuard, TimerId, timer_effect_and_guard};

#[cfg(feature = "bind")]
pub use timer::TimerTrigger;
```

i2 events:

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
    Client(ClientConnectionEvent),
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

## Change 1: `freddie_timer`

New crate `crates/freddie_timer`. Workspace member. The three files are Types. `crates/freddie` depends on it and re-exports. Figaro and mercury keep importing from `freddie`. Arming and tests in both are this change.

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

The three files are Types. Delta from today's `freddie` copies: `TimerFired<P = ()>`; `trigger_if_matching`; `timer_effect_and_guard` takes `payload` and `FnOnce(TimerFired<P>) -> E`; `TimerTrigger` behind `bind`.

Figaro keeps `freddie` as the dep and writes:

```rust
// external figaro/figaro_model/src/events.rs
    Timer(TimerFired),
```

`TimerFired` is `TimerFired<()>`. Handlers stay `_ev: &TimerFired`.

```rust
// external figaro/figaro_model/src/model/mod.rs (before)
            timer_effect_and_guard(OVERLAY_DWELL, |id| FigaroEvent::Timer(TimerFired(id)));
```

```rust
// external figaro/figaro_model/src/model/mod.rs (after)
            timer_effect_and_guard(OVERLAY_DWELL, (), FigaroEvent::Timer);
```

Same replacement at `windows.rs` `PLACEMENT_SETTLE` and `typing/mod.rs` `arm_window`. Bind lines are unchanged (`TimerGuard::trigger()`). `schedule_timer` is unchanged.

Tests: `fired` / `timer_id` cannot read a private id. A test that needs the firing uses the `event` on the `Timer` effect it just got (`m.handle(&timer.event)`), not a reconstructed `TimerFired(id)`. Delete `fired` and `timer_id`. Every `m.handle(&fired(timer_id(&fx)))` becomes that.

Mercury in the freddie repo: the same arming and test change, same commit as the crate.

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
