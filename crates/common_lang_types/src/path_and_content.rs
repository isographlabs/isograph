use crate::{ArtifactFileName, EntityNameAndSelectableName};

#[derive(Debug, Clone)]
pub struct FileContent(pub String);

impl From<String> for FileContent {
    fn from(value: String) -> Self {
        FileContent(value)
    }
}

impl std::fmt::Display for FileContent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for FileContent {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct ArtifactPathAndContent {
    pub file_content: FileContent,
    pub artifact_path: ArtifactPath,
}

pub struct ArtifactPath {
    pub type_and_field: Option<EntityNameAndSelectableName>,
    pub file_name: ArtifactFileName,
}

#[derive(Debug, Clone)]
pub struct ArtifactHash(String);

impl From<String> for ArtifactHash {
    fn from(value: String) -> Self {
        ArtifactHash(value)
    }
}

impl std::fmt::Display for ArtifactHash {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::ops::Deref for ArtifactHash {
    type Target = String;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl PartialEq for ArtifactHash {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
