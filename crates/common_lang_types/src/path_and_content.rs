use crate::{ArtifactFilePrefix, ObjectTypeAndFieldName};

pub struct ArtifactPathAndContent {
    pub type_and_field: Option<ObjectTypeAndFieldName>,
    pub file_name_prefix: ArtifactFilePrefix,
    pub file_content: String,
}
