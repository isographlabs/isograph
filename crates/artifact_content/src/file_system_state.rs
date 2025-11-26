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

        let mut new_server_objects = HashSet::new();
        let mut new_selectables = HashSet::new();

        for (server_object, selectables) in &new.nested_files {
            new_server_objects.insert(*server_object);
            let server_object_path = artifact_directory.join(server_object);

            for (selectable, files) in selectables {
                new_selectables.insert((*server_object, *selectable));
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
                .map(|(_, old_hash)| old_hash != new_hash)
                .unwrap_or(true);

            if should_write {
                operations.push(FileSystemOperation::WriteFile(file_path, new_index.clone()));
            }
        }

        for (server_object, selectables) in &old.nested_files {
            let server_object_path = artifact_directory.join(server_object);

            if !new_server_objects.contains(server_object) {
                operations.push(FileSystemOperation::DeleteDirectory(server_object_path));
                continue;
            }

            for (selectable, files) in selectables {
                let selectable_path = server_object_path.join(selectable);

                if !new_selectables.contains(&(*server_object, *selectable)) {
                    operations.push(FileSystemOperation::DeleteDirectory(selectable_path));
                    continue;
                }

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

#[cfg(test)]
mod tests {
    use super::*;
    use common_lang_types::{
        ArtifactPath, ParentObjectEntityNameAndSelectableName, SelectableName,
        ServerObjectEntityName,
    };
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
                parent_object_entity_name: ServerObjectEntityName::from(s.intern()),
                selectable_name: SelectableName::from(sel.intern()),
            }),
            _ => None,
        };

        ArtifactPathAndContent {
            artifact_path: ArtifactPath {
                type_and_field,
                file_name: ArtifactFileName::from(file_name.intern()),
            },
            file_content: FileContent::from(content.to_string()),
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
                .contains_key(&ArtifactFileName::from("package.json".intern()))
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

        let server = &ServerObjectEntityName::from("User".intern());
        let selectable = SelectableName::from("name".intern());
        let file_name = &ArtifactFileName::from("query.graphql".intern());

        assert!(
            state
                .nested_files
                .get(server)
                .and_then(|s| s.get(&selectable))
                .and_then(|f| f.get(file_name))
                .is_some()
        );
    }

    #[test]
    fn test_from_artifacts() {
        let artifacts = vec![
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

        let user_server = &ServerObjectEntityName::from("User".intern());
        let selectables = state.nested_files.get(user_server).unwrap();
        assert_eq!(selectables.len(), 2);
    }

    #[test]
    fn test_diff_empty_to_new() {
        let artifacts = vec![create_artifact(
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
        let artifact1 = vec![create_artifact(None, None, "file.txt", "content")];
        let artifact2 = vec![create_artifact(None, None, "file.txt", "content")];

        let old_state = FileSystemState::from(&artifact1[..]);
        let new_state = FileSystemState::from(&artifact2[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);
        assert_eq!(ops.len(), 0);
    }

    #[test]
    fn test_diff_file_content_() {
        let old_artifacts = vec![create_artifact(None, None, "file.txt", "old content")];
        let new_artifacts = vec![create_artifact(None, None, "file.txt", "new content")];

        let old_state = FileSystemState::from(&old_artifacts[..]);
        let new_state = FileSystemState::from(&new_artifacts[..]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = FileSystemState::diff(&old_state, &new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_add_new_file() {
        let old_artifacts = vec![create_artifact(None, None, "existing.txt", "content")];
        let new_artifacts = vec![
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
        let old_artifacts = vec![
            create_artifact(None, None, "keep.txt", "content"),
            create_artifact(None, None, "delete.txt", "content"),
        ];
        let new_artifacts = vec![create_artifact(None, None, "keep.txt", "content")];

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
        let old_artifacts = vec![create_artifact(
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
        let old_artifacts = vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "old query",
        )];
        let new_artifacts = vec![create_artifact(
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
        let old_artifacts = vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )];
        let new_artifacts = vec![
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
        let old_artifacts = vec![
            create_artifact(None, None, "root.txt", "old root"),
            create_artifact(Some("User"), Some("name"), "query.graphql", "old query"),
            create_artifact(Some("User"), Some("email"), "query.graphql", "delete me"),
            create_artifact(Some("Post"), Some("title"), "query.graphql", "post query"),
        ];
        let new_artifacts = vec![
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
