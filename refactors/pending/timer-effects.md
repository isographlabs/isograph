# Timer effects

Requires event-loop.md (landed), freddie-timer.md, and lsp-outstanding.md. lsp-outstanding.md must be past before this ships. Independent of lsp-tokens.md. lsp-diagnostics.md is a consumer of this.

Incorporation of `freddie_timer` into i2. HelloWorld arms a `Duration::ZERO` timer with payload `HelloWorldDebounced`. `handle` of the matching fire returns `LogHelloWorld`. lsp-diagnostics.md deletes that payload and the guard field.

`IsographDb` is the pico database in `isograph_compiler`. `IsographState` is the CLI process wrapper (`db`, `live_sessions`, and this slice's guard). The guard is not a pico source. It is RAII over a oneshot that cancels a tokio sleep. It lives on `IsographState`. `handle` matches `IsographEvent` only. `Internal` is `handle_internal_event(state, internal)`. That function takes `IsographState` and matches `InternalEvent`. DiskChanged still calls `handle_disk_changed(&mut state.db, change)`.

`isograph send` of HelloWorld is `Lsp(Notification { method: isograph/event, params: HelloWorld })`. `dispatch_lsp_notification` registers `handle_internal_event` on that method. In-process `Internal` is the same function.

Origin: figaro `src/daemon.rs` `schedule_timer` / `perform_effect` taking `event_tx`. Origin of the halves: freddie-timer.md. Delta: `IsographEvent::Timer` / `IsographEffect::Timer`; `perform` gains `event_tx` and the Timer arm returns `ControlFlow::Continue`; payload `HelloWorldDebounced`; `isograph/event` registers `handle_internal_event`.

One shippable change.

## What the user does

Same verbs. `isograph send` of HelloWorld still logs `hello world`. The log arrives after the timer fires (`Duration::ZERO` in this slice). `Duration::ZERO` plus two send processes does not log once: each send is a full LSP handshake, and the ZERO timer fires as soon as the runtime polls the spawn, which is long before a second CLI send exists. Overwrite-of-guard is the handle-only test `a_second_hello_world_drops_the_first_guard`. `two_sends_then_stop_exits_without_force` still asserts the log contains `hello world`, not that it appears once.

## Types

The crate is freddie-timer.md. i2 does not enable `bind`. No `freddie` dep. `TimerFired<TimerPayload>` and `TimerEffect<TimerPayload>` share `TimerPayload`. `perform` posts `IsographEvent::Timer(timer.event)`.

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
    Timer(TimerEffect<TimerPayload>),
}
```

```rust
// from crates/isograph_cli/src/state.rs
pub struct IsographState<THostLanguage: HostLanguage> {
    pub db: IsographDb<THostLanguage>,
    pub live_sessions: LiveSessions,
    hello_world_debounce: Option<freddie_timer::TimerGuard>,
}
```

`IsographState` has no `Default`. Every `IsographState { ... }` writes `hello_world_debounce: None`:

- `crates/isograph_cli/src/state.rs` `empty` (and `with_config`, which calls `empty`)
- `crates/isograph_cli/src/daemon.rs` `serve`
- `crates/isograph_cli/src/daemon.rs` tests `empty_state`
- `crates/isograph_cli/src/lsp_socket.rs` `listen_and_reply`
- `crates/isograph_cli/src/watch.rs` `Harness::new`

Arming is figaro's pair. Overwrite assigns a new guard onto the field. That drops the old one, so `select!` cancels that sleep.

```rust
// from crates/isograph_cli/src/state.rs
        let (guard, effect) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::HelloWorldDebounced,
        );
        state.hello_world_debounce = guard.wrap_some();
        IsographEffect::Timer(effect).wrap_vec()
```

## Change 1: isograph uses `freddie_timer`

`785c7950b009309eee8b3810e078876b77d2015e` is `freddie_cli` / `freddie_single_instance` only. The crate appears at `38df736` as the 3-arg API (`timer_effect_and_guard(delay, payload, Event::Timer)`, `TimerEffect<E> { event: E }`). The 2-arg API this slice uses (`timer_effect_and_guard(delay, payload)`, `TimerEffect<P> { event: TimerFired<P> }`) is `e2dde74d6560e65705fdc796663fe32f6f1b8ba0`, the commit that landed freddie-timer.md. This slice cannot start until that rev is on `origin` of `https://github.com/freddiehg/freddie`. All three git deps use that rev.

