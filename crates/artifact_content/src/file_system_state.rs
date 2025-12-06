use pico::Index;
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

use crate::operation_text::hash;
use common_lang_types::{
    ArtifactFileName, ArtifactHash, ArtifactPathAndContent, EntityName, FileContent,
    FileSystemOperation, SelectableName,
};
use isograph_config::PersistedDocumentsHashAlgorithm;

#[derive(Debug, Clone, Default)]
#[expect(clippy::type_complexity)]
pub struct FileSystemState {
    root_files: HashMap<ArtifactFileName, (Index<FileContent>, ArtifactHash)>,
    nested_files: HashMap<
        EntityName,
        HashMap<SelectableName, HashMap<ArtifactFileName, (Index<FileContent>, ArtifactHash)>>,
    >,
}

impl FileSystemState {
    pub fn recreate_all(state: &Self, artifact_directory: &Path) -> Vec<FileSystemOperation> {
        let mut operations: Vec<FileSystemOperation> = Vec::new();
        operations.push(FileSystemOperation::DeleteDirectory(
            artifact_directory.to_path_buf(),
        ));

        for (new_server_object_entity_name, new_selectable_map) in &state.nested_files {
            let new_server_object_path = artifact_directory.join(new_server_object_entity_name);

            for (new_selectable, new_files) in new_selectable_map {
                let new_selectable_path = new_server_object_path.join(new_selectable);
                operations.push(FileSystemOperation::CreateDirectory(
                    new_selectable_path.clone(),
                ));

                for (new_file_name, (new_index, _)) in new_files {
                    let new_file_path = new_selectable_path.join(new_file_name);
                    operations.push(FileSystemOperation::WriteFile(
                        new_file_path,
                        new_index.clone(),
                    ));
                }
            }
        }

        for (new_file_name, (new_index, _)) in &state.root_files {
            let new_file_path = artifact_directory.join(new_file_name);
            operations.push(FileSystemOperation::WriteFile(
                new_file_path,
                new_index.clone(),
            ));
        }

        operations
    }

    pub fn diff(old: &Self, new: &Self, artifact_directory: &Path) -> Vec<FileSystemOperation> {
        let mut operations: Vec<FileSystemOperation> = Vec::new();

        let mut new_server_object_entity_name_set = HashSet::new();
        let mut new_selectable_set = HashSet::new();

        for (new_server_object_entity_name, new_selectable_map) in &new.nested_files {
            new_server_object_entity_name_set.insert(*new_server_object_entity_name);
            let new_server_object_path = artifact_directory.join(*new_server_object_entity_name);

            let old_selectables_for_object = old.nested_files.get(new_server_object_entity_name);

            for (new_selectable, new_files) in new_selectable_map {
                new_selectable_set.insert((*new_server_object_entity_name, *new_selectable));
                let new_selectable_path = new_server_object_path.join(new_selectable);
                let old_files_for_selectable =
                    old_selectables_for_object.and_then(|s| s.get(new_selectable));

                if old_files_for_selectable.is_none() {
                    operations.push(FileSystemOperation::CreateDirectory(
                        new_selectable_path.clone(),
                    ));
                }

                for (new_file_name, (new_index, new_hash)) in new_files {
                    let new_file_path = new_selectable_path.join(new_file_name);

                    let old_file = old_files_for_selectable.and_then(|f| f.get(new_file_name));

                    let should_write = old_file
                        .map(|(_, old_hash)| old_hash != new_hash)
                        .unwrap_or(true);

                    if should_write {
                        operations.push(FileSystemOperation::WriteFile(
                            new_file_path,
                            new_index.clone(),
                        ));
                    }
                }
            }
        }

        for (new_file_name, (new_index, new_hash)) in &new.root_files {
            let new_file_path = artifact_directory.join(new_file_name);

            let should_write = old
                .root_files
                .get(new_file_name)
                .map(|(_, old_hash)| old_hash != new_hash)
                .unwrap_or(true);

            if should_write {
                operations.push(FileSystemOperation::WriteFile(
                    new_file_path,
                    new_index.clone(),
                ));
            }
        }

        for (old_server_object_entity_name, old_selectable_map) in &old.nested_files {
            let old_server_object_path = artifact_directory.join(*old_server_object_entity_name);

            if !new_server_object_entity_name_set.contains(old_server_object_entity_name) {
                operations.push(FileSystemOperation::DeleteDirectory(old_server_object_path));
                continue;
            }

            let new_selectable_map_for_object = new.nested_files.get(old_server_object_entity_name);

            for (old_selectable, old_files) in old_selectable_map {
                let old_selectable_path = old_server_object_path.join(old_selectable);

                if !new_selectable_set.contains(&(*old_server_object_entity_name, *old_selectable))
                {
                    operations.push(FileSystemOperation::DeleteDirectory(old_selectable_path));
                    continue;
                }

                let new_files_for_selectable =
                    new_selectable_map_for_object.and_then(|s| s.get(old_selectable));

                for file_name in old_files.keys() {
                    let new_file = new_files_for_selectable.and_then(|f| f.get(file_name));

                    if new_file.is_none() {
                        let file_path = old_selectable_path.join(file_name);
                        operations.push(FileSystemOperation::DeleteFile(file_path));
                    }
                }
            }
        }

        for file_name in old.root_files.keys() {
            if !new.root_files.contains_key(file_name) {
                let file_path = artifact_directory.join(file_name);
                operations.push(FileSystemOperation::DeleteFile(file_path));
            }
        }

        operations
    }
}

