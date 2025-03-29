#[macro_export]
macro_rules! string_key_newtype {
    ($named:ident) => {
        // TODO serialize, deserialize

        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $named(pub(crate) intern::string_key::StringKey);

        impl intern::Lookup for $named {
            fn lookup(self) -> &'static str {
                self.0.lookup()
            }
        }

        impl std::fmt::Display for $named {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                use intern::Lookup;
                f.write_fmt(format_args!("{}", self.0.lookup()))
            }
        }

        impl From<intern::string_key::StringKey> for $named {
            fn from(other: intern::string_key::StringKey) -> Self {
                Self(other)
            }
        }

        impl<'de> serde::Deserialize<'de> for $named {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                let interned = intern::string_key::Intern::intern(s);
                Ok($named::from(interned))
            }
        }

        impl $named {
            pub fn unchecked_conversion<T: From<intern::string_key::StringKey>>(self) -> T {
                self.0.into()
            }
        }

        impl std::cmp::PartialEq<&'static str> for $named {
            fn eq(&self, other: &&'static str) -> bool {
                use intern::Lookup;
                self.lookup() == *other
            }
        }

        impl std::cmp::PartialEq<$named> for &'static str {
            fn eq(&self, other: &$named) -> bool {
                use intern::Lookup;
                *self == other.lookup()
            }
        }
    };
}

#[macro_export]
macro_rules! string_key_newtype_no_display {
    ($named:ident) => {
        #[derive(Clone, Copy, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $named(intern::string_key::StringKey);

        impl intern::Lookup for $named {
            fn lookup(self) -> &'static str {
                self.0.lookup()
            }
        }

        impl From<intern::string_key::StringKey> for $named {
            fn from(other: intern::string_key::StringKey) -> Self {
                Self(other)
            }
        }

        impl<'de> serde::Deserialize<'de> for $named {
            fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                let s: String = serde::Deserialize::deserialize(deserializer)?;
                let interned = intern::string_key::Intern::intern(s);
                Ok($named::from(interned))
            }
        }

        impl $named {
            pub fn unchecked_conversion<T: From<intern::string_key::StringKey>>(self) -> T {
                self.0.into()
            }
        }
    };
}

#[macro_export]
macro_rules! string_key_one_way_conversion {
    (from: $from:ident, to: $to:ident) => {
        impl From<$from> for $to {
            fn from(other: $from) -> Self {
                Self(other.0)
            }
        }

        impl std::cmp::PartialEq<$from> for $to {
            fn eq(&self, other: &$from) -> bool {
                self.0 == other.0
            }
        }

        impl std::cmp::PartialEq<$to> for $from {
            fn eq(&self, other: &$to) -> bool {
                self.0 == other.0
            }
        }
    };
}