```toml
# from crates/isograph_cli/Cargo.toml (before)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "785c7950b009309eee8b3810e078876b77d2015e" }
```

```toml
# from crates/isograph_cli/Cargo.toml (after)
freddie_cli = { git = "https://github.com/freddiehg/freddie", rev = "e2dde74d6560e65705fdc796663fe32f6f1b8ba0" }
freddie_single_instance = { git = "https://github.com/freddiehg/freddie", rev = "e2dde74d6560e65705fdc796663fe32f6f1b8ba0" }
freddie_timer = { git = "https://github.com/freddiehg/freddie", rev = "e2dde74d6560e65705fdc796663fe32f6f1b8ba0" }
```

`dropping_the_guard_after_perform_does_not_fire` uses `tokio::time::pause` / `advance`. Those need `test-util`, which `tokio` `full` does not include. Add it on the test build only:

```toml
# from crates/isograph_cli/Cargo.toml (dev-dependencies, after)
tokio = { workspace = true, features = ["rt", "macros", "sync", "time", "test-util"] }
```

After lsp-outstanding, `handle` and `dispatch_lsp` already take `&mut IsographState`. `dispatch_lsp_notification` already takes `&mut IsographState`. `on_isograph_event` peels `db` into `handle_internal`.

```rust
// from crates/isograph_cli/src/state.rs (before)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal(&mut state.db, internal),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
```

```rust
// from crates/isograph_cli/src/state.rs (after)
    match event {
        IsographEvent::Lsp(lsp) => dispatch_lsp(state, lsp),
        IsographEvent::Internal(internal) => handle_internal_event(state, internal),
        IsographEvent::Timer(fired) => handle_timer_fired(state, fired),
        IsographEvent::ClientConnection(client) => handle_client_connection(state, client),
    }
```

```rust
// from crates/isograph_cli/src/state.rs (before)
fn on_isograph_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    handle_internal(&mut state.db, internal)
}

fn handle_internal<THostLanguage: HostLanguage>(
    state: &mut IsographDb<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => IsographEffect::LogHelloWorld.wrap_vec(),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(state, change),
    }
}
```

`LSPNotificationDispatch` is already generic over `TState`. The handler is `fn(&mut TState, Params) -> Vec<IsographEffect>`. `IsographEventNotification::Params` is `InternalEvent`, the same as `handle_internal_event`.

```rust
// from crates/isograph_cli/src/state.rs (after)
fn dispatch_lsp_notification<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    notification: lsp_server::Notification,
) -> Vec<IsographEffect> {
    let dispatch = || {
        crate::lsp_notification_dispatch::LSPNotificationDispatch::new(notification, state)
            .on_notification_sync::<crate::lsp_socket::IsographEventNotification>(
                handle_internal_event::<THostLanguage>,
            )?
            .notification();
        ControlFlow::Continue(())
    };
    match dispatch() {
        ControlFlow::Break(effects) => effects,
        ControlFlow::Continue(()) => no_effects(),
    }
}

fn handle_internal_event<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    internal: InternalEvent,
) -> Vec<IsographEffect> {
    match internal {
        InternalEvent::HelloWorld => handle_hello_world(state),
        InternalEvent::Quit => IsographEffect::Kill.wrap_vec(),
        InternalEvent::DiskChanged(change) => handle_disk_changed(&mut state.db, change),
    }
}

fn handle_hello_world<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
) -> Vec<IsographEffect> {
    let (guard, effect) = freddie_timer::timer_effect_and_guard(
        Duration::ZERO,
        TimerPayload::HelloWorldDebounced,
    );
    state.hello_world_debounce = guard.wrap_some();
    IsographEffect::Timer(effect).wrap_vec()
}

fn handle_timer_fired<THostLanguage: HostLanguage>(
    state: &mut IsographState<THostLanguage>,
    fired: freddie_timer::TimerFired<TimerPayload>,
) -> Vec<IsographEffect> {
    match fired.trigger_if_matching(state.hello_world_debounce.as_ref()) {
        Some(&TimerPayload::HelloWorldDebounced) => {
            state.hello_world_debounce = None;
            IsographEffect::LogHelloWorld.wrap_vec()
        }
        None => no_effects(),
    }
}
```

