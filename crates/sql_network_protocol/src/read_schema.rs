use common_lang_types::DiagnosticResult;
use isograph_schema::{CompilationProfile, IsographDatabase, SchemaSource};
use pico_macros::memo;
use sql_lang_types::SQLTypeSystemDocument;
use sql_schema_parser::parse_schema;

#[memo]
// TODO add error recovery and the non_fatal_diagnostics vec, i.e. do not return
// a Result
pub fn parse_sql_schema<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
) -> DiagnosticResult<SQLTypeSystemDocument> {
    // TODO use db.get_standard_sources()
    let SchemaSource { content, .. } = db.get_schema_source();

    parse_schema(content)
}
