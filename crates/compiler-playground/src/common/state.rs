use pico_core::storage::Storage;
use pico_macros::Db;

#[derive(Debug, Db)]
pub struct State {
    pub storage: Storage<Self>,
}

impl State {
    pub fn new() -> Self {
        Self {
            storage: Storage::default(),
        }
    }
}
