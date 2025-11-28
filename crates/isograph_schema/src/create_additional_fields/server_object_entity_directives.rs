use crate::create_additional_fields::canonical_id_directive::CanonicalIdDirective;
use crate::create_additional_fields::expose_field_directive::ExposeFieldDirective;
use serde::Deserialize;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerObjectEntityDirectives {
    #[serde(default)]
    pub expose_field: Vec<ExposeFieldDirective>,
    #[serde(default)]
    pub canonical_id: Option<CanonicalIdDirective>,
}
