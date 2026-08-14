use prelude::Postfix;

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
            Some(_) => Peek(&mut self.peeked).wrap_some(),
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

#[cfg(test)]
mod test {
    use prelude::Postfix;

    use crate::IntoSafePeekable;

    #[test]
    fn a_dropped_peek_leaves_the_item_as_the_next_item() {
        let mut iter = [1, 2].into_iter().safe_peekable();

        {
            let peek = iter.peek().expect("two items remain");
            assert_eq!(peek.view(), &1);
        }

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
        assert_eq!(iter.size_hint(), (2, 2usize.wrap_some()));

        {
            let _peek = iter.peek().expect("two items remain");
        }
        assert_eq!(iter.size_hint(), (2, 2usize.wrap_some()));

        iter.peek().expect("two items remain").commit();
        assert_eq!(iter.size_hint(), (1, 1usize.wrap_some()));
    }

    #[test]
    fn next_returns_the_item_a_dropped_peek_left() {
        let mut iter = [1, 2].into_iter().safe_peekable();

        {
            let _peek = iter.peek().expect("two items remain");
        }

        assert_eq!(iter.next(), 1i32.wrap_some());
        assert_eq!(iter.next(), 2i32.wrap_some());
        assert_eq!(iter.next(), None);
    }
}