#[expect(clippy::type_complexity)]
impl From<&[ArtifactPathAndContent]> for FileSystemState {
    fn from(artifacts: &[ArtifactPathAndContent]) -> Self {
        let mut root_files = HashMap::new();
        let mut nested_files: HashMap<
            EntityName,
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

#[cfg(test)]
mod tests {
    use super::*;
    use common_lang_types::{ArtifactPath, ParentObjectEntityNameAndSelectableName};
    use intern::string_key::Intern;
    use std::path::PathBuf;

    fn create_artifact(
        server: Option<&str>,
        selectable: Option<&str>,
        file_name: &str,
        content: &str,
    ) -> ArtifactPathAndContent {
        let type_and_field = match (server, selectable) {
            (Some(s), Some(sel)) => Some(ParentObjectEntityNameAndSelectableName {
                parent_object_entity_name: s.intern().into(),
                selectable_name: sel.intern().into(),
            }),
            _ => None,
        };

        ArtifactPathAndContent {
            artifact_path: ArtifactPath {
                type_and_field,
                file_name: file_name.intern().into(),
            },
            file_content: content.to_string().into(),
        }
    }

    #[test]
    fn test_insert_root_file() {
        let artifact = create_artifact(None, None, "package.json", "{}");
        let state = FileSystemState::from(&[artifact][..]);

        assert_eq!(state.root_files.len(), 1);
        assert!(
            state
                .root_files
                .contains_key(&"package.json".intern().into())
        );
    }

    #[test]
    fn test_insert_nested_file() {
        let artifact = create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query { user { name } }",
        );

        let state = FileSystemState::from(&[artifact][..]);

        assert_eq!(state.nested_files.len(), 1);

        let server = "User".intern().into();
        let selectable = "name".intern().into();
        let file_name = "query.graphql".intern().into();

        assert!(
            state
                .nested_files
                .get(&server)
                .and_then(|s| s.get(&selectable))
                .and_then(|f| f.get(&file_name))
                .is_some()
        );
    }

    #[test]
    fn test_from_artifacts() {
        let artifacts = [
            create_artifact(None, None, "schema.graphql", "type Query"),
            create_artifact(Some("User"), Some("name"), "query.graphql", "query {}"),
            create_artifact(
                Some("User"),
                Some("email"),
                "mutation.graphql",
                "mutation {}",
            ),
        ];

        let state = FileSystemState::from(&artifacts[..]);

        assert_eq!(state.root_files.len(), 1);
        assert_eq!(state.nested_files.len(), 1);

        let user_server = "User".intern().into();
        let selectables = state.nested_files.get(&user_server).unwrap();
        assert_eq!(selectables.len(), 2);
    }

    #[test]
    fn test_diff_empty_to_new() {
        let artifacts = [create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )];
        let new_state = FileSystemState::from(&artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::recreate_all(&new_state, &artifact_dir);

        assert!(matches!(ops[0], FileSystemOperation::DeleteDirectory(_)));

        let create_dirs = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::CreateDirectory(_)))
            .count();
        let write_files = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::WriteFile(_, _)))
            .count();

        assert_eq!(create_dirs, 1);
        assert_eq!(write_files, 1);
    }

    #[test]
    fn test_diff_no_changes() {
        let artifact1 = [create_artifact(None, None, "file.txt", "content")];
        let artifact2 = [create_artifact(None, None, "file.txt", "content")];

        let old_state = FileSystemState::from(&artifact1[..]);
        let new_state = FileSystemState::from(&artifact2[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);
        assert_eq!(ops.len(), 0);
    }

    #[test]
    fn test_diff_file_content_() {
        let old_artifacts = [create_artifact(None, None, "file.txt", "old content")];
        let new_artifacts = [create_artifact(None, None, "file.txt", "new content")];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_add_new_file() {
        let old_artifacts = [create_artifact(None, None, "existing.txt", "content")];
        let new_artifacts = [
            create_artifact(None, None, "existing.txt", "content"),
            create_artifact(None, None, "new.txt", "new content"),
        ];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        if let FileSystemOperation::WriteFile(path, _) = &ops[0] {
            assert!(path.ends_with("new.txt"));
        } else {
            panic!("Expected WriteFile operation");
        }
    }

    #[test]
    fn test_diff_delete_file() {
        let old_artifacts = [
            create_artifact(None, None, "keep.txt", "content"),
            create_artifact(None, None, "delete.txt", "content"),
        ];
        let new_artifacts = [create_artifact(None, None, "keep.txt", "content")];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        if let FileSystemOperation::DeleteFile(path) = &ops[0] {
            assert!(path.ends_with("delete.txt"));
        } else {
            panic!("Expected DeleteFile operation");
        }
    }

    #[test]
    fn test_diff_delete_empty_directory() {
        let old_artifacts = [create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )];
        let old_state = FileSystemState::from(&old_artifacts[..]);

        let new_state = FileSystemState::default();

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::DeleteDirectory(_)));
    }

    #[test]
    fn test_diff_nested_file_changes() {
        let old_artifacts = [create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "old query",
        )];
        let new_artifacts = [create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "new query",
        )];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_add_new_selectable() {
        let old_artifacts = [create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )];
        let new_artifacts = [
            create_artifact(Some("User"), Some("name"), "query.graphql", "query"),
            create_artifact(Some("User"), Some("email"), "query.graphql", "query"),
        ];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], FileSystemOperation::CreateDirectory(_)));
        assert!(matches!(ops[1], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_complex_scenario() {
        let old_artifacts = [
            create_artifact(None, None, "root.txt", "old root"),
            create_artifact(Some("User"), Some("name"), "query.graphql", "old query"),
            create_artifact(Some("User"), Some("email"), "query.graphql", "delete me"),
            create_artifact(Some("Post"), Some("title"), "query.graphql", "post query"),
        ];
        let new_artifacts = [
            create_artifact(None, None, "root.txt", "new root"),
            create_artifact(None, None, "new_root.txt", "new file"),
            create_artifact(Some("User"), Some("name"), "query.graphql", "new query"),
            create_artifact(Some("Post"), Some("title"), "query.graphql", "post query"),
            create_artifact(Some("Comment"), Some("text"), "query.graphql", "comment"),
        ];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        let writes = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::WriteFile(_, _)))
            .count();
        let deletes = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::DeleteFile(_)))
            .count();
        let create_dirs = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::CreateDirectory(_)))
            .count();
        let delete_dirs = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::DeleteDirectory(_)))
            .count();

        assert!(writes >= 3); // root.txt, new_root.txt, User/name/query.graphql, Comment/text/query.graphql
        assert_eq!(deletes, 0);
        assert!(create_dirs >= 1); // Comment/text
        assert_eq!(delete_dirs, 1); // User/email directory
    }
}
