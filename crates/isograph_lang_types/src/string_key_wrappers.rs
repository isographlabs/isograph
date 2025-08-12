use common_lang_types::{DescriptionValue, UnvalidatedTypeName};

macro_rules! define_wrapper {
    ($struct_name:ident, $inner:ident) => {
        #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
        pub struct $struct_name(pub common_lang_types::$inner);

        impl From<common_lang_types::$inner> for $struct_name {
            fn from(value: $inner) -> Self {
                Self(value)
            }
        }

        impl std::ops::Deref for $struct_name {
            type Target = common_lang_types::$inner;

            fn deref(&self) -> &Self::Target {
                &self.0
            }
        }

        impl std::fmt::Display for $struct_name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                self.0.fmt(f)
            }
        }
    };
}

define_wrapper!(Description, DescriptionValue);
define_wrapper!(ParentType, UnvalidatedTypeName);
