use isograph_config::IsographProjectConfig;
use schemars::schema_for;

fn main() {
    let schema = schema_for!(IsographProjectConfig);
    println!("{}", serde_json::to_string_pretty(&schema).unwrap());
}
