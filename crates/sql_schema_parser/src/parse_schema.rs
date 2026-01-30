use common_lang_types::{Diagnostic, DiagnosticResult, Location};
use prelude::*;
use sql_lang_types::SQLTypeSystemDocument;
use sqlparser::{dialect::SQLiteDialect, parser::Parser};

pub fn parse_schema(source: &str) -> DiagnosticResult<SQLTypeSystemDocument> {
    // TODO: figure out what to do with dialects
    let dialect = SQLiteDialect {};
    // TODO: sqlparser does not provide location for `ParseError`. There is an
    // open PR that fixes it, so we can wait or reimplement parse function and use
    // self.peek_token() on error to extract location
    let ast = Parser::parse_sql(&dialect, source)
        .map_err(|e| Diagnostic::new(e.to_string(), Location::Generated.wrap_some()))?;

    SQLTypeSystemDocument(ast).wrap_ok()
}
