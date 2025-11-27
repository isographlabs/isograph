use std::path::PathBuf;

use crate::FileContent;

use pico::Index;

#[derive(Debug, Clone)]
pub enum FileSystemOperation {
    DeleteDirectory(PathBuf),
    CreateDirectory(PathBuf),
    WriteFile(PathBuf, Index<FileContent>),
    DeleteFile(PathBuf),
}
