# `freddie_timer`

The two halves leave `freddie` for their own crate. Figaro and mercury keep importing from `freddie`, which re-exports. Arming takes a payload and an event constructor. The payload is readable only through `trigger_if_matching`.

timer-effects.md is the isograph consumer. It requires this.

Origin: `freddie/crates/freddie/src/timer.rs`, `drop_guard.rs`, `always_equal.rs`. Origin of the two halves: freddie `docs-website/docs/architecture/timers.md`. Origin of figaro arming: `figaro_model` overlay / windows / typing. Delta: crate `freddie_timer`; `TimerFired<P = ()>`; `timer_effect_and_guard(delay, payload, event)`; `TimerTrigger` behind `bind`; figaro and mercury `(), Event::Timer`.

One shippable change. Lands in the freddie repo, then figaro (path dep) in lockstep.

## What the user does

Figaro is unchanged at the keys. Overlay still hides on its dwell. A placement still settles. `jk` still times out.

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

Figaro: `timer_effect_and_guard(OVERLAY_DWELL, (), FigaroEvent::Timer)`. The enum variant is `fn(TimerFired<P>) -> Event`.

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

Copy. `TimerFired` here is `TimerFired<()>` via the default. Figaro bind: `|path| path.overlay_timer().map(TimerGuard::trigger)`.

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

## Change 2: figaro

Figaro keeps the `freddie` dep. Bind lines are unchanged. `schedule_timer` is unchanged.

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

Same replacement at `windows.rs` `PLACEMENT_SETTLE` and `typing/mod.rs` `arm_window`.

Tests: `fired` / `timer_id` cannot read a private id. A test that needs the firing uses the `event` on the `Timer` effect it just got (`m.handle(&timer.event)`), not a reconstructed `TimerFired(id)`. Delete `fired` and `timer_id`. Every `m.handle(&fired(timer_id(&fx)))` becomes that.

## Change 3: mercury

Mercury in the freddie repo: the same arming and test change as figaro, same commit as the crate.
