use common_lang_types::RelativePathToSourceFile;
use graphql_schema_parser::{
    DefinitionError, ParsedSchema, TypeSystemDefinitionOrExtension, parse_schema,
};
use isograph_compiler::{
    ClientId, CompilationProfile, IsographDb, file_contents, schema_file_paths,
};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

/// The parse of one schema file as `client` reads it; `None` when the file has no text.
#[memo]
pub fn parsed_schema_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<ParsedSchema> {
    file_contents(db, client, path).as_deref().map(parse_schema)
}

/// One top-level entry of a schema file.
pub(crate) struct SchemaEntry<'db> {
    pub path: RelativePathToSourceFile,
    pub entry: &'db Result<WithSpan<TypeSystemDefinitionOrExtension>, WithSpan<DefinitionError>>,
}

/// Every parsed entry of every schema file `client` reads, with its file.
pub(crate) fn schema_entries<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
) -> impl Iterator<Item = SchemaEntry<'_>> {
    schema_file_paths(db, client).iter().flat_map(move |path| {
        parsed_schema_file(db, client, path.dereference())
            .iter()
            .flat_map(|parsed| parsed.definitions.iter())
            .map(move |entry| SchemaEntry {
                path: path.dereference(),
                entry,
            })
    })
}

/// An error the GraphQL protocol reports for a schema file.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum GraphQLSchemaFileError {
    #[error("{0}")]
    Parse(graphql_schema_parser::ParseError),
}

/// The errors of the schema file `path` as `client` reads it, file-absolute; `None` when the
/// file has no text.
#[memo]
pub fn graphql_schema_file_errors<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<GraphQLSchemaFileError>>> {
    let parsed = parsed_schema_file(db, client, path).as_ref()?;
    parsed
        .definitions
        .iter()
        .filter_map(|entry| entry.as_ref().err())
        .map(|failed| {
            GraphQLSchemaFileError::Parse(failed.item.error.item)
                .with_span(failed.item.error.location)
        })
        .collect::<Vec<_>>()
        .wrap_some()
}
