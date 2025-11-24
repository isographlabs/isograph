use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

use crate::operation_text::hash;
use isograph_config::PersistedDocumentsHashAlgorithm;

type Filepath = String;
type FileContent = String;
type FileHash = String;

#[derive(Debug, Clone)]
pub struct FileSystemState {
    state: HashMap<Filepath, (FileContent, FileHash)>,
    pub sorted_keys: BTreeSet<String>,
}

impl FileSystemState {
    pub fn new() -> Self {
        Self {
            state: HashMap::new(),
            sorted_keys: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, filename: Filepath, content: FileContent) {
        let hashed_content = hash(&content, PersistedDocumentsHashAlgorithm::Sha256);
        self.sorted_keys.insert(filename.clone());
        self.state.insert(filename, (content, hashed_content));
    }

    pub fn get_hashed_content(&self, filename: &str) -> Option<&FileHash> {
        self.state
            .get(filename)
            .map(|(_, hashed_content)| hashed_content)
    }

    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    fn difference(&self, other: &FileSystemState) -> Vec<String> {
        self.sorted_keys
            .difference(&other.sorted_keys)
            .map(|k| k.clone())
            .collect()
    }

    fn intersection(&self, other: &FileSystemState) -> Vec<String> {
        self.sorted_keys
            .intersection(&other.sorted_keys)
            .map(|k| k.clone())
            .collect()
    }

    pub fn compare(&self, other: &FileSystemState) -> (Vec<String>, Vec<String>) {
        let to_delete = self.difference(other);
        let mut to_add = other.difference(self);
        let candidate_to_update = self.intersection(other);
        for key in candidate_to_update {
            if self.get_hashed_content(&key).unwrap() != other.get_hashed_content(&key).unwrap() {
                to_add.push(key);
            }
        }
        (to_delete, to_add)
    }
}

impl Default for FileSystemState {
    fn default() -> Self {
        Self::new()
    }
}