`state.rs` gains `use std::time::Duration` and `use crate::event::TimerPayload`. Payload is readable only through `trigger_if_matching`.

Origin of `schedule_timer` and of `perform` taking `event_tx`: figaro `src/daemon.rs`. Figaro's `perform_effect` matches, then returns `Continue` after the match (`Kill` is `return Break`). i2's `perform` returns from each arm. `schedule_timer` is `-> ()`. `IsographEffect::Timer(timer) => schedule_timer(timer, event_tx)` is a type error. The Timer arm calls `schedule_timer` then returns `Continue`.

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
        IsographEffect::Timer(timer) => {
            schedule_timer(timer, event_tx);
            ControlFlow::Continue(())
        }
    }
}

/// Schedule `timer`: fire its event after its delay, unless the guard the model kept drops first.
///
/// Fire-and-forget on the runtime, so a pending sleep runs off the effect loop.
fn schedule_timer(timer: TimerEffect<TimerPayload>, event_tx: &UnboundedSender<IsographEvent>) {
    let TimerEffect {
        delay,
        event,
        mut cancel,
    } = timer;
    // If the guard dropped before the loop reached this, the receiver is closed and the timer is
    // already cancelled, so there is nothing to spawn.
    if matches!(cancel.try_recv(), Err(TryRecvError::Closed)) {
        return;
    }
    let event_tx = event_tx.clone();
    tokio::spawn(async move {
        tokio::select! {
            () = tokio::time::sleep(delay) => { let _ = event_tx.send(IsographEvent::Timer(event)); }
            _ = cancel => {}
        }
    });
}
```

The effect carries `TimerFired<TimerPayload>`. The wrap to `IsographEvent::Timer` is in `schedule_timer`. `event_tx` is cloned into the spawn; the receiver stays on the worker.

`daemon.rs` gains `use freddie_timer::TimerEffect;` and `use tokio::sync::oneshot::error::TryRecvError`.

Callers of `run_effect_loop` and `perform` pass `event_tx`:

- `serve`'s `select!`: `event_tx.clone()`
- `lsp_socket.rs` `listen_and_reply`: clone `loop_tx` before the forwarder spawn, pass that clone
- `kill_ends_the_effect_loop`: an `event_tx` the test does not read
- every `perform` of `SendLspResponse` in `state.rs` tests: an `event_tx` the test does not read

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

```rust
// from crates/isograph_cli/src/lsp_socket.rs (after), next to let (loop_tx, loop_rx) = unbounded_channel();
        let effect_event_tx = loop_tx.clone();
```

and `run_effect_loop(effect_rx, effect_event_tx)`.

`sending_hello_world_emits_log_hello_world` drives the event loop only, so it now receives `Timer` not `LogHelloWorld`. Assert `Timer`. Rename it `sending_hello_world_emits_a_timer`.

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
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
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
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
        assert!(effects.is_empty());
    }

    #[test]
    fn a_second_hello_world_drops_the_first_guard() {
        let mut state = empty();
        let first = handle(&mut state, InternalEvent::HelloWorld.to());
        let second = handle(&mut state, InternalEvent::HelloWorld.to());
        let IsographEffect::Timer(first_timer) = first
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(first_timer.event));
        assert!(effects.is_empty());
        let IsographEffect::Timer(second_timer) = second
            .into_iter()
            .next()
            .expect("HelloWorld returns one Timer")
        else {
            panic!("HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(second_timer.event));
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
        assert!(state.hello_world_debounce.is_none());
    }

    #[test]
    fn lsp_isograph_event_hello_world_returns_a_timer() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            event_notification(
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        assert!(matches!(effects.as_slice(), [IsographEffect::Timer(_)]));
        assert!(state.hello_world_debounce.is_some());
    }

    #[test]
    fn lsp_isograph_event_hello_world_then_its_timer_returns_log_hello_world() {
        let mut state = empty();
        let effects = handle(
            &mut state,
            event_notification(
                serde_json::to_value(InternalEvent::HelloWorld).expect("serializes"),
            ),
        );
        let IsographEffect::Timer(timer) = effects
            .into_iter()
            .next()
            .expect("LSP HelloWorld returns one Timer")
        else {
            panic!("LSP HelloWorld returns Timer");
        };
        let effects = handle(&mut state, IsographEvent::Timer(timer.event));
        assert!(matches!(
            effects.as_slice(),
            [IsographEffect::LogHelloWorld]
        ));
    }
```

