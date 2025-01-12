use pico::storage::DefaultStorage;
use pico_macros::Db;

#[derive(Debug, Db)]
pub struct State {
    pub storage: DefaultStorage<Self>,
}

impl State {
    pub fn new() -> Self {
        Self {
            storage: DefaultStorage::new(),
        }
    }
}
