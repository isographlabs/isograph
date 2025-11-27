use serde::Deserialize;
use crate::create_additional_fields::expose_field_directive::ExposeFieldDirective;

#[derive(Deserialize, Debug, Clone, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServerObjectEntityDirectives {
    #[serde(default)]
    pub expose_field: Vec<ExposeFieldDirective>,
}