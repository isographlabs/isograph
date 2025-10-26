use std::collections::HashMap;

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, legacy_memo};

#[derive(Default)]
pub struct TestMap(HashMap<String, SourceId<Input>>);

#[derive(Db, Default)]
struct TestDatabase {
    storage: Storage<Self>,
    #[tracked]
    map: TestMap,
}

#[test]
fn tracking_field_correctness() {
    let mut db = TestDatabase::default();

    // Seed the database
    let input_id_1 = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key".to_string(), input_id_1);
    // The first time we run a function, it will always have the correct value.
    assert_eq!(get_values_untracked(&db).len(), 1);

    let input_id_2 = db.set(Input {
        key: "key2",
        value: "isograph".to_string(),
    });
    // If we insert a new node without tracking and call untracked iter(),
    // we get the previous value
    db.get_map_mut()
        .tracked()
        .0
        .insert("key2".to_string(), input_id_2);
    assert_eq!(get_values_untracked(&db).len(), 1);

    // What? We're showing incorrect results? Why?!
    // Note that you should never do an untracked iter! Untracked is for reading
    // a single value in a Map<Key, SourceId>.
    //
    // See [super::efficiency] for correct usage of untracked.

    // Instead we have to use tracked iter together with tracked insert
    // (first call here, so value is correct anyway)
    assert_eq!(get_values_tracked(&db).len(), 2);

    let input_id_3 = db.set(Input {
        key: "key3",
        value: "database".to_string(),
    });
    db.get_map_mut()
        .tracked()
        .0
        .insert("key3".to_string(), input_id_3);
    // and now we got a correct value!
    assert_eq!(get_values_tracked(&db).len(), 3);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[legacy_memo]
fn get_values_tracked(db: &TestDatabase) -> Vec<String> {
    let mut result = vec![];
    for (_, input_id) in db.get_map().tracked().0.iter() {
        let input = db.get(*input_id);
        result.push(input.value.clone())
    }
    result
}

#[legacy_memo]
fn get_values_untracked(db: &TestDatabase) -> Vec<String> {
    let mut result = vec![];
    for (_, input_id) in db.get_map().untracked().0.iter() {
        let input = db.get(*input_id);
        result.push(input.value.clone())
    }
    result
}
