use std::path::PathBuf;

use crate::FileContent;

#[derive(Debug, Clone)]
pub enum FileSystemOperation {
    DeleteDirectory(PathBuf),
    CreateDirectory(PathBuf),
    WriteFile(PathBuf, FileContent),
    DeleteFile(PathBuf),
}
