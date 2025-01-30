use crate::{ArtifactFileName, ObjectTypeAndFieldName};

pub struct ArtifactPathAndContent {
    pub type_and_field: Option<ObjectTypeAndFieldName>,
    pub file_name: ArtifactFileName,
    pub file_content: String,
}
