pub trait StringKeyNewtype:
    From<intern::string_key::StringKey> + std::fmt::Display + Clone
{
}

#[macro_export]
macro_rules! string_key_newtype {
    ($named:ident) => {
        // TODO serialize, deserialize

        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $named(intern::string_key::StringKey);

        impl string_key_newtype::StringKeyNewtype for $named {}

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
    };
}

#[macro_export]
macro_rules! string_key_conversion {
    (from: $from:ident, to: $to:ident) => {
        impl From<$from> for $to {
            fn from(other: $from) -> Self {
                Self(other.0)
            }
        }
    };
}
