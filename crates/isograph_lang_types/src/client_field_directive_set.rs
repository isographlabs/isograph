use serde::Deserialize;

use crate::EmptyDirectiveSet;

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ClientScalarSelectionDirectiveSet {
    Component(ComponentDirectiveSet),
    None(EmptyDirectiveSet),
}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentDirectiveSet {
    pub component: ComponentDirectiveParameters,
}
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ComponentDirectiveParameters {}

#[derive(Deserialize, Debug, Default, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EntityDirectiveSet {
    pub expose_as: Option<ExposeFieldDirective>,
    pub canonical_id: Option<CanonicalIdDirective>,
}

struct CanonicalIdDirective {
    field_name: 
}