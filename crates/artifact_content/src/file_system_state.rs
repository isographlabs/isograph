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
    fn test_empty_state() {
        let state = FileSystemState::default();
        assert!(state.is_empty());
    }

    #[test]
    fn test_insert_root_file() {
        let mut state = FileSystemState::default();
        let artifact = create_artifact(None, None, "package.json", "{}");

        state.insert(&artifact);

        assert!(!state.is_empty());
        assert_eq!(state.root_files.len(), 1);
        assert!(
            state
                .root_files
                .contains_key(&ArtifactFileName::from("package.json".intern()))
        );
    }

    #[test]
    fn test_insert_nested_file() {
        let mut state = FileSystemState::default();
        let artifact = create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query { user { name } }",
        );

        state.insert(&artifact);

        assert!(!state.is_empty());
        assert_eq!(state.nested_files.len(), 1);

        let server = &ServerObjectEntityName::from("User".intern());
        let selectable = &SelectableName::from("name".intern());
        let file_name = &ArtifactFileName::from("query.graphql".intern());

        assert!(
            state
                .nested_files
                .get(server)
                .and_then(|s| s.get(selectable))
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

        let state = FileSystemState::from_artifacts(artifacts);

        assert_eq!(state.root_files.len(), 1);
        assert_eq!(state.nested_files.len(), 1);

        let user_server = &ServerObjectEntityName::from("User".intern());
        let selectables = state.nested_files.get(user_server).unwrap();
        assert_eq!(selectables.len(), 2);
    }

    #[test]
    fn test_diff_empty_to_new() {
        let old_state = FileSystemState::default();
        let mut new_state = FileSystemState::default();

        new_state.insert(&create_artifact(None, None, "root.txt", "content"));
        new_state.insert(&create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        ));

        let artifact_dir = PathBuf::from("/artifacts");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert!(matches!(ops[0], FileSystemOperation::DeleteDirectory(_)));

        let create_dirs = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::CreateDirectory(_)))
            .count();
        let write_files = ops
            .iter()
            .filter(|op| matches!(op, FileSystemOperation::WriteFile(_, _)))
            .count();

        assert_eq!(create_dirs, 2); // User dir and name dir
        assert_eq!(write_files, 2); // root.txt and query.graphql
    }

    #[test]
    fn test_diff_no_changes() {
        let artifact1 = create_artifact(None, None, "file.txt", "content");
        let artifact2 = create_artifact(None, None, "file.txt", "content");

        let old_state = FileSystemState::from_artifacts(vec![artifact1]);
        let new_state = FileSystemState::from_artifacts(vec![artifact2]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 0);
    }

    #[test]
    fn test_diff_file_content_changed() {
        let old_artifact = create_artifact(None, None, "file.txt", "old content");
        let new_artifact = create_artifact(None, None, "file.txt", "new content");

        let old_state = FileSystemState::from_artifacts(vec![old_artifact]);
        let new_state = FileSystemState::from_artifacts(vec![new_artifact]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_add_new_file() {
        let old_state = FileSystemState::from_artifacts(vec![create_artifact(
            None,
            None,
            "existing.txt",
            "content",
        )]);

        let new_state = FileSystemState::from_artifacts(vec![
            create_artifact(None, None, "existing.txt", "content"),
            create_artifact(None, None, "new.txt", "new content"),
        ]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        if let FileSystemOperation::WriteFile(path, _) = &ops[0] {
            assert!(path.ends_with("new.txt"));
        } else {
            panic!("Expected WriteFile operation");
        }
    }

    #[test]
    fn test_diff_delete_file() {
        let old_state = FileSystemState::from_artifacts(vec![
            create_artifact(None, None, "keep.txt", "content"),
            create_artifact(None, None, "delete.txt", "content"),
        ]);

        let new_state = FileSystemState::from_artifacts(vec![create_artifact(
            None, None, "keep.txt", "content",
        )]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        if let FileSystemOperation::DeleteFile(path) = &ops[0] {
            assert!(path.ends_with("delete.txt"));
        } else {
            panic!("Expected DeleteFile operation");
        }
    }

    #[test]
    fn test_diff_delete_empty_directory() {
        let old_state = FileSystemState::from_artifacts(vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )]);

        let new_state = FileSystemState::default();

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 3);
        assert!(matches!(ops[0], FileSystemOperation::DeleteFile(_)));
        assert!(matches!(ops[1], FileSystemOperation::DeleteDirectory(_)));
        assert!(matches!(ops[2], FileSystemOperation::DeleteDirectory(_)));
    }

    #[test]
    fn test_diff_nested_file_changes() {
        let old_state = FileSystemState::from_artifacts(vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "old query",
        )]);

        let new_state = FileSystemState::from_artifacts(vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "new query",
        )]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 1);
        assert!(matches!(ops[0], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_add_new_selectable() {
        let old_state = FileSystemState::from_artifacts(vec![create_artifact(
            Some("User"),
            Some("name"),
            "query.graphql",
            "query",
        )]);

        let new_state = FileSystemState::from_artifacts(vec![
            create_artifact(Some("User"), Some("name"), "query.graphql", "query"),
            create_artifact(Some("User"), Some("email"), "query.graphql", "query"),
        ]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

        assert_eq!(ops.len(), 2);
        assert!(matches!(ops[0], FileSystemOperation::CreateDirectory(_)));
        assert!(matches!(ops[1], FileSystemOperation::WriteFile(_, _)));
    }

    #[test]
    fn test_diff_complex_scenario() {
        let old_state = FileSystemState::from_artifacts(vec![
            create_artifact(None, None, "root.txt", "old root"),
            create_artifact(Some("User"), Some("name"), "query.graphql", "old query"),
            create_artifact(Some("User"), Some("email"), "query.graphql", "delete me"),
            create_artifact(Some("Post"), Some("title"), "query.graphql", "post query"),
        ]);

        let new_state = FileSystemState::from_artifacts(vec![
            create_artifact(None, None, "root.txt", "new root"), // changed
            create_artifact(None, None, "new_root.txt", "new file"), // added
            create_artifact(Some("User"), Some("name"), "query.graphql", "new query"), // changed
            create_artifact(Some("Post"), Some("title"), "query.graphql", "post query"), // unchanged
            create_artifact(Some("Comment"), Some("text"), "query.graphql", "comment"), // new server
        ]);

        let artifact_dir = PathBuf::from("/__isograph");
        let ops = old_state.diff(&new_state, &artifact_dir);

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
        assert_eq!(deletes, 1); // User/email/query.graphql
        assert!(create_dirs >= 2); // Comment and Comment/text
        assert_eq!(delete_dirs, 1); // User/email directory
    }
}
