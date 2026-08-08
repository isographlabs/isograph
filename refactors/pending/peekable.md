# Peekable with a guarded peek

A new crate, `peekable`, holds an iterator wrapper whose `peek` returns a guard. `view` hands out the peeked item, `commit` consumes it, and dropping the guard leaves the iterator exactly where `peek` found it — the same non-consuming peek semantics as `std::iter::Peekable`, so repeated peeks see the same item. The guard holds the one `&mut` to the wrapper, so while it lives nothing can call `next` underneath it: "peek, then decide" is the only shape the API admits, and the peek-then-`next` pairs in the matcher become a single `commit` on the item that was actually viewed.

The wrapper requires `I::Item: Copy`. That bound is what makes every method total: the guard hands the item out by value, so nothing has to be put back on drop, there is no vacant state, and no method has a panic arm.

## The API

The whole crate:

```rust
// from crates/peekable/src/lib.rs
/// An iterator wrapper whose peek is scoped: [`peek`](Peekable::peek) returns a guard
/// holding the next item, [`commit`](Peeked::commit) consumes that item, and dropping
/// the guard leaves the iterator where `peek` found it. The guard borrows the wrapper,
/// so it is the only handle that can advance the iterator while it lives.
pub struct Peekable<I: Iterator>
where
    I::Item: Copy,
{
    iter: I,
    /// The item `peek` pulled out of `iter` and no guard has committed: the next item,
    /// ahead of everything still in `iter`.
    peeked: Option<I::Item>,
}

impl<I: Iterator> Peekable<I>
where
    I::Item: Copy,
{
    pub fn new(iter: I) -> Self {
        Peekable { iter, peeked: None }
    }

    /// The next item, in a guard. Dropping the guard leaves the item as the next item;
    /// [`commit`](Peeked::commit) consumes it.
    pub fn peek(&mut self) -> Option<Peeked<'_, I>> {
        let item = match self.peeked {
            Some(item) => item,
            None => {
                let item = self.iter.next()?;
                self.peeked = Some(item);
                item
            }
        };
        Some(Peeked { owner: self, item })
    }
}

impl<I: Iterator> Iterator for Peekable<I>
where
    I::Item: Copy,
{
    type Item = I::Item;

    fn next(&mut self) -> Option<I::Item> {
        self.peeked.take().or_else(|| self.iter.next())
    }
}

/// The guard for one peeked item. While it lives it holds the one `&mut` to the
/// wrapper, so the item [`view`](Peeked::view) returns is the item
/// [`commit`](Peeked::commit) consumes.
pub struct Peeked<'a, I: Iterator>
where
    I::Item: Copy,
{
    owner: &'a mut Peekable<I>,
    item: I::Item,
}

impl<I: Iterator> Peeked<'_, I>
where
    I::Item: Copy,
{
    /// The item this guard peeked.
    pub fn view(&self) -> I::Item {
        self.item
    }

    /// Consume the item: the wrapper's `next`, whose result `view` already handed out.
    pub fn commit(self) {
        self.owner.next();
    }
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

The soundness argument is small. The guard's copy of the item and the wrapper's `peeked` slot cannot diverge, because the guard holds the one `&mut` to the wrapper for its whole life: no `next`, no second `peek`, and no other guard can run while it lives. `commit` consumes the guard and drains the slot through `next`, so the item `view` handed out is the item that comes off the stream. Dropping the guard runs no code — there is no `Drop` impl — and the item simply stays in the slot as the next item, so the restore-on-drop property does not depend on a destructor running; even a `mem::forget` of the guard changes nothing. `peek` has one exit without a guard, the `?` on an exhausted underlying iterator, which leaves the slot `None` and returns `None`.

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

`parse_items` today peeks once and then calls `tokens.next()` in three of its four arms; the fourth arm's correctness rests on remembering not to call it. With the guard, each consuming arm commits the peek it viewed, and the leave-it arm just breaks:

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
        let token = peek.view();
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

`parse_bracketed` today peeks at the token that stopped its children and consumes it only when it is the group's own close. With the guard, the consuming arm commits and the fall-through arm drops the guard where the wildcard binds it:

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
            let token = peek.view();
            peek.commit();
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
// from crates/peekable/src/lib.rs
#[cfg(test)]
mod test {
    use crate::Peekable;

    #[test]
    fn a_dropped_peek_leaves_the_item_as_the_next_item() {
        let mut iter = Peekable::new([1, 2].into_iter());

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), 1);
        drop(peek);

        let peek = iter.peek().expect("two items remain");
        assert_eq!(peek.view(), 1);
    }

    #[test]
    fn commit_consumes_the_viewed_item() {
        let mut iter = Peekable::new([1, 2].into_iter());

        iter.peek().expect("two items remain").commit();

        let peek = iter.peek().expect("one item remains");
        assert_eq!(peek.view(), 2);
    }

    #[test]
    fn peek_on_an_exhausted_iterator_is_none() {
        let mut iter = Peekable::new(std::iter::empty::<i32>());

        assert!(iter.peek().is_none());
    }

    #[test]
    fn committing_the_last_item_exhausts_the_iterator() {
        let mut iter = Peekable::new([1].into_iter());

        iter.peek().expect("one item remains").commit();

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

- `cargo test -p peekable` passes.
- `cargo test -p isograph_parser` passes.
- `cargo clippy --workspace --exclude pico --all-targets -- -D warnings` passes.
- The doc moves to `refactors/past/`.
