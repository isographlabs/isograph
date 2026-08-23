use crate::{ArtifactFileName, EntityNameAndSelectableName};

#[derive(Debug, Clone, derive_more::From, derive_more::Display, derive_more::Deref)]
pub struct FileContent(pub String);

pub struct ArtifactPathAndContent {
    pub file_content: FileContent,
    pub artifact_path: ArtifactPath,
}

pub struct ArtifactPath {
    pub type_and_field: Option<EntityNameAndSelectableName>,
    pub file_name: ArtifactFileName,
}

#[derive(
    Debug, Clone, PartialEq, Eq, derive_more::From, derive_more::Display, derive_more::Deref,
)]
pub struct ArtifactHash(String);
