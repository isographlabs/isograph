use crate::calc::{error::Result, eval, lexer::Lexer, parser::Parser};

use crate::calc::ast::Program;
use pico_core::database::Database;
use pico_core::source::SourceId;
use pico_macros::{memo, Source};

#[derive(Debug, Clone, PartialEq, Eq, Source)]
pub struct Input {
    #[key]
    pub key: &'static str,
    pub value: String,
}

#[memo]
pub fn parse_ast(db: &Database, id: SourceId<Input>) -> Result<Program> {
    let source_text = db.get(id);
    let mut lexer = Lexer::new(source_text.value);
    let mut parser = Parser::new(&mut lexer)?;
    parser.parse_program()
}

#[memo(reference)]
pub fn evaluate_input(db: &Database, id: SourceId<Input>) -> i64 {
    let ast = parse_ast(db, id).expect("ast must be correct");
    eval::eval(ast.expression).expect("value must be evaluated")
}
