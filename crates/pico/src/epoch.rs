use std::num::NonZeroUsize;

const INIT: usize = 1;

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Epoch(NonZeroUsize);

impl Epoch {
    pub fn new() -> Self {
        Self::from(INIT)
    }

    pub fn from(value: usize) -> Self {
        Self(NonZeroUsize::new(value).unwrap())
    }

    pub fn increment(&mut self) -> Self {
        *self = Self::from(self.0.get() + 1);
        *self
    }

    /// An iterator from `self` to `to`, inclusive on the self and exclusive
    /// on the right.
    pub fn to(&self, to: Epoch) -> impl Iterator<Item = Epoch> {
        let left = self.0.get();
        let right = to.0.get();
        (left..right).map(Epoch::from)
    }
}

impl From<usize> for Epoch {
    fn from(value: usize) -> Self {
        Self::from(value)
    }
}

impl From<Epoch> for usize {
    fn from(epoch: Epoch) -> Self {
        epoch.0.into()
    }
}

impl std::fmt::Debug for Epoch {
    fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(fmt, "{}", self.0)
    }
}

impl Default for Epoch {
    fn default() -> Self {
        Self::from(INIT)
    }
}
