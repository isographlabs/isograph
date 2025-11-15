use std::collections::BTreeMap;

use common_lang_types::{RelativePathToSourceFile, WithLocation};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{SchemaParseError, parse_schema, parse_schema_extensions};
use isograph_schema::{IsographDatabase, NetworkProtocol, SchemaSource};
use pico::{MemoRef, SourceId};
use pico_macros::legacy_memo;

#[expect(clippy::type_complexity)]
#[legacy_memo]
pub fn parse_graphql_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<
    (
        MemoRef<GraphQLTypeSystemDocument>,
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
            parse_schema_extensions_file(db, *schema_extension_source_id).to_owned()?;
        schema_extensions.insert(*relative_path, extensions_document);
    }

    Ok((db.intern(schema), schema_extensions))
}

#[legacy_memo]
pub fn parse_schema_extensions_file<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema_extension_source_id: SourceId<SchemaSource>,
) -> Result<MemoRef<GraphQLTypeSystemExtensionDocument>, WithLocation<SchemaParseError>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_extension_source_id);
    let schema_extensions = parse_schema_extensions(content, *text_source)
        .map_err(|with_span| with_span.to_with_location(*text_source))?;

    Ok(db.intern(schema_extensions))
}
