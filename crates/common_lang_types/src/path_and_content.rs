use crate::{ArtifactFileName, ParentObjectEntityNameAndSelectableName};

pub struct ArtifactPathAndContent {
    pub type_and_field: Option<ParentObjectEntityNameAndSelectableName>,
    pub file_name: ArtifactFileName,
    pub file_content: String,
}
