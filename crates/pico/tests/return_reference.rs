use calc::{ast::Program, error::Result, eval::eval, lexer::Lexer, parser::Parser};
use pico::storage::DefaultStorage;
use pico_core::{database::Database, source::SourceId};
use pico_macros::{memo, Db, Source};

mod calc;

#[test]
fn return_reference() {
    let mut state = TestDatabase::default();

    let input = state.set(Input {
        key: "input",
        value: "2 + 2 * 2".to_string(),
    });

    let result = evaluate_input(&mut state, input);
    assert_eq!(*result, 6);
}

#[derive(Debug, Default, Db)]
struct TestDatabase {
    pub storage: DefaultStorage<Self>,
}

#[derive(Debug, Clone, PartialEq, Eq, Source)]
struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
pub fn parse_ast(db: &mut TestDatabase, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo(reference)]
pub fn evaluate_input(db: &mut TestDatabase, id: SourceId<Input>) -> i64 {
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval(ast.expression).expect("value must be evaluated")
}
