use crate::{ArtifactFileType, ObjectTypeAndFieldName};

pub struct ArtifactPathAndContent {
    pub type_and_field: Option<ObjectTypeAndFieldName>,
    pub file_name_prefix: ArtifactFileType,
    pub file_content: String,
}