The matching test does not `perform` the timer: it feeds `timer.event` to `handle` as if the sleep had won. The stale test drops the field first. The second HelloWorld test is overwrite plus the second fire. The LSP tests are the send-shaped path through `handle_internal_event`. `lsp_isograph_event_hello_world_returns_log_hello_world` is deleted; those two replace it.

```rust
// from crates/isograph_cli/src/daemon.rs
    #[tokio::test]
    async fn a_timer_effect_fires_its_event_after_a_zero_delay() {
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::HelloWorldDebounced,
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
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::ZERO,
            TimerPayload::HelloWorldDebounced,
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
        tokio::time::pause();
        let (event_tx, mut event_rx) = unbounded_channel();
        let (guard, timer) = freddie_timer::timer_effect_and_guard(
            Duration::from_secs(60),
            TimerPayload::HelloWorldDebounced,
        );
        let _ = perform(IsographEffect::Timer(timer), event_tx.reference());
        drop(guard);
        tokio::task::yield_now().await;
        tokio::time::advance(Duration::from_secs(60)).await;
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

`Duration::ZERO` is the degenerate delay for the fire test. `dropping_the_guard_before_perform` checks the `try_recv` Closed short-circuit, not the spawned `select!`.

`dropping_the_guard_after_perform` is the spawned `select!`. Pause, perform a long delay, drop the guard, yield so the spawn polls the closed oneshot before time moves (otherwise both branches can be ready and fair `select!` can pick sleep even when cancel worked), advance past the delay, send Quit, `recv` is Quit. If cancel failed, the spawn is still on sleep; advance posts Timer; `recv` is Timer.

Keep `guard` alive until after `trigger_if_matching` in the fire test.

```rust
// from crates/isograph_cli/src/daemon.rs
    #[tokio::test]
    async fn hello_world_through_event_and_effect_loops_returns_log_hello_world() {
        let (event_tx, event_rx) = unbounded_channel();
        let (effect_tx, mut effect_rx) = unbounded_channel();
        event_tx
            .send(InternalEvent::HelloWorld.to())
            .expect("the test sends HelloWorld");
        let event_loop = run_event_loop(empty_state(), event_rx, effect_tx);
        let effect_loop = async {
            loop {
                let effect = effect_rx.recv().await.expect("effects until LogHelloWorld");
                if matches!(effect, IsographEffect::LogHelloWorld) {
                    break;
                }
                assert!(perform(effect, event_tx.reference()).is_continue());
            }
        };
        tokio::select! {
            () = event_loop => panic!("event loop ended before LogHelloWorld"),
            () = effect_loop => {}
        }
    }
```

`event_tx` is not dropped: the spawn needs it, and `run_event_loop` must still be parked for the fire. The test loop `perform`s every effect except `LogHelloWorld`; receiving that effect is the assertion. Path: HelloWorld -> Timer effect -> `schedule_timer` -> fire -> Timer event -> `LogHelloWorld`.

`daemon.rs` tests gain `use super::perform`, `use std::time::Duration`, `IsographEvent`, `TimerPayload`.

CLI tests in `crates/ts_graphql_react_isograph_cli/tests/cli.rs` send HelloWorld and wait for `hello world` in the log. They fail if the LSP path does not arm, perform, fire, and log.

## Call sites

- `handle_internal_event` of HelloWorld -> `Timer` effect plus guard
- `handle` of `TimerFired` -> `trigger_if_matching` -> `LogHelloWorld` or empty
- `perform` of `Timer` -> `schedule_timer` -> wrap as `IsographEvent::Timer` on `event_tx.send`, or cancel
- `serve` / `listen_and_reply` / `kill_ends_the_effect_loop` pass `event_tx` into `run_effect_loop`
- `lsp_request_returns_method_not_found` and the other `SendLspResponse` `perform`s pass `event_tx` into `perform`

## Design doc

`docs-website/docs/design-docs/event-model.md` gains `Timer(TimerFired<TimerPayload>)` on `IsographEvent` and `Timer(TimerEffect<TimerPayload>)` on `IsographEffect`. It is not on `InternalEvent`. `handle` of `Timer` uses `trigger_if_matching`. `handle` does not sleep. Performing `Timer` is `schedule_timer`, which posts `IsographEvent::Timer(timer.event)`.
