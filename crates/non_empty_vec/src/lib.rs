use std::iter::once;
use std::ops::Index;

/// A vec with at least one element, by representation: the first element is its own
/// field, so no emptying operation can exist and `first`/`last` are total.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NonEmptyVec<T> {
    first: T,
    rest: Vec<T>,
}

impl<T> NonEmptyVec<T> {
    pub fn of(first: T) -> Self {
        NonEmptyVec {
            first,
            rest: Vec::new(),
        }
    }

    pub fn push(&mut self, item: T) {
        self.rest.push(item);
    }

    pub fn first(&self) -> &T {
        &self.first
    }

    pub fn last(&self) -> &T {
        self.rest.last().unwrap_or(&self.first)
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        1 + self.rest.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        once(&self.first).chain(self.rest.iter())
    }

    pub fn get(&self, index: usize) -> Option<&T> {
        match index {
            0 => Some(&self.first),
            index => self.rest.get(index - 1),
        }
    }
}

/// Indexes like a slice, panicking out of bounds like one; `get` is the checked form.
impl<T> Index<usize> for NonEmptyVec<T> {
    type Output = T;

    fn index(&self, index: usize) -> &T {
        match index {
            0 => &self.first,
            index => &self.rest[index - 1],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::NonEmptyVec;

    #[test]
    fn a_single_element_is_first_and_last() {
        let vec = NonEmptyVec::of(7);
        assert_eq!(*vec.first(), 7);
        assert_eq!(*vec.last(), 7);
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.get(0), Some(&7));
        assert_eq!(vec.get(1), None);
        assert_eq!(vec[0], 7);
        assert_eq!(vec.iter().copied().collect::<Vec<_>>(), vec![7]);
    }

    #[test]
    fn push_appends_and_last_moves() {
        let mut vec = NonEmptyVec::of(1);
        vec.push(2);
        vec.push(3);
        assert_eq!(*vec.first(), 1);
        assert_eq!(*vec.last(), 3);
        assert_eq!(vec.len(), 3);
        assert_eq!(vec.get(1), Some(&2));
        assert_eq!(vec.get(2), Some(&3));
        assert_eq!(vec.get(3), None);
        assert_eq!(vec[0], 1);
        assert_eq!(vec[1], 2);
        assert_eq!(vec[2], 3);
        assert_eq!(vec.iter().copied().collect::<Vec<_>>(), vec![1, 2, 3]);
    }
}
