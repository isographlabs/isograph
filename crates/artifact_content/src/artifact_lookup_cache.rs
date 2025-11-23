use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArtifactLookupCache {
    hashes: HashMap<String, String>,
    sorted_keys: BTreeSet<String>,
}

impl ArtifactLookupCache {
    pub fn new() -> Self {
        Self {
            hashes: HashMap::new(),
            sorted_keys: BTreeSet::new(),
        }
    }

    pub fn insert(&mut self, filename: String, hash: String) {
        self.sorted_keys.insert(filename.clone());
        self.hashes.insert(filename, hash);
    }

    pub fn get(&self, filename: &str) -> Option<&String> {
        self.hashes.get(filename)
    }

    fn to_ordered_map(&self) -> BTreeMap<&String, &String> {
        self.sorted_keys
            .iter()
            .map(|k| (k, &self.hashes[k]))
            .collect()
    }

    pub fn write_to<W: std::io::Write>(&self, writer: W) -> Result<(), serde_json::Error> {
        serde_json::to_writer_pretty(writer, &self.to_ordered_map())
    }
}
