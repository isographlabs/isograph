use std::sync::atomic::{AtomicUsize, Ordering};

use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::storage::DefaultStorage;
use pico_core::{database::Database as Db, source::SourceId};
use pico_macros::{memo, Db, Source};

mod calc;

static COUNTER: AtomicUsize = AtomicUsize::new(0);

#[test]
fn returns_result() {
    let mut db = Database::default();
    let key = "test_expr";

    let mut input = Input {
        key,
        value: "2 + 2 * 2".to_string(),
    };
    let id = db.set(input);
    let mut result = evaluate_input(&mut db, id);
    assert_eq!(result, 6);

    input = Input {
        key,
        value: "3 + 3".to_string(),
    };
    let id = db.set(input);
    result = evaluate_input(&mut db, id);
    assert_eq!(result, 6);

    assert_eq!(COUNTER.load(Ordering::SeqCst), 4);
}

#[derive(Debug, Default, Db)]
struct Database {
    pub storage: DefaultStorage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
pub fn parse_ast(db: &mut Database, id: SourceId<Input>) -> Result<Program> {
    COUNTER.fetch_add(1, Ordering::SeqCst);
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
pub fn evaluate_input(db: &mut Database, id: SourceId<Input>) -> i64 {
    COUNTER.fetch_add(1, Ordering::SeqCst);
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}
