use std::collections::BTreeMap;

use common_lang_types::{RelativePathToSourceFile, WithLocation};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{SchemaParseError, parse_schema, parse_schema_extensions};
use isograph_schema::{IsographDatabase, NetworkProtocol, SchemaSource};
use pico::{MemoRef, SourceId};
use pico_macros::memo;

#[allow(clippy::type_complexity)]
#[memo]
pub fn parse_graphql_schema<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        GraphQLTypeSystemDocument,
        BTreeMap<RelativePathToSourceFile, MemoRef<GraphQLTypeSystemExtensionDocument>>,
    ),
    WithLocation<SchemaParseError>,
> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get_schema_source();

    let schema = parse_schema(content, *text_source)
        .map_err(|with_span| with_span.to_with_location(*text_source))?;

    let mut schema_extensions = BTreeMap::new();
    for (relative_path, schema_extension_source_id) in db
        .get_standard_sources()
        .tracked()
        .schema_extension_sources
        .iter()
    {
        let extensions_document =
            parse_schema_extensions_file(db, *schema_extension_source_id).try_ok()?;
        schema_extensions.insert(*relative_path, extensions_document);
    }

    Ok((schema, schema_extensions))
}

#[memo]
pub fn parse_schema_extensions_file<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema_extension_source_id: SourceId<SchemaSource>,
) -> Result<GraphQLTypeSystemExtensionDocument, WithLocation<SchemaParseError>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_extension_source_id);
    let schema_extensions = parse_schema_extensions(content, *text_source)
        .map_err(|with_span| with_span.to_with_location(*text_source))?;

    Ok(schema_extensions)
}
