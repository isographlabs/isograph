use std::path::PathBuf;

use crate::ArtifactFileType;

pub struct PathAndContent {
    pub relative_directory: PathBuf,
    pub file_name_prefix: ArtifactFileType,
    pub file_content: String,
}
