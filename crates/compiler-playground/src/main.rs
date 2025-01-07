use std::io;

use common::{
    database::Database,
    owned::{evaluate_input, Input},
};
use pico_core::database::Database as Db;
use tracing::{debug, info};
use tracing_subscriber::EnvFilter;

mod calc;
mod common;

fn main() {
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(EnvFilter::from_default_env())
        .with_writer(io::stderr)
        .init();

    let mut db = Database::new();
    let key = "expr1";

    let mut input = Input {
        key,
        value: "2 + 2 * 2".to_string(),
    };
    let id = db.set(input);
    let mut result = evaluate_input(&mut db, id);
    info!("result: {result}");

    input = Input {
        key,
        value: "3 * 2".to_string(),
    };
    let id = db.set(input);
    result = evaluate_input(&mut db, id);
    info!("result: {result}");
    debug!("db: {db:#?}");
}
