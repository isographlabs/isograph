# Scoped stack

A new crate, `scoped_stack`, holds a stack whose entries come off by scope, not by hand. Callers never touch the stack directly: they hold a `Frame`, a borrow of the stack with a restore point, and the frame's `Drop` truncates back to that point. There is no `pop`. Pushing is either scoped to a closure (`with`) or plain (`push`), and a plain push comes off when the pushing frame's scope ends. A callee therefore cannot disturb what its caller pushed: the only reachable mutations sit above the callee's own restore point.

This is a prefactor for raw-items.md, whose matcher is the first consumer, and it generalizes iso1's recursion-with-a-path pattern, surveyed below.

## The API

The whole crate:

```rust
// from crates/scoped_stack/src/lib.rs
/// A borrow of the stack that a function walks with: `all` reads every entry, the
/// caller's frames included, and the mutations are `push` and the scoped `with`.
/// There is no `pop`; entries come off when the frame that pushed them drops.
#[derive(Debug)]
pub struct Frame<'a, T> {
    stack: &'a mut Vec<T>,
    /// Drop truncates to this, so everything this frame pushed comes off however
    /// the scope exits, a panic included.
    restore: usize,
}

impl<T> Drop for Frame<'_, T> {
    fn drop(&mut self) {
        self.stack.truncate(self.restore);
    }
}

impl<T> Frame<'_, T> {
    /// Every entry on the stack, the caller's entries included, innermost last.
    pub fn all(&self) -> &[T] {
        self.stack
    }

    /// The entry stays for the rest of this frame's scope.
    pub fn push(&mut self, value: T) {
        self.stack.push(value);
    }

    /// A restore point without a push: whatever the closure pushes comes off when
    /// the child frame drops.
    pub fn scope<R>(&mut self, do_stuff: impl FnOnce(&mut Frame<'_, T>) -> R) -> R {
        let mut child = Frame {
            restore: self.stack.len(),
            stack: &mut *self.stack,
        };
        do_stuff(&mut child)
    }

    /// The `with_` bracketing pattern from iso1's peekable lexer: the value is on
    /// the stack exactly for the duration of the closure.
    pub fn with<R>(&mut self, value: T, do_stuff: impl FnOnce(&mut Frame<'_, T>) -> R) -> R {
        self.scope(|frame| {
            frame.push(value);
            do_stuff(frame)
        })
    }
}

/// The owner. It exists to mint the root frame; every read and write goes through a
/// [`Frame`].
#[derive(Debug, Default)]
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack(Vec::new())
    }

    pub fn frame(&mut self) -> Frame<'_, T> {
        Frame {
            restore: self.0.len(),
            stack: &mut self.0,
        }
    }
}
```

```toml
# from crates/scoped_stack/Cargo.toml
[package]
name = "scoped_stack"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]

[lints]
workspace = true
```

No slice type can replace `Frame`: `&mut [T]` permits overwriting existing entries but cannot grow past a possible reallocation, and the permission set we need is the inverse — grow freely, never touch what sits below. Hence a wrapper that exposes only the operations it is willing to allow. `Frame` has no `DerefMut`, no `IndexMut`, and no accessor handing out `&mut Vec`, so the entries below `restore` are unreachable for mutation, and nesting works through the reborrow in `scope`.

## The use cases

The API is sized against every stack-shaped push/pop site in iso1 plus the i2 matcher.

- The i2 matcher (raw-items.md): `parse_bracketed` holds a `BracketKind` on the stack for the duration of the recursion into `parse_items`, and close-bracket classification reads the whole stack. `with` and `all().contains(..)` cover it.
- iso1 `reader_ast.rs` (`generate_reader_ast_with_path` and `refetched_paths_with_path`, four sites): each pushes a `NormalizationKey`, recurses, and pops, with whole-path reads via `path.clone()` inside the recursion. Each site is a `with`; the clone is `frame.all().to_vec()`.
- iso1 `create_merged_selection_set.rs` (`traversal_path`): the pushes live in callees — `merge_server_object_field` pushes the inline-fragment or linked-field key, `insert_client_object_selectable_into_refetch_paths` pushes the client-pointer key — and the one pop lives in the caller's object-selection arm. No closure wraps the pair, so `with` alone cannot express it; `scope` plus `push` can: the caller wraps the arm in `scope`, the callee pushes onto the frame it receives, and the arm's end restores. The restore also does not care how many entries the callee pushed.

