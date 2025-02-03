#[macro_export]
macro_rules! u64_newtype {
    ($named:ident) => {
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $named(u64);

        impl std::fmt::Display for $named {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_fmt(format_args!("$named({})", self.0))
            }
        }

        impl From<u64> for $named {
            fn from(other: u64) -> Self {
                Self(other)
            }
        }

        impl From<usize> for $named {
            fn from(other: usize) -> Self {
                Self(other as u64)
            }
        }

        impl std::ops::Deref for $named {
            type Target = u64;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl $named {
            pub fn as_usize(&self) -> usize {
                self.0 as usize
            }
        }
    };
}

#[macro_export]
macro_rules! u64_conversion {
    (from: $from:ident, to: $to:ident) => {
        impl From<$from> for $to {
            fn from(other: $from) -> Self {
                Self(other.0)
            }
        }
    };
}
