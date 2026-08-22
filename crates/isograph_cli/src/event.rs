use std::path::PathBuf;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
#[serde(tag = "kind", content = "value")]
pub enum IsographEvent {
    HelloWorld,
    Quit,
    DiskChanged(DiskChanged),
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub struct DiskChanged {
    pub path: PathBuf,
    pub presence: Presence,
}

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
pub enum Presence {
    Present(String),
    Absent,
}
