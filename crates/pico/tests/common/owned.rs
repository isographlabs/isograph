use crate::calc::{error::Result, eval, lexer::Lexer, parser::Parser};

use crate::calc::ast::Program;
use pico::database::Database;
use pico_macros::{memo, source};

#[source]
pub struct Input {
    pub value: String,
}

#[memo]
pub fn parse_ast(db: &mut Database, key: &'static str) -> Result<Program> {
    let source_text = Input::get(db, key);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo]
pub fn evaluate_input(db: &mut Database, key: &'static str) -> i64 {
    let ast = parse_ast(db, key).expect("ast must be correct");
    eval::eval(ast.expression).expect("value must be evaluated")
}
