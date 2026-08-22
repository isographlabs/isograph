use std::collections::HashMap;
use std::marker::PhantomData;
use std::path::{Path, PathBuf};

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source};

use crate::HostLanguage;

#[derive(Debug, Db)]
pub struct IsographState<THostLanguage: HostLanguage> {
    storage: Storage<Self>,
    #[tracked]
    disk_file_map: DiskFileMap,
    phantom_data: PhantomData<THostLanguage>,
}

impl<THostLanguage: HostLanguage> Default for IsographState<THostLanguage> {
    fn default() -> Self {
        Self {
            storage: Storage::default(),
            disk_file_map: DiskFileMap::default(),
            phantom_data: PhantomData,
        }
    }
}

#[derive(Debug, Default)]
pub struct DiskFileMap(pub HashMap<PathBuf, SourceId<DiskFile>>);

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct DiskFile {
    #[key]
    pub path: PathBuf,
    pub contents: String,
}

impl<THostLanguage: HostLanguage> IsographState<THostLanguage> {
    pub fn insert_disk_file(&mut self, path: PathBuf, contents: String) {
        let source_id = self.set(DiskFile {
            path: path.clone(),
            contents,
        });
        self.get_disk_file_map_mut()
            .tracked()
            .0
            .insert(path, source_id);
    }

    pub fn remove_disk_file(&mut self, path: &Path) {
        if let Some(source_id) = self.get_disk_file_map_mut().tracked().0.remove(path) {
            self.remove(source_id);
        }
    }
}
