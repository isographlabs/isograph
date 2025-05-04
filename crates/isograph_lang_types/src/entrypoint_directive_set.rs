use serde::Deserialize;

use crate::EmptyDirectiveSet;

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(rename_all = "camelCase", untagged)]
pub enum EntrypointDirectiveSet {
    LazyLoad(LazyLoadDirectiveSet),
    None(EmptyDirectiveSet),
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LazyLoadDirectiveSet {
    pub lazy_load: LazyLoadDirectiveParameters,
}

#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct LazyLoadDirectiveParameters {}