Two stack-adjacent sites in iso1 are not use cases, and the API deliberately does not stretch to them. pico's `DependencyStack` registers dependencies from arbitrary call depth through `&Database`, so it needs shared mutation via `RefCell` rather than a threaded `&mut`, and its `leave` returns the popped value; it keeps its own guard. The peekable lexer's `semantic_tokens.pop()` removes an entry another function pushed as a side effect — an accumulator with a corrective pop, not scope discipline.

There is no `pop` because no surveyed caller needs one once scope ends do the popping, and `pop` is the one operation that could cross a frame boundary, which would drag in a floor index to police it. If a future caller needs the popped value, that is the point to add both.

## Tests

```rust
// from crates/scoped_stack/src/lib.rs
#[cfg(test)]
mod test {
    use crate::{Frame, Stack};

    #[test]
    fn with_holds_the_value_for_the_closure() {
        let mut stack = Stack::new();
        let mut frame = stack.frame();

        frame.with(1, |frame| {
            assert_eq!(frame.all(), &[1]);
        });

        assert!(frame.all().is_empty());
    }

    #[test]
    fn nested_frames_see_outer_entries() {
        let mut stack = Stack::new();
        let mut frame = stack.frame();

        frame.with(1, |frame| {
            frame.with(2, |frame| {
                assert_eq!(frame.all(), &[1, 2]);
            });
            assert_eq!(frame.all(), &[1]);
        });
    }

    #[test]
    fn scope_removes_what_the_closure_pushed() {
        let mut stack = Stack::new();
        let mut frame = stack.frame();

        frame.with(1, |frame| {
            frame.scope(|frame| {
                frame.push(2);
                frame.push(3);
                assert_eq!(frame.all(), &[1, 2, 3]);
            });
            assert_eq!(frame.all(), &[1]);
        });
    }

    #[test]
    fn a_callee_push_lasts_until_the_caller_scope_ends() {
        fn callee(frame: &mut Frame<'_, i32>) {
            frame.push(2);
        }

        let mut stack = Stack::new();
        let mut frame = stack.frame();
        frame.push(1);

        frame.scope(|frame| {
            callee(frame);
            assert_eq!(frame.all(), &[1, 2]);
        });

        assert_eq!(frame.all(), &[1]);
    }

    #[test]
    fn pushes_come_off_when_the_frame_drops() {
        let mut stack = Stack::new();

        {
            let mut frame = stack.frame();
            frame.push(1);
        }

        assert!(stack.0.is_empty());
    }

    #[test]
    fn with_returns_the_closure_result() {
        let mut stack = Stack::new();
        let mut frame = stack.frame();

        let result = frame.with(1, |frame| frame.all().len());

        assert_eq!(result, 1);
    }

    #[test]
    fn a_panic_still_restores() {
        let mut stack = Stack::new();
        let mut frame = stack.frame();
        frame.push(1);

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            frame.with(2, |_frame| panic!("unwind out of the closure"));
        }));

        assert!(panicked.is_err());
        assert_eq!(frame.all(), &[1]);
    }
}
```

## Shipping order

One change: create the crate as printed, with its tests. The workspace member glob picks it up; no other file changes. raw-items.md consumes it when raw-items lands.

## Consequences

- raw-items.md's matcher loses its bespoke `EnclosingStack`/`EnclosingFrame` and uses `scoped_stack::{Stack, Frame}`; that doc is updated alongside this one.
- iso1's `reader_ast.rs` and `create_merged_selection_set.rs` patterns have a home when their i2 equivalents get written; nothing rewrites iso1 itself.

## Landing checklist

- `cargo test -p scoped_stack` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
