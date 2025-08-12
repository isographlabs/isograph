use common_lang_types::DescriptionValue;

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
    };
}

define_wrapper!(Description, DescriptionValue);
