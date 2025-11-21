use std::collections::BTreeMap;

use common_lang_types::{
    EmbeddedLocation, Location, RelativePathToSourceFile, Span, TextSource, WithEmbeddedLocation,
    WithLocation,
};
use graphql_lang_types::{
    GraphQLObjectTypeDefinition, GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument,
    GraphQLTypeSystemExtensionDocument,
};
use graphql_schema_parser::{SchemaParseError, parse_schema, parse_schema_extensions};
use intern::string_key::Intern;
use isograph_schema::{IsographDatabase, NetworkProtocol, SchemaSource};
use pico::{MemoRef, SourceId};
use pico_macros::memo;
use tracing::info;

#[expect(clippy::type_complexity)]
#[memo]
pub fn parse_graphql_schema<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<(MemoRef<GraphQLTypeSystemDocument>,), WithLocation<SchemaParseError>> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get_schema_source();

    // parse schema is not a memoized function...
    // but it does depend on the content (which changes)
    // let schema = parse_schema(content, *text_source)
    //     .map_err(|with_span| with_span.to_with_location(*text_source))?;
    //
    // we simulate that by looking for the string FOOBARBAZ, and keying off of that
    let schema = if content.contains("FOOBARBAZ") {
        info!("parsing graphql schema (contains FOOBARBAZ)");
        GraphQLTypeSystemDocument(vec![WithLocation::new(
            GraphQLTypeSystemDefinition::ObjectTypeDefinition(GraphQLObjectTypeDefinition {
                description: None,
                name: WithEmbeddedLocation::new(
                    "foo".intern().into(),
                    EmbeddedLocation {
                        text_source: TextSource {
                            current_working_directory: "foo".intern().into(),
                            relative_path_to_source_file: "foo".intern().into(),
                            span: None,
                        },
                        span: Span::new(0, 20),
                    },
                ),
                interfaces: vec![],
                directives: vec![],
                fields: vec![],
            }),
            Location::generated(),
        )])
    } else {
        info!("parsing graphql schema (does not contain FOOBARBAZ)");
        GraphQLTypeSystemDocument(vec![])
    };

    // *returning the interned schema is required*
    Ok((db.intern(schema),))
}

#[memo]
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
