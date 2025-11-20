use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use common_lang_types::CurrentWorkingDirectory;
use isograph_config::create_config;
use isograph_schema::{IsographDatabase, NetworkProtocol};
use pico::Database;

use crate::{batch_compile::BatchCompileError, source_files::initialize_sources};

const GC_DURATION_SECONDS: u64 = 5;

#[derive(Debug)]
pub struct CompilerState<TNetworkProtocol: NetworkProtocol> {
    pub db: IsographDatabase<TNetworkProtocol>,
    pub last_gc_run: Instant,
}

impl<TNetworkProtocol: NetworkProtocol> CompilerState<TNetworkProtocol> {
    pub fn new(
        config_location: &PathBuf,
        current_working_directory: CurrentWorkingDirectory,
    ) -> Result<Self, BatchCompileError<TNetworkProtocol>> {
        let mut db = IsographDatabase::default();
        db.set(current_working_directory);
        db.set(create_config(config_location, current_working_directory));
        initialize_sources(&mut db)?;
        Ok(Self {
            db,
            last_gc_run: Instant::now(),
        })
    }

    pub fn run_garbage_collection(&mut self) {
        if self.last_gc_run.elapsed() >= Duration::from_secs(GC_DURATION_SECONDS) {
            eprintln!("running gc");
            self.db.run_garbage_collection();
            self.last_gc_run = Instant::now();
        }
    }
}
