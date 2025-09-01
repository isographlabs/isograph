use pico::{Database, SourceId, Storage};
use pico_macros::{Db, Source, memo};

#[derive(Db, Default)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

#[test]
#[should_panic]
fn removing_direct_param_causes_panic() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });
    db.remove(input_id);

    // Reading the input_id causes a panic
    first_letter(&db, input_id);
}

#[test]
#[should_panic]
fn removing_indirect_param_causes_panic() {
    let mut db = TestDatabase::default();

    let input_id = db.set(Input {
        key: "key",
        value: "asdf".to_string(),
    });

    capitalized_first_letter(&db, input_id);

    db.remove(input_id);

    // When we verify the result, we eventually try to validate
    // input_id, which is missing, so we panic.
    capitalized_first_letter(&db, input_id);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    let input = db.get(input_id);
    input.value.chars().next().unwrap()
}

#[memo]
fn capitalized_first_letter(db: &TestDatabase, input_id: SourceId<Input>) -> char {
    let first = first_letter(db, input_id);
    first.to_ascii_uppercase()
}
