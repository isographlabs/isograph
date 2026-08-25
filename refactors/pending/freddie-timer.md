# `freddie_timer`

Freddie + figaro refactor. The two halves leave `freddie` for their own crate. Figaro keeps importing from `freddie`, which re-exports. Arming takes a payload. `TimerEffect<P>` and `TimerFired<P>` share `P`. The payload is readable only through `trigger_if_matching`. The consumer's event enum is not `P`: `perform` posts `Event::Timer(timer.event)`.

timer-effects.md is the i2 incorporation. It requires this.

Origin: `freddie/crates/freddie/src/timer.rs`, `drop_guard.rs`, `always_equal.rs`. Origin of the two halves: freddie `docs-website/docs/architecture/timers.md`. Origin of figaro arming: `figaro_model` overlay / windows / typing. Delta: crate `freddie_timer`; `TimerFired<P = ()>` and `TimerEffect<P = ()>` share `P`; `timer_effect_and_guard(delay, payload)`; `TimerTrigger` behind `bind`; figaro `timer_effect_and_guard(delay, ())`.

Two changes. Crate in the freddie repo (`mercury_model` is the in-tree consumer, same arming delta, same commit). Then figaro (path dep) in lockstep.

## What the user does

Figaro is unchanged at the keys. Overlay still hides on its dwell. A placement still settles. `jk` still times out.

## Types

The crate is `freddie_timer`. Two halves from one call. `timer_effect_and_guard` mints the id, builds the oneshot pair, puts the sender in the guard and the receiver in the effect, and stores `TimerFired { id, payload }` on the effect. `P` is shared: `TimerFired<P>` and `TimerEffect<P>`.

Copy: `TimerId`, `TimerTrigger`, `TimerFired<P>` when `P: Copy`. Not Copy: `DropGuard` (owns `oneshot::Sender`), `TimerGuard` (owns `DropGuard`), `TimerEffect<P>` (owns delay, firing, cancel receiver). None of the non-Copy types are `Clone`. The guard and the effect are each used once. Overwrite of a guard field is drop of the old one.

`schedule_timer` is not in this crate. The consumer's effect loop performs `TimerEffect`.

### `timer_effect_and_guard`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
pub fn timer_effect_and_guard<P>(
    delay: Duration,
    payload: P,
) -> (TimerGuard, TimerEffect<P>) {
    let (guard, receiver) = drop_guard();
    let id = TimerId::mint();
    (
        TimerGuard { id, guard },
        TimerEffect {
            delay,
            event: TimerFired { id, payload },
            cancel: receiver,
        },
    )
}
```

Figaro: `timer_effect_and_guard(OVERLAY_DWELL, ())`. `perform` posts `FigaroEvent::Timer(timer.event)`.

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

Figaro bind uses `trigger()`.

### `TimerEffect`

```rust
// external freddie/crates/freddie_timer/src/timer.rs
#[derive(Debug)]
pub struct TimerEffect<P = ()> {
    pub delay: Duration,
    pub event: TimerFired<P>,
    pub cancel: oneshot::Receiver<()>,
}
```

Not Copy. Not Clone. Used once: the effect loop moves it into `schedule_timer`. Under `testing`, `PartialEq` is delay and firing; the receiver is a handle, not data. `P` is the payload, the same `P` as `TimerFired`.

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

Copy. `TimerFired` here is `TimerFired<()>` via the default. Figaro bind: `|path| path.overlay_timer().map(TimerGuard::trigger)`.

### Crate root

```rust
// external freddie/crates/freddie_timer/src/lib.rs
pub mod drop_guard;
pub mod timer;

pub use drop_guard::{DropGuard, drop_guard};
pub use timer::{TimerEffect, TimerFired, TimerGuard, TimerId, timer_effect_and_guard};

#[cfg(feature = "bind")]
pub use timer::TimerTrigger;
```

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

## Change 1: the crate, and `freddie` re-exports

Workspace `members` gains `"crates/freddie_timer"`. The three files are Types.

```rust
// external freddie/crates/freddie/src/lib.rs
pub use freddie_timer::{
    DropGuard, TimerEffect, TimerFired, TimerGuard, TimerId, TimerTrigger, drop_guard,
    timer_effect_and_guard,
};
```

`freddie` always enables `freddie_timer/bind`, so `TimerTrigger` is always re-exported.

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

## Change 2: figaro

Figaro keeps the `freddie` dep. Bind lines are unchanged.

```rust
// external figaro/figaro_model/src/events.rs
    Timer(TimerFired),
```

`TimerFired` is `TimerFired<()>`. Bind still passes `&TimerFired`. `TimerFired` is Copy, so those handlers take `#[expect(clippy::trivially_copy_pass_by_ref)]`.

```rust
// external figaro/figaro_model/src/effect.rs
    Timer(TimerEffect),
```

```rust
// external figaro/figaro_model/src/model/mod.rs (after)
            timer_effect_and_guard(OVERLAY_DWELL, ());
```

Same at `windows.rs` `PLACEMENT_SETTLE` and `typing/mod.rs` `arm_window`.

```rust
// external figaro/src/daemon.rs
            () = tokio::time::sleep(delay) => { let _ = event_tx.send(FigaroEvent::Timer(event)); }
```

Tests: a firing is `FigaroEvent::Timer(timer.event)`. `mercury_model` does the same arming, effect type, and `schedule_timer` wrap, in the crate commit.
