# Peekable with a guarded peek

A new crate, `peekable`, holds an iterator wrapper whose `peek` returns a guard. `view` lends the peeked item as `&I::Item`, `commit` consumes the item and returns it owned, and dropping the guard leaves the iterator exactly where `peek` found it — the same non-consuming peek semantics as `std::iter::Peekable`, so repeated peeks see the same item. The guard's lifetime is `peek`'s `&mut` borrow of the wrapper, so while a guard lives nothing can call `next` underneath it: "peek, then decide" is the only shape the API admits, and the peek-then-`next` pairs in the matcher become a single `commit` on the item that was actually viewed.

## The API

The whole crate:

```rust
// from crates/peekable/src/lib.rs
/// An iterator wrapper whose peek is scoped: [`peek`](Peekable::peek) returns a guard
/// holding the next item, [`view`](Peek::view) lends that item,
/// [`commit`](Peek::commit) consumes and returns it, and dropping the guard leaves the
/// iterator where `peek` found it. The guard's lifetime is `peek`'s borrow of the
/// wrapper, so it is the only handle that can advance the iterator while it lives.
pub struct Peekable<I: Iterator> {
    iter: I,
    /// The item `peek` pulled out of `iter` and no guard has committed: the next item,
    /// ahead of everything still in `iter`.
    peeked: Option<I::Item>,
}

impl<I: Iterator> Peekable<I> {
    pub fn new(iter: I) -> Self {
        Peekable { iter, peeked: None }
    }

    /// The next item, in a guard. Dropping the guard leaves the item as the next item;
    /// [`commit`](Peek::commit) consumes it.
    pub fn peek(&mut self) -> Option<Peek<'_, I::Item>> {
        if self.peeked.is_none() {
            self.peeked = self.iter.next();
        }
        Full::new(&mut self.peeked).map(Peek)
    }
}

impl<I: Iterator> Iterator for Peekable<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.peeked.take().or_else(|| self.iter.next())
    }
}

/// The guard for one peeked item. While it lives it holds `peek`'s `&mut` to the
/// wrapper, so the item [`view`](Peek::view) lends is the item
/// [`commit`](Peek::commit) returns.
pub struct Peek<'a, T>(Full<'a, T>);

impl<T> Peek<'_, T> {
    /// The item this guard peeked.
    ///
    /// The borrow is the guard's, not the wrapper's: a view dies with its `Peek`.
    ///
    /// ```compile_fail
    /// let mut iter = peekable::Peekable::new([1].into_iter());
    /// let peek = iter.peek().expect("one item remains");
    /// let item = peek.view();
    /// peek.commit();
    /// assert_eq!(item, &1);
    /// ```
    pub fn view(&self) -> &T {
        self.0.get()
    }

    /// Consume the item: the wrapper's `next`, owned.
    pub fn commit(self) -> T {
        self.0.take()
    }
}

/// A slot proven full. The constructor is the one place that checks, and a `Full`
/// holds the slot's only reference, so nothing can empty it while the `Full` lives.
struct Full<'a, T>(&'a mut Option<T>);

impl<'a, T> Full<'a, T> {
    fn new(slot: &'a mut Option<T>) -> Option<Full<'a, T>> {
        match slot {
            Some(_) => Some(Full(slot)),
            None => None,
        }
    }

    fn get(&self) -> &T {
        match &*self.0 {
            Some(item) => item,
            None => slot_emptied_under_full(),
        }
    }

    fn take(self) -> T {
        match self.0.take() {
            Some(item) => item,
            None => slot_emptied_under_full(),
        }
    }
}

/// The crate's one panic path, unreachable by construction: a [`Full`] is built only
/// over a `Some` and holds the slot's only reference for its whole life.
fn slot_emptied_under_full() -> ! {
    unreachable!("a Full exists only while its slot holds an item")
}
```

```toml
# from crates/peekable/Cargo.toml
[package]
name = "peekable"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]

