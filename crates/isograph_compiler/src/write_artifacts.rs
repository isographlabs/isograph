use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::PathBuf,
};

use common_lang_types::ArtifactPathAndContent;
use intern::string_key::Lookup;
use thiserror::Error;

use crate::changed_artifacts::ChangedArtifacts;
use artifact_content::FileSystemState;

pub fn get_artifacts_to_write(
    paths_and_contents: impl IntoIterator<Item = ArtifactPathAndContent>,
    artifact_directory: &PathBuf,
    file_system_state: &mut FileSystemState,
) -> ChangedArtifacts {
    let mut new_file_system_state = FileSystemState::new();
    let mut artifact_map: HashMap<String, (PathBuf, ArtifactPathAndContent)> = HashMap::new();

    for path_and_content in paths_and_contents {
        let absolute_directory = match path_and_content.type_and_field {
            Some(type_and_field) => artifact_directory
                .join(type_and_field.parent_object_entity_name.lookup())
                .join(type_and_field.selectable_name.lookup()),
            None => artifact_directory.clone(),
        };

        let absolute_file_path = absolute_directory.join(path_and_content.file_name.lookup());

        let relative_file_path = absolute_file_path
            .strip_prefix(artifact_directory)
            .expect("absolute paths should contain artifact_directory")
            .to_string_lossy()
            .to_string();

        new_file_system_state.insert(
            relative_file_path.clone(),
            path_and_content.file_content.clone(),
        );
        artifact_map.insert(relative_file_path, (absolute_file_path, path_and_content));
    }

    let mut artifacts_to_disk = ChangedArtifacts::new();

    if file_system_state.is_empty() {
        artifacts_to_disk.artifacts_to_write = artifact_map
            .into_iter()
            .map(|(_, (path, content))| (path, content))
            .collect();
        artifacts_to_disk.cleanup_artifact_directory = true;
    } else {
        let (to_delete, to_add) = file_system_state.compare(&new_file_system_state);

        for relative_path in to_add.into_iter() {
            if let Some((absolute_path, content)) = artifact_map.remove(&relative_path) {
                artifacts_to_disk
                    .artifacts_to_write
                    .insert(absolute_path, content);
            }
        }

        artifacts_to_disk.artifacts_to_delete = to_delete
            .into_iter()
            .map(|path| artifact_directory.join(path))
            .collect();
        artifacts_to_disk.cleanup_artifact_directory = false;
    }

    *file_system_state = new_file_system_state;

    artifacts_to_disk
}

#[tracing::instrument(skip(artifacts_to_disk, artifact_directory))]
pub(crate) fn write_artifacts_to_disk(
    artifacts_to_disk: ChangedArtifacts,
    artifact_directory: &PathBuf,
) -> Result<usize, GenerateArtifactsError> {
    if artifact_directory.exists() && artifacts_to_disk.cleanup_artifact_directory {
        fs::remove_dir_all(artifact_directory).map_err(|e| {
            GenerateArtifactsError::UnableToDeleteDirectory {
                path: artifact_directory.clone(),
                message: e.to_string(),
            }
        })?;

        fs::create_dir_all(artifact_directory).map_err(|e| {
            GenerateArtifactsError::UnableToCreateDirectory {
                path: artifact_directory.clone(),
                message: e.to_string(),
            }
        })?;
    }

    let mut count = 0;
    for (path, content) in artifacts_to_disk.artifacts_to_write.iter() {
        count += 1;

        let absolute_directory = path.parent().expect("path must have a parent");

        fs::create_dir_all(&absolute_directory).map_err(|e| {
            GenerateArtifactsError::UnableToCreateDirectory {
                path: absolute_directory.to_path_buf().clone(),
                message: e.to_string(),
            }
        })?;

        let mut file = File::create(&path).map_err(|e| {
            GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: path.clone(),
                message: e.to_string(),
            }
        })?;

        file.write(content.file_content.as_bytes()).map_err(|e| {
            GenerateArtifactsError::UnableToWriteToArtifactFile {
                path: path.clone(),
                message: e.to_string(),
            }
        })?;
    }

    for path in artifacts_to_disk.artifacts_to_delete.iter() {
        fs::remove_file(path).map_err(|e| GenerateArtifactsError::UnableToDeleteArtifactFile {
            path: path.clone(),
            message: e.to_string(),
        })?;
    }

    Ok(count)
}

#[expect(clippy::enum_variant_names)]
#[derive(Debug, Error, PartialEq, Eq, Clone)]
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

    #[error(
        "Unable to delete artifact file at path {path:?}. \
        Is there another instance of the Isograph compiler running?\
        \nReason: {message:?}"
    )]
    UnableToDeleteArtifactFile { path: PathBuf, message: String },
}
