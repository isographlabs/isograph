#[derive(Debug, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
}