[lints]
workspace = true
```

The soundness argument is small. The guard stores only the slot's `&mut`, but its lifetime is `peek`'s borrow of the whole wrapper, so while a `Peek` lives no `next`, no second `peek`, and no other guard can compile against the wrapper — the guard provably cannot touch `iter`, and nothing else can either. Fullness is proven once, in `Full::new`; `get` and `take` are the only readers, and they read through the slot's only reference, so nothing can empty it between the check and the read. `slot_emptied_under_full` is the crate's one panic path, no input reaches it — exhaustion is handled at `peek`, which returns `None` before a guard exists — and `Peek` itself has no panic path. `view` returns a borrow of the guard, not of the wrapper: the elided lifetime is `&self`'s, which is what forces every view to die before `commit` moves the guard, and the `compile_fail` doctest on `view` pins that signature against a widening to `'a`. Dropping the guard runs no code — there is no `Drop` impl — and the item stays in the slot as the next item, so restore-on-drop does not depend on a destructor running; even a `mem::forget` of the guard changes nothing.

## The consumer

`crates/isograph_parser/src/matched_brackets.rs` swaps `std::iter::Peekable` for this crate. The `TokenStream` alias keeps its exact text; only the import behind it changes, and every `tokens.peek()`/`tokens.next()` pair becomes a guard.

```toml
# from crates/isograph_parser/Cargo.toml
[dependencies]
logos = { workspace = true }
peekable = { path = "../peekable" }
resolve_position = { path = "../resolve_position" }
resolve_position_macros = { path = "../resolve_position_macros" }
span = { path = "../span" }
```

The import and construction:

```rust
// from crates/isograph_parser/src/matched_brackets.rs (before)
use std::fmt;
use std::iter::Peekable;

type TokenStream = Peekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = tokens.into_iter().peekable();
    let mut enclosing = Vec::new();
    let items = parse_items(&mut tokens, &mut enclosing);
    MatchedBrackets(items)
}
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs (after)
use std::fmt;

use peekable::Peekable;

type TokenStream = Peekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = Peekable::new(tokens.into_iter());
    let mut enclosing = Vec::new();
    let items = parse_items(&mut tokens, &mut enclosing);
    MatchedBrackets(items)
}
```

`parse_items` today peeks once and then calls `tokens.next()` in three of its four arms; the fourth arm's correctness rests on remembering not to call it. With the guard, each consuming arm commits the peek it viewed, and the leave-it arm just breaks. The token is `Copy`, so the arms copy it out of `view` up front and discard `commit`'s return:

```rust
// from crates/isograph_parser/src/matched_brackets.rs (before)
fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem<BracketsMatched>>> {
    let mut items = Vec::new();
    let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    while let Some(&token) = tokens.peek() {
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                tokens.next();
                run.push(WithSpan::new(kind, token.location));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(parse_bracketed(
                    tokens,
                    enclosing,
                    WithSpan::new(OpenBracket(kind), token.location),
                ));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Leaving it unconsumed is what
                    // synthetically closes every group between here and its owner.
                    break;
                }
                flush_run(&mut items, &mut run);
                tokens.next();
                items.push(WithSpan::new(
                    BracketItem::StrayClose(CloseBracket(kind)),
                    token.location,
                ));
            }
        }
    }
    flush_run(&mut items, &mut run);
    items
}
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs (after)
fn parse_items(
    tokens: &mut TokenStream,
    enclosing: &mut Vec<BracketKind>,
) -> Vec<WithSpan<BracketItem<BracketsMatched>>> {
    let mut items = Vec::new();
    let mut run: Vec<WithSpan<NonBracketTokenKind>> = Vec::new();
    while let Some(peek) = tokens.peek() {
        let token = *peek.view();
        match SplitToken::from(token.item) {
            SplitToken::NonBracket(kind) => {
                peek.commit();
                run.push(WithSpan::new(kind, token.location));
            }
            SplitToken::Bracket(BracketToken::Open(kind)) => {
                flush_run(&mut items, &mut run);
                peek.commit();
                items.push(parse_bracketed(
                    tokens,
                    enclosing,
                    WithSpan::new(OpenBracket(kind), token.location),
                ));
            }
            SplitToken::Bracket(BracketToken::Close(kind)) => {
                if enclosing.contains(&kind) {
                    // Some enclosing group owns this close. Dropping the peek leaves it
                    // unconsumed, which is what synthetically closes every group between
                    // here and its owner.
                    break;
                }
                flush_run(&mut items, &mut run);
                peek.commit();
                items.push(WithSpan::new(
                    BracketItem::StrayClose(CloseBracket(kind)),
                    token.location,
                ));
            }
        }
    }
    flush_run(&mut items, &mut run);
    items
}
```

