use intern::{string_key::StringKey, Lookup};
use std::fmt;

use super::TypeTrait;

macro_rules! string_key_newtype {
    ($named:ident) => {
        // TODO serialize, deserialize
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $named(StringKey);

        impl Lookup for $named {
            fn lookup(self) -> &'static str {
                self.0.lookup()
            }
        }

        impl fmt::Display for $named {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_fmt(format_args!("{}", self.0.lookup()))
            }
        }

        impl From<StringKey> for $named {
            fn from(other: StringKey) -> Self {
                Self(other)
            }
        }
    };
}

string_key_newtype!(DirectiveName);
string_key_newtype!(DirectiveArgumentName);
string_key_newtype!(FieldDefinitionName);
string_key_newtype!(InputValueName);
string_key_newtype!(EnumLiteralValue);
string_key_newtype!(StringLiteralValue);
string_key_newtype!(DescriptionValue);
string_key_newtype!(VariableName);
string_key_newtype!(ValueKeyName);

// OutputTypeName and InputTypeName should **only** exist on the schema parsing
// side! Later, they should be converted to some sort of enums. These represent
// unvalidated strings.
string_key_newtype!(OutputTypeName);
impl TypeTrait for OutputTypeName {}
string_key_newtype!(InputTypeName);
impl TypeTrait for InputTypeName {}

string_key_newtype!(ObjectTypeName);
string_key_newtype!(InterfaceTypeName);
