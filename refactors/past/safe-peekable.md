# Safe peekable

A new crate, `safe_peekable`, holds an iterator wrapper whose `peek` returns a guard; `.safe_peekable()` is the adapter that builds it, in `std`'s `.peekable()` spelling. `view` lends the peeked item as `&I::Item`, `commit` consumes the item and returns it owned, and dropping the guard leaves the iterator exactly where `peek` found it — the same non-consuming peek semantics as `std::iter::Peekable`, so repeated peeks see the same item. The guard's lifetime is `peek`'s `&mut` borrow of the wrapper, so while a guard lives nothing can call `next` underneath it: "peek, then decide" is the only shape the API admits, and the peek-then-`next` pairs in the matcher become a single `commit` on the item that was actually viewed.

## The API

The whole crate:

```rust
// from crates/safe_peekable/src/lib.rs
/// An iterator wrapper whose peek is scoped: [`peek`](SafePeekable::peek) returns a
/// guard holding the next item, [`view`](Peek::view) lends that item,
/// [`commit`](Peek::commit) consumes and returns it, and dropping the guard leaves the
/// iterator where `peek` found it. The guard's lifetime is `peek`'s borrow of the
/// wrapper, so it is the only handle that can advance the iterator while it lives.
pub struct SafePeekable<I: Iterator> {
    iter: I,
    /// The item `peek` pulled out of `iter` and no guard has committed: the next item,
    /// ahead of everything still in `iter`.
    peeked: Option<I::Item>,
}

/// The adapter that builds a [`SafePeekable`], in `std`'s `.peekable()` spelling.
pub trait IntoSafePeekable: Iterator + Sized {
    fn safe_peekable(self) -> SafePeekable<Self>;
}

impl<I: Iterator> IntoSafePeekable for I {
    fn safe_peekable(self) -> SafePeekable<I> {
        SafePeekable {
            iter: self,
            peeked: None,
        }
    }
}

impl<I: Iterator> SafePeekable<I> {
    /// The next item, in a guard. Dropping the guard leaves the item as the next item;
    /// [`commit`](Peek::commit) consumes it.
    pub fn peek(&mut self) -> Option<Peek<'_, I::Item>> {
        if self.peeked.is_none() {
            self.peeked = self.iter.next();
        }
        match self.peeked {
            Some(_) => Some(Peek(&mut self.peeked)),
            None => None,
        }
    }
}

impl<I: Iterator> Iterator for SafePeekable<I> {
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.peeked.take().or_else(|| self.iter.next())
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let buffered = usize::from(self.peeked.is_some());
        let (lower, upper) = self.iter.size_hint();
        (
            lower.saturating_add(buffered),
            upper.and_then(|upper| upper.checked_add(buffered)),
        )
    }
}

/// The guard for one peeked item.
pub struct Peek<'a, T>(&'a mut Option<T>);

impl<T> Peek<'_, T> {
    pub fn view(&self) -> &T {
        self.0
            .as_ref()
            .expect("a Peek exists only while the slot holds an item")
    }

    pub fn commit(self) -> T {
        self.0
            .take()
            .expect("a Peek exists only while the slot holds an item")
    }
}
```

```toml
# from crates/safe_peekable/Cargo.toml
[package]
name = "safe_peekable"
version = { workspace = true }
edition = { workspace = true }
license = { workspace = true }

[dependencies]

[lints]
workspace = true
```

The soundness argument is small. The guard stores only the slot's `&mut`, but its lifetime is `peek`'s borrow of the whole wrapper, so while a `Peek` lives no `next`, no second `peek`, and no other guard can compile against the wrapper — the guard provably cannot touch `iter`, and nothing else can either. `peek` constructs a guard only over a full slot, and `view` and `commit` read through the slot's only reference, so nothing can empty it between the check and the read; their two `expect`s are the crate's only panic paths, and no input reaches them — exhaustion is handled at `peek`, which returns `None` before a guard exists. `view` returns a borrow of the guard, not of the wrapper: the elided lifetime is `&self`'s, which is what forces every view to die before `commit` moves the guard. Dropping the guard runs no code — there is no `Drop` impl — and the item stays in the slot as the next item, so restore-on-drop does not depend on a destructor running; even a `mem::forget` of the guard changes nothing.