The `Open` arm's `peek.commit()` consumes the guard, which ends its borrow of `tokens`, so the recursive `parse_bracketed(tokens, ..)` call borrows `tokens` fresh. In the `break` arm the guard is dropped un-committed, and the close stays as the next token for the enclosing level.

`parse_bracketed` today peeks at the token that stopped its children and consumes it only when it is the group's own close. With the guard, the consuming arm takes the token from `commit`'s return, and the fall-through arm drops the guard where the wildcard binds it:

```rust
// from crates/isograph_parser/src/matched_brackets.rs (before)
    let (closing, end) = match tokens.peek() {
        Some(&token)
            if SplitToken::from(token.item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            tokens.next();
            (
                Some(WithSpan::new(CloseBracket(opening.item.0), token.location)),
                token.location.end,
            )
        }
        // The group was forced to end: at a close an enclosing group owns, or at the end
        // of the tokens. It ends after its last child, or right after the opening when
        // there is none.
        _ => (
            None,
            children
                .last()
                .map_or(opening.location.end, |last| last.location.end),
        ),
    };
```

```rust
// from crates/isograph_parser/src/matched_brackets.rs (after)
    let (closing, end) = match tokens.peek() {
        Some(peek)
            if SplitToken::from(peek.view().item)
                == SplitToken::Bracket(BracketToken::Close(opening.item.0)) =>
        {
            let token = peek.commit();
            (
                Some(WithSpan::new(CloseBracket(opening.item.0), token.location)),
                token.location.end,
            )
        }
        // The group was forced to end: at a close an enclosing group owns, or at the end
        // of the tokens. It ends after its last child, or right after the opening when
        // there is none. Dropping the peek leaves the stopping close for its owner.
        _ => (
            None,
            children
                .last()
                .map_or(opening.location.end, |last| last.location.end),
        ),
    };
```

## Tests

The `compile_fail` doctest on `view` is part of the suite; `cargo test -p peekable` runs it with the unit tests below.

```rust
// from crates/peekable/src/lib.rs
#[cfg(test)]
mod test {
    use crate::Peekable;

    #[test]
    fn a_dropped_peek_leaves_the_item_as_the_next_item() {
        let mut iter = Peekable::new([1, 2].into_iter());

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), &1);
        drop(peek);

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), &1);
    }

    #[test]
    fn commit_returns_the_viewed_item_and_consumes_it() {
        let mut iter = Peekable::new([1, 2].into_iter());

        assert_eq!(iter.peek().expect("two items remain").commit(), 1);

        let peek = iter.peek().expect("one item remains");
        assert_eq!(peek.view(), &2);
    }

    #[test]
    fn a_non_copy_item_moves_out_through_commit() {
        let mut iter = Peekable::new(vec![String::from("a")].into_iter());

        let peek = iter.peek().expect("one item remains");
        assert_eq!(*peek.view(), "a");
        assert_eq!(peek.commit(), "a");
    }

    #[test]
    fn peek_on_an_exhausted_iterator_is_none() {
        let mut iter = Peekable::new(std::iter::empty::<i32>());

        assert!(iter.peek().is_none());
    }

    #[test]
    fn committing_the_last_item_exhausts_the_iterator() {
        let mut iter = Peekable::new([1].into_iter());

        assert_eq!(iter.peek().expect("one item remains").commit(), 1);

        assert!(iter.peek().is_none());
    }

    #[test]
    fn next_returns_the_item_a_dropped_peek_left() {
        let mut iter = Peekable::new([1, 2].into_iter());

        drop(iter.peek().expect("two items remain"));

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
    }
}
```

## Shipping order

1. Create `crates/peekable` as printed, with its tests. The workspace member glob picks it up; no other file changes.
2. Adopt it in `crates/isograph_parser/src/matched_brackets.rs`: the dependency line, the import, `match_brackets`, `parse_items`, and `parse_bracketed`, exactly as printed. The existing matcher tests cover the change; no test changes.

## Landing checklist

- `cargo test -p peekable` passes, the `compile_fail` doctest included.
- `cargo test -p isograph_parser` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
