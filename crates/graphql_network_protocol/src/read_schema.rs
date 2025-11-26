use std::collections::BTreeMap;

use common_lang_types::{DiagnosticResult, RelativePathToSourceFile};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions};
use isograph_schema::{IsographDatabase, NetworkProtocol, SchemaSource};
use pico::{MemoRef, SourceId};
use pico_macros::memo;
use prelude::Postfix;

#[expect(clippy::type_complexity)]
#[memo]
pub fn parse_graphql_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticResult<(
    MemoRef<GraphQLTypeSystemDocument>,
    BTreeMap<RelativePathToSourceFile, MemoRef<GraphQLTypeSystemExtensionDocument>>,
)> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get_schema_source();

    let schema = parse_schema(content, *text_source)?;

    let mut schema_extensions = BTreeMap::new();
    for (relative_path, schema_extension_source_id) in db
        .get_standard_sources()
        .tracked()
        .schema_extension_sources
        .iter()
    {
        let extensions_document = parse_schema_extensions_file(db, *schema_extension_source_id)
            .to_owned()
            .note_todo("Do not clone. Use a MemoRef.")?;
        schema_extensions.insert(*relative_path, extensions_document);
    }

    (db.intern(schema), schema_extensions).wrap_ok()
}

#[memo]
pub fn parse_schema_extensions_file<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema_extension_source_id: SourceId<SchemaSource>,
) -> DiagnosticResult<MemoRef<GraphQLTypeSystemExtensionDocument>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_extension_source_id);
    let schema_extensions = parse_schema_extensions(content, *text_source)?;

    db.intern(schema_extensions).wrap_ok()
}
