use std::ops::Deref;

use common_lang_types::DescriptionValue;

#[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, PartialOrd, Ord)]
pub struct Description(pub DescriptionValue);

impl From<DescriptionValue> for Description {
    fn from(value: DescriptionValue) -> Self {
        Self(value)
    }
}

impl Deref for Description {
    type Target = DescriptionValue;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
