use std::num::NonZeroUsize;

const INIT: usize = 1;

macro_rules! define_epoch_type {
    ($name:ident) => {
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(NonZeroUsize);

        impl $name {
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
            pub fn to(&self, to: $name) -> impl Iterator<Item = $name> {
                let left = self.0.get();
                let right = to.0.get();
                (left..right).map($name::from)
            }
        }

        impl From<usize> for $name {
            fn from(value: usize) -> Self {
                Self::from(value)
            }
        }

        impl From<$name> for usize {
            fn from(epoch: $name) -> Self {
                epoch.0.into()
            }
        }

        impl std::fmt::Debug for $name {
            fn fmt(&self, fmt: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(fmt, "{}", self.0)
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::from(INIT)
            }
        }
    };
}

define_epoch_type!(Epoch);
define_epoch_type!(GcEpoch);
