use std::path::PathBuf;

use crate::RelativePathToSourceFile;

#[derive(Debug, Clone)]
pub struct AbsolutePathAndRelativePath {
    pub absolute_path: PathBuf,
    pub relative_path: RelativePathToSourceFile,
}
