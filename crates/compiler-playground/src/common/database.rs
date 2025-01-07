use pico::storage::DefaultStorage;
use pico_macros::Db;

#[derive(Debug, Db)]
pub struct Database {
    pub storage: DefaultStorage<Self>,
}

impl Database {
    pub fn new() -> Self {
        Self {
            storage: DefaultStorage::new(),
        }
    }
}
