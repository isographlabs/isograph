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
    root_files: HashMap<ArtifactFileName, (FileContent, ArtifactHash)>,
    nested_files: HashMap<
        ServerObjectEntityName,
        HashMap<SelectableName, HashMap<ArtifactFileName, (FileContent, ArtifactHash)>>,
    >,
}

impl FileSystemState {
    pub fn insert(&mut self, path_and_content: &ArtifactPathAndContent) {
        let value = (
            FileContent::from(path_and_content.file_content.clone()),
            ArtifactHash::from(hash(
                &path_and_content.file_content,
                PersistedDocumentsHashAlgorithm::Md5,
            )),
        );

        match &path_and_content.artifact_path.type_and_field {
            Some(type_and_field) => {
                self.nested_files
                    .entry(type_and_field.parent_object_entity_name)
                    .or_default()
                    .entry(type_and_field.selectable_name)
                    .or_default()
                    .insert(path_and_content.artifact_path.file_name, value);
            }
            None => {
                self.root_files
                    .insert(path_and_content.artifact_path.file_name, value);
            }
        }
    }

    pub fn from_artifacts(artifacts: Vec<ArtifactPathAndContent>) -> Self {
        let mut state = Self::default();
        for artifact in artifacts {
            state.insert(&artifact);
        }
        state
    }

    pub fn is_empty(&self) -> bool {
        self.root_files.is_empty() && self.nested_files.is_empty()
    }

    // Computes filesystem operations needed to transform this state into the target state.
    // Returns operations to create, update, and delete files and directories. Files are updated
    // only when their hash differs. If the current state is empty, emits a DeleteDirectory
    // operation for the artifact root followed by recreation of all files. Empty directories
    // are automatically removed after their files are deleted.
    pub fn diff(&self, new: &Self, artifact_directory: &PathBuf) -> Vec<FileSystemOperation> {
        let mut operations: Vec<FileSystemOperation> = Vec::new();

        if self.is_empty() {
            operations.push(FileSystemOperation::DeleteDirectory(
                artifact_directory.to_path_buf(),
            ));

            for (server_object, selectables) in &new.nested_files {
                let server_object_path = artifact_directory.join(server_object.to_string());
                operations.push(FileSystemOperation::CreateDirectory(
                    server_object_path.clone(),
                ));

                for (selectable, files) in selectables {
                    let selectable_path = server_object_path.join(selectable.to_string());
                    operations.push(FileSystemOperation::CreateDirectory(
                        selectable_path.clone(),
                    ));

                    for (file_name, (content, _)) in files {
                        let file_path = selectable_path.join(file_name.to_string());
                        operations.push(FileSystemOperation::WriteFile(file_path, content.clone()));
                    }
                }

                for (file_name, (content, _)) in &new.root_files {
                    let file_path = artifact_directory.join(file_name.to_string());
                    operations.push(FileSystemOperation::WriteFile(file_path, content.clone()));
                }
            }

            return operations;
        }

        let mut new_server_objects: HashSet<&ServerObjectEntityName> = HashSet::new();
        let mut new_selectables: HashSet<(&ServerObjectEntityName, &SelectableName)> =
            HashSet::new();

        for (server_object, selectables) in &new.nested_files {
            new_server_objects.insert(server_object);
            let server_object_path = artifact_directory.join(server_object.to_string());

            if !self.nested_files.contains_key(server_object) {
                operations.push(FileSystemOperation::CreateDirectory(
                    server_object_path.clone(),
                ));
            }

            for (selectable, files) in selectables {
                new_selectables.insert((server_object, selectable));
                let selectable_path = server_object_path.join(selectable.to_string());

                let should_create_dir = self
                    .nested_files
                    .get(server_object)
                    .and_then(|s| s.get(selectable))
                    .is_none();

                if should_create_dir {
                    operations.push(FileSystemOperation::CreateDirectory(
                        selectable_path.clone(),
                    ));
                }

                for (file_name, (new_content, new_hash)) in files {
                    let file_path = selectable_path.join(file_name.to_string());

                    let should_write = self
                        .nested_files
                        .get(server_object)
                        .and_then(|s| s.get(selectable))
                        .and_then(|f| f.get(file_name))
                        .map(|(_, old_hash)| old_hash != new_hash)
                        .unwrap_or(true);

                    if should_write {
                        operations.push(FileSystemOperation::WriteFile(
                            file_path,
                            new_content.clone(),
                        ));
                    }
                }
            }
        }

        for (file_name, (new_content, new_hash)) in &new.root_files {
            let file_path = artifact_directory.join(file_name.to_string());

            let should_write = self
                .root_files
                .get(file_name)
                .map(|(_old_content, old_hash)| old_hash != new_hash)
                .unwrap_or(true);

            if should_write {
                operations.push(FileSystemOperation::WriteFile(
                    file_path,
                    new_content.clone(),
                ));
            }
        }

        for (server_object, selectables) in &self.nested_files {
            let server_object_path = artifact_directory.join(server_object.to_string());

            for (selectable, files) in selectables {
                let selectable_path = server_object_path.join(selectable.to_string());

                for file_name in files.keys() {
                    let exist_in_new = new
                        .nested_files
                        .get(server_object)
                        .and_then(|s| s.get(selectable))
                        .and_then(|f| f.get(file_name))
                        .is_some();

                    if !exist_in_new {
                        let file_path = selectable_path.join(file_name.to_string());
                        operations.push(FileSystemOperation::DeleteFile(file_path));
                    }
                }

                if !new_selectables.contains(&(server_object, selectable)) {
                    operations.push(FileSystemOperation::DeleteDirectory(selectable_path));
                }
            }

            if !new_server_objects.contains(server_object) {
                operations.push(FileSystemOperation::DeleteDirectory(server_object_path));
            }
        }

        for file_name in self.root_files.keys() {
            if !new.root_files.contains_key(file_name) {
                let file_path = artifact_directory.join(file_name.to_string());
                operations.push(FileSystemOperation::DeleteFile(file_path));
            }
        }

        return operations;
    }
}
