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

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(from = "LazyLoadDirectiveParametersInput")]
pub struct LazyLoadDirectiveParameters {
    #[serde(default)]
    pub reader: bool,
    #[serde(default)]
    pub normalization: bool,
}

#[derive(Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
struct LazyLoadDirectiveParametersInput {
    #[serde(default)]
    reader: Option<bool>,
    #[serde(default)]
    normalization: Option<bool>,
}

impl From<LazyLoadDirectiveParametersInput> for LazyLoadDirectiveParameters {
    fn from(value: LazyLoadDirectiveParametersInput) -> Self {
        // @lazyLoad, @lazyLoad() => reader: true, normalization: true
        // @lazyLoad(reader: true) => reader: true, normalization: false
        // @lazyLoad(normalization: true) => reader: true, normalization: true
        // otherwise user-written values used
        match value {
            LazyLoadDirectiveParametersInput {
                reader: None,
                normalization: None,
            } => Self::default(),
            _ => Self {
                reader: value.reader.unwrap_or(true),
                normalization: value.normalization.unwrap_or(false),
            },
        }
    }
}

impl Default for LazyLoadDirectiveParameters {
    fn default() -> Self {
        Self {
            reader: true,
            normalization: true,
        }
    }
}
