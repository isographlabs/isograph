use std::time::{Duration, Instant};

use common_lang_types::{CurrentWorkingDirectory, Diagnostic};
use isograph_config::CompilerConfig;
use isograph_schema::{IsographDatabase, NetworkProtocol};
use pico::Database;
use prelude::Postfix;

use crate::source_files::initialize_sources;
use artifact_content::FileSystemState;

const GC_DURATION_SECONDS: u64 = 60;

#[derive(Debug)]
pub struct CompilerState<TNetworkProtocol: NetworkProtocol> {
    pub db: IsographDatabase<TNetworkProtocol>,
    pub last_gc_run: Instant,
    // TODO this doesn't belong here... it's watch-mode-specific
    pub file_system_state: Option<FileSystemState>,
}

impl<TNetworkProtocol: NetworkProtocol> CompilerState<TNetworkProtocol> {
    pub fn new(
        config: CompilerConfig,
        current_working_directory: CurrentWorkingDirectory,
    ) -> Result<Self, Diagnostic> {
        let mut db = IsographDatabase::default();
        db.set(current_working_directory);
        db.set(config);
        initialize_sources(&mut db)?;
        Self {
            db,
            last_gc_run: Instant::now(),
            file_system_state: None,
        }
        .wrap_ok()
    }

    pub fn run_garbage_collection(&mut self) {
        if self.last_gc_run.elapsed() >= Duration::from_secs(GC_DURATION_SECONDS) {
            self.db.run_garbage_collection();
            self.last_gc_run = Instant::now();
        }
    }
}
