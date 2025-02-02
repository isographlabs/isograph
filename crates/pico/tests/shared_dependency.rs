use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico_core::{database::Database, source::SourceId, storage::Storage};
use pico_macros::{memo, Db, Source};

mod calc;

/// Assert that we correctly compare epochs when 2 queries share the same dependency,
/// i.e. use the same memoized `parse_ast` function
#[test]
fn shared_dependency() {
    let mut state = TestDatabase::default();

    let input = state.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let result = evaluate(&state, input);
    assert_eq!(result, 6);
    let result_exp = evaluate_exp(&state, input, 2);
    assert_eq!(result_exp, 36);

    let input = state.set(Input {
        key: "input",
        value: "3 * 3".to_string(),
    });

    let result = evaluate(&state, input);
    assert_eq!(result, 9);
    let result_exp = evaluate_exp(&state, input, 2);
    assert_eq!(result_exp, 81);
}

#[derive(Debug, Default, Db)]
struct TestDatabase {
    pub storage: Storage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
fn parse_ast(db: &TestDatabase, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
fn evaluate(db: &TestDatabase, id: SourceId<Input>) -> i64 {
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}

#[memo]
fn evaluate_exp(db: &TestDatabase, id: SourceId<Input>, exp: u32) -> i64 {
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression)
        .expect("value must be evaluated")
        .pow(exp)
}
