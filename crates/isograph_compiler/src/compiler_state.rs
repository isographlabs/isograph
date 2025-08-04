use std::{
    collections::HashMap,
    path::PathBuf,
    time::{Duration, Instant},
};

use common_lang_types::CurrentWorkingDirectory;
use isograph_config::create_config;
use isograph_lang_types::{IsographDatabase, OpenFileMap};
use isograph_schema::NetworkProtocol;
use pico::Database;

use crate::{batch_compile::BatchCompileError, source_files::initialize_sources};

const GC_DURATION_SECONDS: u64 = 60;

#[derive(Debug)]
pub struct CompilerState {
    pub db: IsographDatabase,
    pub last_gc_run: Instant,
}

impl CompilerState {
    pub fn new<TNetworkProtocol: NetworkProtocol + 'static>(
        config_location: &PathBuf,
        current_working_directory: CurrentWorkingDirectory,
    ) -> Result<Self, BatchCompileError<TNetworkProtocol>> {
        let mut db = IsographDatabase::default();
        db.set(current_working_directory);
        db.set(create_config(config_location, current_working_directory));
        db.set(OpenFileMap(HashMap::new()));
        initialize_sources(&mut db)?;
        Ok(Self {
            db,
            last_gc_run: Instant::now(),
        })
    }

    pub fn run_garbage_collection(&mut self) {
        if self.last_gc_run.elapsed() >= Duration::from_secs(GC_DURATION_SECONDS) {
            self.db.run_garbage_collection();
            self.last_gc_run = Instant::now();
        }
    }
}
