use std::{
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use common_lang_types::ArtifactPathAndContent;
use intern::string_key::Lookup;
use thiserror::Error;

pub(crate) fn write_artifacts_to_disk(
    paths_and_contents: impl IntoIterator<Item = ArtifactPathAndContent>,
    artifact_directory: &PathBuf,
) -> Result<usize, GenerateArtifactsError> {
    if artifact_directory.exists() {
        fs::remove_dir_all(artifact_directory).map_err(|e| {
            GenerateArtifactsError::UnableToDeleteDirectory {
                path: artifact_directory.clone(),
                message: e.to_string(),
            }
        })?;
    }
    fs::create_dir_all(artifact_directory).map_err(|e| {
        GenerateArtifactsError::UnableToCreateDirectory {
            path: artifact_directory.clone(),
            message: e.to_string(),
        }
    })?;

    let mut count = 0;
    for path_and_content in paths_and_contents {
        // Is this better than materializing paths_and_contents sooner?
        count += 1;

        let absolute_directory = match path_and_content.type_and_field {
            Some(type_and_field) => artifact_directory
                .join(type_and_field.type_name.lookup())
                .join(type_and_field.field_name.lookup()),
            None => artifact_directory.clone(),
        };
        fs::create_dir_all(&absolute_directory).map_err(|e| {
            GenerateArtifactsError::UnableToCreateDirectory {
                path: absolute_directory.clone(),
                message: e.to_string(),
            }
        })?;

        let absolute_file_path =
            absolute_directory.join(format!("{}.ts", path_and_content.file_name_prefix));
        let mut file = File::create(&absolute_file_path).map_err(|e| {
            GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: absolute_file_path.clone(),
                message: e.to_string(),
            }
        })?;

        file.write(path_and_content.file_content.as_bytes())
            .map_err(|e| GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: absolute_file_path.clone(),
                message: e.to_string(),
            })?;
    }
    Ok(count)
}

#[allow(clippy::enum_variant_names)]
#[derive(Debug, Error)]
pub enum GenerateArtifactsError {
    #[error(
        "Unable to write to artifact file at path {path:?}. \
        Is there another instance of the Isograph compiler running?\
        \nReason: {message:?}"
    )]
    UnableToWriteToArtifactFile { path: PathBuf, message: String },

    #[error(
        "Unable to create directory at path {path:?}. \
        Is there another instance of the Isograph compiler running?\
        \nReason: {message:?}"
    )]
    UnableToCreateDirectory { path: PathBuf, message: String },

    #[error(
        "Unable to delete directory at path {path:?}. \
        Is there another instance of the Isograph compiler running?\
        \nReason: {message:?}"
    )]
    UnableToDeleteDirectory { path: PathBuf, message: String },
}
