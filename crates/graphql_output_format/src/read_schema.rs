use std::{path::PathBuf, str::Utf8Error};

use common_lang_types::WithLocation;
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use isograph_lang_types::SchemaSource;
use pico::{Database, MemoRef, SourceId};
use pico_macros::memo;
use thiserror::Error;

#[memo]
pub fn parse_graphql_schema(
    db: &Database,
    schema_source_id: SourceId<SchemaSource>,
) -> Result<MemoRef<GraphQLTypeSystemDocument>, BatchCompileError> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_source_id);
    let schema = parse_schema(content, *text_source)
        .map(|document| db.intern(document))
        .map_err(|with_span| with_span.to_with_location(*text_source))?;
    Ok(schema)
}

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum BatchCompileError {
    #[error("Unable to load schema file at path {path:?}.\nReason: {message}")]
    UnableToLoadSchema { path: PathBuf, message: String },

    #[error("Attempted to load the graphql schema at the following path: {path:?}, but that is not a file.")]
    SchemaNotAFile { path: PathBuf },

    #[error("Unable to read the file at the following path: {path:?}.\nReason: {message}")]
    UnableToReadFile { path: PathBuf, message: String },

    #[error("Unable to create schema.\nReason: {0}")]
    UnableToCreateSchema(#[from] WithLocation<isograph_schema::CreateAdditionalFieldsError>),

    #[error("Error when processing an entrypoint declaration.\nReason: {0}")]
    ErrorWhenProcessingEntrypointDeclaration(
        #[from] WithLocation<isograph_schema::ValidateEntrypointDeclarationError>,
    ),

    #[error("Unable to strip prefix.\nReason: {0}")]
    UnableToStripPrefix(#[from] std::path::StripPrefixError),

    #[error("Unable to convert file {path:?} to utf8.\nDetailed reason: {reason}")]
    UnableToConvertToString { path: PathBuf, reason: Utf8Error },

    #[error("Unable to parse schema.\n\n{0}")]
    UnableToParseSchema(#[from] WithLocation<SchemaParseError>),
}

#[memo]
pub fn parse_schema_extensions_file(
    db: &Database,
    schema_extension_source_id: SourceId<SchemaSource>,
) -> Result<MemoRef<GraphQLTypeSystemExtensionDocument>, BatchCompileError> {
    let SchemaSource {
        content,
        text_source,
        ..
    } = db.get(schema_extension_source_id);
    let schema_extensions = parse_schema_extensions(content, *text_source)
        .map(|document| db.intern(document))
        .map_err(|with_span| with_span.to_with_location(*text_source))?;

    Ok(schema_extensions)
}
