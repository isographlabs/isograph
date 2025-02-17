use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::{Database, MemoRef, SourceId};
use pico_macros::{memo, Source};

mod calc;

static VALUE: AtomicUsize = AtomicUsize::new(0);
static EXPECTED: AtomicUsize = AtomicUsize::new(0);

#[test]
fn arg_reference() {
    let mut db = Database::default();

    let input = db.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let value = evaluate_input(&db, input);
    // assert that the value was not cloned
    assert_eq!(VALUE.load(Ordering::SeqCst), 0);

    let expected = Expected(6);
    assert_result(&db, value, &expected);
    // assert that the argument of type `MemoRef<Value>` was not clone
    assert_eq!(VALUE.load(Ordering::SeqCst), 0);
    // compare with the argument of type `Expected` which is cloned once
    assert_eq!(EXPECTED.load(Ordering::SeqCst), 1);
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Value(pub i64);

impl Clone for Value {
    fn clone(&self) -> Self {
        VALUE.fetch_add(1, Ordering::SeqCst);
        Self(self.0)
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
struct Expected(pub i64);

impl Clone for Expected {
    fn clone(&self) -> Self {
        EXPECTED.fetch_add(1, Ordering::SeqCst);
        Self(self.0)
    }
}

#[memo]
fn parse_ast(db: &Database, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
fn evaluate_input(db: &Database, id: SourceId<Input>) -> Value {
    let ast = parse_ast(db, id).to_owned().expect("ast must be correct");
    let result = eval(ast.expression).expect("value must be evaluated");
    Value(result)
}

#[memo]
fn assert_result(_db: &Database, result: MemoRef<Value>, expected: &Expected) -> bool {
    result.0 == expected.0
}