## The consumer

`crates/isograph_parser/src/matched_brackets.rs` swaps `std::iter::Peekable` for this crate. The `TokenStream` alias changes type name, and every `tokens.peek()`/`tokens.next()` pair becomes a guard.

```toml
# from crates/isograph_parser/Cargo.toml
[dependencies]
logos = { workspace = true }
resolve_position = { path = "../resolve_position" }
resolve_position_macros = { path = "../resolve_position_macros" }
safe_peekable = { path = "../safe_peekable" }
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

use safe_peekable::{IntoSafePeekable, SafePeekable};

type TokenStream = SafePeekable<std::vec::IntoIter<WithSpan<IsographLangTokenKind>>>;

pub fn match_brackets(
    tokens: Vec<WithSpan<IsographLangTokenKind>>,
) -> MatchedBrackets<BracketsMatched> {
    let mut tokens = tokens.into_iter().safe_peekable();
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

```rust
// from crates/safe_peekable/src/lib.rs
#[cfg(test)]
mod test {
    use crate::IntoSafePeekable;

    #[test]
    fn a_dropped_peek_leaves_the_item_as_the_next_item() {
        let mut iter = [1, 2].into_iter().safe_peekable();

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), &1);
        drop(peek);

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), &1);
    }

    #[test]
    fn commit_returns_the_viewed_item_and_consumes_it() {
        let mut iter = [1, 2].into_iter().safe_peekable();

        assert_eq!(iter.peek().expect("two items remain").commit(), 1);

        let peek = iter.peek().expect("one item remains");
        assert_eq!(peek.view(), &2);
    }

    #[test]
    fn a_non_copy_item_moves_out_through_commit() {
        let mut iter = vec![String::from("a")].into_iter().safe_peekable();

        let peek = iter.peek().expect("one item remains");
        assert_eq!(*peek.view(), "a");
        assert_eq!(peek.commit(), "a");
    }

    #[test]
    fn peek_on_an_exhausted_iterator_is_none() {
        let mut iter = std::iter::empty::<i32>().safe_peekable();

        assert!(iter.peek().is_none());
    }

    #[test]
    fn committing_the_last_item_exhausts_the_iterator() {
        let mut iter = [1].into_iter().safe_peekable();

        assert_eq!(iter.peek().expect("one item remains").commit(), 1);

        assert!(iter.peek().is_none());
    }

    #[test]
    fn size_hint_counts_the_buffered_item() {
        let mut iter = [1, 2].into_iter().safe_peekable();
        assert_eq!(iter.size_hint(), (2, Some(2)));

        drop(iter.peek().expect("two items remain"));
        assert_eq!(iter.size_hint(), (2, Some(2)));

        iter.peek().expect("two items remain").commit();
        assert_eq!(iter.size_hint(), (1, Some(1)));
    }

    #[test]
    fn next_returns_the_item_a_dropped_peek_left() {
        let mut iter = [1, 2].into_iter().safe_peekable();

        drop(iter.peek().expect("two items remain"));

        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
    }
}
```

## Shipping order

1. Create `crates/safe_peekable` as printed, with its tests. The workspace member glob picks it up; no other file changes.
2. Adopt it in `crates/isograph_parser/src/matched_brackets.rs`: the dependency line, the import, the `TokenStream` alias, `match_brackets`, `parse_items`, and `parse_bracketed`, exactly as printed. The existing matcher tests cover the change; no test changes.

## Landing checklist

- `cargo test -p safe_peekable` passes.
- `cargo test -p isograph_parser` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
