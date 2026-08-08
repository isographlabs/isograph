# Scoped stack

A new crate, `scoped_stack`, holds a stack whose only mutation is a scoped push, so everything pushed is popped by construction. `temp_push` pushes an item and returns a guard; the guard is the only usable handle to the stack while it lives, and its `Drop` pops the item. `with_pushed` is the closure form of the same thing: the item is on the stack exactly for the duration of the closure. There is no bare `push` and no `pop`; the point of the structure is to add items for a scope, and the scope's end is the only way they come off.

This is a prefactor for raw-items.md, whose matcher is the first consumer, and it generalizes iso1's recursion-with-a-path pattern, surveyed below.

## The API

The whole crate:

```rust
// from crates/scoped_stack/src/lib.rs
/// A stack whose only mutation is a scoped push: [`temp_push`](Stack::temp_push)
/// returns a guard that pops on drop, and [`with_pushed`](Stack::with_pushed) scopes
/// the push to a closure. Everything pushed is popped when its scope ends.
#[derive(Debug)]
pub struct Stack<T>(Vec<T>);

impl<T> Stack<T> {
    pub fn new() -> Self {
        Stack(Vec::new())
    }

    /// Every item on the stack, the callers' items included, innermost last.
    pub fn all(&self) -> &[T] {
        &self.0
    }

    /// The item stays until the returned guard drops. The guard borrows the stack, so
    /// it is the only usable handle while it lives, and it derefs to [`Stack`], so a
    /// callee takes `&mut Stack<T>` whether or not its caller holds a guard.
    pub fn temp_push(&mut self, item: T) -> Pushed<'_, T> {
        self.0.push(item);
        Pushed { stack: self }
    }

    /// The `with_` bracketing pattern from iso1's peekable lexer: the item is on the
    /// stack exactly for the duration of the closure.
    pub fn with_pushed<R>(&mut self, item: T, do_stuff: impl FnOnce(&mut Stack<T>) -> R) -> R {
        let mut pushed = self.temp_push(item);
        do_stuff(&mut pushed)
    }
}

impl<T> Default for Stack<T> {
    fn default() -> Self {
        Stack::new()
    }
}

/// The guard for one pushed item. Drop pops that item, however the scope exits, a
/// panic included.
#[derive(Debug)]
pub struct Pushed<'a, T> {
    stack: &'a mut Stack<T>,
}

impl<T> Drop for Pushed<'_, T> {
    fn drop(&mut self) {
        self.stack.0.pop();
    }
}

impl<T> std::ops::Deref for Pushed<'_, T> {
    type Target = Stack<T>;

    fn deref(&self) -> &Stack<T> {
        self.stack
    }
}

impl<T> std::ops::DerefMut for Pushed<'_, T> {
    fn deref_mut(&mut self) -> &mut Stack<T> {
        self.stack
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

The soundness argument is small. A guard pops exactly the item it pushed: while it lives it holds the one `&mut` to the stack, so nothing else can push or pop underneath it, and nested guards release in reverse order because each borrows the one before. `DerefMut` hands out `&mut Stack<T>`, but the field is private and `Stack`'s own surface is only `all`, `temp_push`, and `with_pushed`, so the deref grants nothing unscoped. The enforcement is `Drop`, so `mem::forget(pushed)` would leak the item past its scope; nothing calls `forget`, and doing so requires doing it deliberately.

## The use cases

The API is sized against every stack-shaped push/pop site in iso1 plus the i2 matcher.

- The i2 matcher (raw-items.md): `parse_bracketed` holds a `BracketKind` on the stack for the duration of the recursion into `parse_items`, and close-bracket classification reads the whole stack. `with_pushed` and `all().contains(..)` cover it.
- iso1 `reader_ast.rs` (`generate_reader_ast_with_path` and `refetched_paths_with_path`, four sites): each pushes a `NormalizationKey`, recurses, and pops, with whole-path reads via `path.clone()` inside the recursion. Each site is a `with_pushed`; the clone is `stack.all().to_vec()`.
- iso1 `create_merged_selection_set.rs` (`traversal_path`): the pushes live in callees — `merge_server_object_field` pushes the inline-fragment or linked-field key, `insert_client_object_selectable_into_refetch_paths` pushes the client-pointer key — and the one pop lives in the caller's object-selection arm. Nothing reads the stack between the callee's return and that pop, so the item only needs to live for the rest of the callee's own body, and `temp_push` expresses that: the callee computes the key partway through, pushes it, and keeps the guard for the remainder, where a closure would force everything after the push inward.

Two stack-adjacent sites in iso1 are not use cases, and the API deliberately does not stretch to them. pico's `DependencyStack` registers dependencies from arbitrary call depth through `&Database`, so it needs shared mutation via `RefCell` rather than a threaded `&mut`, and its `leave` returns the popped value; it keeps its own guard. The peekable lexer's `semantic_tokens.pop()` removes an entry another function pushed as a side effect — an accumulator with a corrective pop, not scope discipline.

## Tests

```rust
// from crates/scoped_stack/src/lib.rs
#[cfg(test)]
mod test {
    use crate::Stack;

    #[test]
    fn with_pushed_holds_the_item_for_the_closure() {
        let mut stack = Stack::new();

        stack.with_pushed(1, |stack| {
            assert_eq!(stack.all(), &[1]);
        });

        assert!(stack.all().is_empty());
    }

    #[test]
    fn nested_closures_see_outer_items() {
        let mut stack = Stack::new();

        stack.with_pushed(1, |stack| {
            stack.with_pushed(2, |stack| {
                assert_eq!(stack.all(), &[1, 2]);
            });
            assert_eq!(stack.all(), &[1]);
        });
    }

    #[test]
    fn temp_push_pops_when_the_guard_drops() {
        let mut stack = Stack::new();

        {
            let mut pushed = stack.temp_push(1);
            assert_eq!(pushed.all(), &[1]);

            let pushed_again = pushed.temp_push(2);
            assert_eq!(pushed_again.all(), &[1, 2]);
        }

        assert!(stack.all().is_empty());
    }

    #[test]
    fn a_callee_pushes_for_the_rest_of_its_body() {
        fn callee(stack: &mut Stack<i32>) {
            let mut stack = stack.temp_push(2);
            recurse(&mut stack);
            assert_eq!(stack.all(), &[1, 2]);
        }

        fn recurse(stack: &mut Stack<i32>) {
            assert_eq!(stack.all(), &[1, 2]);
        }

        let mut stack = Stack::new();
        stack.with_pushed(1, |stack| {
            callee(stack);
            assert_eq!(stack.all(), &[1]);
        });
    }

    #[test]
    fn with_pushed_returns_the_closure_result() {
        let mut stack = Stack::new();

        let result = stack.with_pushed(1, |stack| stack.all().len());

        assert_eq!(result, 1);
    }

    #[test]
    fn a_panic_still_pops() {
        let mut stack = Stack::new();

        let panicked = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            stack.with_pushed(1, |_stack| panic!("unwind out of the closure"));
        }));

        assert!(panicked.is_err());
        assert!(stack.all().is_empty());
    }
}
```

## Shipping order

One change: create the crate as printed, with its tests. The workspace member glob picks it up; no other file changes. raw-items.md consumes it when raw-items lands.

## Consequences

- raw-items.md's matcher threads `&mut Stack<BracketKind>` through the parse functions and holds each group's kind with `with_pushed`; that doc is updated alongside this one.
- iso1's `reader_ast.rs` and `create_merged_selection_set.rs` patterns have a home when their i2 equivalents get written; nothing rewrites iso1 itself.

## Landing checklist

- `cargo test -p scoped_stack` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
