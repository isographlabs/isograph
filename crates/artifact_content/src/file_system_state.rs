use pico::Index;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
};

use crate::operation_text::hash;
use common_lang_types::{
    ArtifactFileName, ArtifactHash, ArtifactPathAndContent, FileContent, FileSystemOperation,
    SelectableName, ServerObjectEntityName,
};
use isograph_config::PersistedDocumentsHashAlgorithm;

#[derive(Debug, Clone, Default)]
pub struct FileSystemState {
    root_files: HashMap<ArtifactFileName, (Index<FileContent>, ArtifactHash)>,
    nested_files: HashMap<
        ServerObjectEntityName,
        HashMap<SelectableName, HashMap<ArtifactFileName, (Index<FileContent>, ArtifactHash)>>,
    >,
}

impl FileSystemState {
    pub fn recreate_all(state: &Self, artifact_directory: &PathBuf) -> Vec<FileSystemOperation> {
        let mut operations: Vec<FileSystemOperation> = Vec::new();
        operations.push(FileSystemOperation::DeleteDirectory(
            artifact_directory.to_path_buf(),
        ));

        for (server_object, selectables) in &state.nested_files {
            let server_object_path = artifact_directory.join(server_object);

            for (selectable, files) in selectables {
                let selectable_path = server_object_path.join(selectable);
                operations.push(FileSystemOperation::CreateDirectory(
                    selectable_path.clone(),
                ));

                for (file_name, (index, _)) in files {
                    let file_path = selectable_path.join(file_name);
                    operations.push(FileSystemOperation::WriteFile(file_path, index.clone()));
                }
            }
        }

        for (file_name, (index, _)) in &state.root_files {
            let file_path = artifact_directory.join(file_name);
            operations.push(FileSystemOperation::WriteFile(file_path, index.clone()));
        }

        operations
    }

    pub fn diff(old: &Self, new: &Self, artifact_directory: &PathBuf) -> Vec<FileSystemOperation> {
        let mut operations: Vec<FileSystemOperation> = Vec::new();

        let mut new_server_objects: HashSet<ServerObjectEntityName> = HashSet::new();
        let mut new_selectables: HashSet<(ServerObjectEntityName, &SelectableName)> =
            HashSet::new();

        for (server_object, selectables) in &new.nested_files {
            new_server_objects.insert(*server_object);
            let server_object_path = artifact_directory.join(server_object);

            for (selectable, files) in selectables {
                new_selectables.insert((*server_object, selectable));
                let selectable_path = server_object_path.join(selectable);

                let should_create_dir = old
                    .nested_files
                    .get(server_object)
                    .and_then(|s| s.get(selectable))
                    .is_none();

                if should_create_dir {
                    operations.push(FileSystemOperation::CreateDirectory(
                        selectable_path.clone(),
                    ));
                }

                for (file_name, (new_index, new_hash)) in files {
                    let file_path = selectable_path.join(file_name);

                    let should_write = old
                        .nested_files
                        .get(server_object)
                        .and_then(|s| s.get(selectable))
                        .and_then(|f| f.get(file_name))
                        .map(|(_, old_hash)| old_hash != new_hash)
                        .unwrap_or(true);

                    if should_write {
                        operations
                            .push(FileSystemOperation::WriteFile(file_path, new_index.clone()));
                    }
                }
            }
        }

        for (file_name, (new_index, new_hash)) in &new.root_files {
            let file_path = artifact_directory.join(file_name);

            let should_write = old
                .root_files
                .get(file_name)
                .map(|(_old_content, old_hash)| old_hash != new_hash)
                .unwrap_or(true);

            if should_write {
                operations.push(FileSystemOperation::WriteFile(file_path, new_index.clone()));
            }
        }

        for (server_object, selectables) in &old.nested_files {
            let server_object_path = artifact_directory.join(server_object);

            for (selectable, files) in selectables {
                let selectable_path = server_object_path.join(selectable);

                for file_name in files.keys() {
                    let exist_in_new = new
                        .nested_files
                        .get(server_object)
                        .and_then(|s| s.get(selectable))
                        .and_then(|f| f.get(file_name))
                        .is_some();

                    if !exist_in_new {
                        let file_path = selectable_path.join(file_name);
                        operations.push(FileSystemOperation::DeleteFile(file_path));
                    }
                }

                if !new_selectables.contains(&(*server_object, selectable)) {
                    operations.push(FileSystemOperation::DeleteDirectory(selectable_path));
                }
            }

            if !new_server_objects.contains(server_object) {
                operations.push(FileSystemOperation::DeleteDirectory(server_object_path));
            }
        }

        for file_name in old.root_files.keys() {
            if !new.root_files.contains_key(file_name) {
                let file_path = artifact_directory.join(file_name);
                operations.push(FileSystemOperation::DeleteFile(file_path));
            }
        }

        return operations;
    }
}

impl From<&[ArtifactPathAndContent]> for FileSystemState {
    fn from(artifacts: &[ArtifactPathAndContent]) -> Self {
        let mut root_files = HashMap::new();
        let mut nested_files: HashMap<
            ServerObjectEntityName,
            HashMap<SelectableName, HashMap<ArtifactFileName, (Index<FileContent>, ArtifactHash)>>,
        > = HashMap::new();

        for (index, artifact) in artifacts.iter().enumerate() {
            let value = (
                Index::new(index),
                ArtifactHash::from(hash(
                    &artifact.file_content,
                    PersistedDocumentsHashAlgorithm::Md5,
                )),
            );

            match &artifact.artifact_path.type_and_field {
                Some(type_and_field) => {
                    nested_files
                        .entry(type_and_field.parent_object_entity_name)
                        .or_default()
                        .entry(type_and_field.selectable_name)
                        .or_default()
                        .insert(artifact.artifact_path.file_name, value);
                }
                None => {
                    root_files.insert(artifact.artifact_path.file_name, value);
                }
            }
        }

        FileSystemState {
            root_files,
            nested_files,
        }
    }
}
