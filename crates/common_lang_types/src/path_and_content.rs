use crate::{ArtifactFileName, ParentObjectEntityNameAndSelectableName};

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
    pub type_and_field: Option<ParentObjectEntityNameAndSelectableName>,
    pub file_name: ArtifactFileName,
    pub file_content: FileContent,
}
