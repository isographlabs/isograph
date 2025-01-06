use std::fs;

use isograph_config::IsographProjectConfig;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(IsographProjectConfig);

    fs::write(
        "./libs/isograph-compiler/isograph-config-schema.json",
        serde_json::to_string_pretty(&schema).unwrap(),
    )
    .unwrap();
}
