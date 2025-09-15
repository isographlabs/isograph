use crate::{ArtifactFileName, SelectableName, ServerObjectEntityName};

pub struct ArtifactPathAndContent {
    pub type_and_field: Option<ArtifactPathAndContentTypeAndField>,
    pub file_name: ArtifactFileName,
    pub file_content: String,
}

#[derive(Debug, PartialEq, Eq, Hash, Ord, PartialOrd, Clone, Copy)]
pub struct ArtifactPathAndContentTypeAndField {
    pub type_name: ServerObjectEntityName,
    pub field_name: Option<SelectableName>,
}
