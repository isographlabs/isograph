use std::fs;

use isograph_config::IsographProjectConfig;
use prelude::Postfix;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(IsographProjectConfig);

    fs::write(
        "./libs/isograph-compiler/isograph-config-schema.json",
        format!(
            "{}\n",
            serde_json::to_string_pretty(schema.reference()).unwrap()
        ),
    )
    .unwrap();
}
