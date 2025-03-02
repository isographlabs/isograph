use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, PartialEq, PartialOrd, Ord, Eq, Copy, Hash)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ClientFieldDirectiveSet {
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

// No directives -> an EmptyStruct is parsed!
#[derive(Deserialize, Debug, Default, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct EmptyDirectiveSet {}
