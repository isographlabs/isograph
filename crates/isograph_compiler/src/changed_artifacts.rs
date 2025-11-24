use std::{collections::HashMap, path::PathBuf};

use common_lang_types::ArtifactPathAndContent;

pub struct ChangedArtifacts {
    pub committed_artifacts: HashMap<PathBuf, ArtifactPathAndContent>,
    pub artifacts_to_delete: Vec<PathBuf>,
    pub cleanup_artifact_directory: bool,
}

impl ChangedArtifacts {
    pub fn new() -> Self {
        Self {
            committed_artifacts: HashMap::new(),
            artifacts_to_delete: Vec::new(),
            cleanup_artifact_directory: true,
        }
    }
    pub fn delete(&mut self, paths: Vec<PathBuf>) {
        for path in paths {
            self.artifacts_to_delete.push(path.clone());
        }
    }
}

impl Default for ChangedArtifacts {
    fn default() -> Self {
        Self::new()
    }
}
