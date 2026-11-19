use common_lang_types::EntityName;
use common_lang_types::RelativePathToSourceFile;
use graphql_schema_parser::{
    DefinitionError, NullableTypeAnnotation, ParsedSchema, TypeAnnotation, TypeDefinition,
    TypeExtension, TypeSystemDefinition, TypeSystemDefinitionOrExtension, TypeSystemExtension,
    parse_schema,
};
use intern::Lookup;
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
    /// Upstream's `on_invalid_id_type` check at its default, `error`.
    #[error("The `id` field on `{0}` must have type `ID!`.")]
    InvalidIdField(EntityName),
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
        .flat_map(|entry| match entry {
            Err(failed) => GraphQLSchemaFileError::Parse(failed.item.error.item)
                .with_span(failed.item.error.location)
                .wrap_vec(),
            Ok(entry) => invalid_id_fields(entry.item.reference()),
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// An `InvalidIdField` at the type of each field named `id` of an object or interface
/// definition or extension whose type is not `ID!`.
fn invalid_id_fields(
    entry: &TypeSystemDefinitionOrExtension,
) -> Vec<WithSpan<GraphQLSchemaFileError>> {
    let (name, fields) = match entry {
        TypeSystemDefinitionOrExtension::Definition(TypeSystemDefinition::Type(
            TypeDefinition::Object(object),
        )) => (object.name, object.fields.as_slice()),
        TypeSystemDefinitionOrExtension::Definition(TypeSystemDefinition::Type(
            TypeDefinition::Interface(interface),
        )) => (interface.name, interface.fields.as_slice()),
        TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Type(
            TypeExtension::Object(object),
        )) => (object.name, object.fields.as_slice()),
        TypeSystemDefinitionOrExtension::Extension(TypeSystemExtension::Type(
            TypeExtension::Interface(interface),
        )) => (interface.name, interface.fields.as_slice()),
        TypeSystemDefinitionOrExtension::Definition(_)
        | TypeSystemDefinitionOrExtension::Extension(_) => return Vec::new(),
    };
    fields
        .iter()
        .filter(|field| field.item.name.item.0.lookup() == "id")
        .filter(|field| match field.item.type_.item.reference() {
            TypeAnnotation::NonNull(non_null) => match non_null.0.item.reference() {
                NullableTypeAnnotation::Named(named) => named.0.item.0.lookup() != "ID",
                NullableTypeAnnotation::List(_) => true,
            },
            TypeAnnotation::Named(_) | TypeAnnotation::List(_) => true,
        })
        .map(|field| {
            GraphQLSchemaFileError::InvalidIdField(name.item.0).with_span(field.item.type_.location)
        })
        .collect()
}
