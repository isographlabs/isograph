use std::collections::HashMap;

use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

#[derive(Default)]
pub struct TestMap(HashMap<String, SourceId<Input>>);

#[derive(Db, Default)]
struct TestDatabase {
    pub storage: Storage<Self>,
    pub map: TestMap,
}

#[test]
fn tracking_field() {
    let mut db = TestDatabase::default();

    let input_id_1 = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    db.map.0.insert("key".to_string(), input_id_1);
    // First calculation is always correct, because we call inner_fn here
    assert_eq!(get_values_untracked(&db).len(), 1);

    let input_id_2 = db.set(Input {
        key: "key2",
        value: "isograph".to_string(),
    });
    // If we insert a new node without tracking and call untracked iter()
    // we got the previous value
    db.map.0.insert("key2".to_string(), input_id_2);
    assert_eq!(get_values_untracked(&db).len(), 1);

    let input_id_3 = db.set(Input {
        key: "key3",
        value: "pico".to_string(),
    });
    // Using tracking insert is not enough
    db.get_map_tracked_mut()
        .0
        .insert("key3".to_string(), input_id_3);
    assert_eq!(get_values_untracked(&db).len(), 1);

    // Instead we have to use tracked iter together with tracked insert
    // (first call here, so value is correct anyway)
    assert_eq!(get_values_tracked(&db).len(), 3);

    let input_id_4 = db.set(Input {
        key: "key4",
        value: "database".to_string(),
    });
    db.get_map_tracked_mut()
        .0
        .insert("key4".to_string(), input_id_4);
    // and now we got a correct value!
    assert_eq!(get_values_tracked(&db).len(), 4);

    let input_id_5 = db.set(Input {
        key: "key5",
        value: "qwerty".to_string(),
    });
    // we should not use untracked insert even if we use tracked iter
    db.map.0.insert("key5".to_string(), input_id_5);
    assert_eq!(get_values_tracked(&db).len(), 4);

    let input_id_6 = db.set(Input {
        key: "key6",
        value: "isographlabs".to_string(),
    });
    db.get_map_tracked_mut()
        .0
        .insert("key6".to_string(), input_id_6);
    assert_eq!(get_values_tracked(&db).len(), 6);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn get_values_tracked(db: &TestDatabase) -> Vec<String> {
    let mut result = vec![];
    for (_, input_id) in db.get_map_tracked().0.iter() {
        let input = db.get(*input_id);
        result.push(input.value.clone())
    }
    result
}

#[memo]
fn get_values_untracked(db: &TestDatabase) -> Vec<String> {
    let mut result = vec![];
    for (_, input_id) in db.map.0.iter() {
        let input = db.get(*input_id);
        result.push(input.value.clone())
    }
    result
}
