use std::{
    fs::{self, File},
    io::{self, Write},
    path::PathBuf,
};

use graphql_artifact_generation::PathAndContent;
use thiserror::Error;

pub(crate) fn write_to_disk<'schema>(
    paths_and_contents: impl Iterator<Item = PathAndContent>,
    artifact_directory: &PathBuf,
) -> Result<usize, GenerateArtifactsError> {
    if artifact_directory.exists() {
        fs::remove_dir_all(&artifact_directory).map_err(|e| {
            GenerateArtifactsError::UnableToDeleteDirectory {
                path: artifact_directory.clone(),
                message: e,
            }
        })?;
    }
    fs::create_dir_all(&artifact_directory).map_err(|e| {
        GenerateArtifactsError::UnableToCreateDirectory {
            path: artifact_directory.clone(),
            message: e,
        }
    })?;

    let mut count = 0;
    for path_and_content in paths_and_contents {
        // Is this better than materializing paths_and_contents sooner?
        count += 1;

        let absolute_directory = artifact_directory.join(path_and_content.relative_directory);
        fs::create_dir_all(&absolute_directory).map_err(|e| {
            GenerateArtifactsError::UnableToCreateDirectory {
                path: absolute_directory.clone(),
                message: e,
            }
        })?;

        let absolute_file_path =
            absolute_directory.join(&format!("{}.ts", path_and_content.file_name_prefix));
        let mut file = File::create(&absolute_file_path).map_err(|e| {
            GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: absolute_file_path.clone(),
                message: e,
            }
        })?;

        file.write(path_and_content.file_content.as_bytes())
            .map_err(|e| GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: absolute_file_path.clone(),
                message: e,
            })?;
    }
    Ok(count)
}

#[derive(Debug, Error)]
pub enum GenerateArtifactsError {
    #[error("Unable to write to artifact file at path {path:?}.\nReason: {message:?}")]
    UnableToWriteToArtifactFile { path: PathBuf, message: io::Error },

    #[error("Unable to create directory at path {path:?}.\nReason: {message:?}")]
    UnableToCreateDirectory { path: PathBuf, message: io::Error },

    #[error("Unable to delete directory at path {path:?}.\nReason: {message:?}")]
    UnableToDeleteDirectory { path: PathBuf, message: io::Error },
}
