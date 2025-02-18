use std::{path::PathBuf, str::Utf8Error};

use common_lang_types::{RelativePathToSourceFile, TextSource, WithLocation};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use isograph_config::{AbsolutePathAndRelativePath, CompilerConfig};
use thiserror::Error;

pub fn read_and_parse_graphql_schema(
    config: &CompilerConfig,
) -> Result<GraphQLTypeSystemDocument, BatchCompileError> {
    let content = read_schema_file(&config.schema.absolute_path)?;
    let schema_text_source = TextSource {
        relative_path_to_source_file: config.schema.relative_path,
        span: None,
        current_working_directory: config.current_working_directory,
    };
    let schema = parse_schema(&content, schema_text_source)
        .map_err(|with_span| with_span.to_with_location(schema_text_source))?;
    Ok(schema)
}

/// Read schema file
fn read_schema_file(path: &PathBuf) -> Result<String, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(path);
    let canonicalized_existing_path =
        joined
            .canonicalize()
            .map_err(|e| BatchCompileError::UnableToLoadSchema {
                path: joined,
                message: e.to_string(),
            })?;

    if !canonicalized_existing_path.is_file() {
        return Err(BatchCompileError::SchemaNotAFile {
            path: canonicalized_existing_path,
        });
    }

    let contents = std::fs::read(canonicalized_existing_path.clone()).map_err(|e| {
        BatchCompileError::UnableToReadFile {
            path: canonicalized_existing_path.clone(),
            message: e.to_string(),
        }
    })?;

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| BatchCompileError::UnableToConvertToString {
            path: canonicalized_existing_path.clone(),
            reason: e,
        })?
        .to_owned();

    Ok(contents)
}

#[derive(Error, Debug)]
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

pub fn read_and_parse_schema_extensions(
    schema_extension_path: &AbsolutePathAndRelativePath,
    config: &CompilerConfig,
) -> Result<(RelativePathToSourceFile, GraphQLTypeSystemExtensionDocument), BatchCompileError> {
    let extension_content = read_schema_file(&schema_extension_path.absolute_path)?;
    let extension_text_source = TextSource {
        relative_path_to_source_file: schema_extension_path.relative_path,
        span: None,
        current_working_directory: config.current_working_directory,
    };

    let schema_extensions = parse_schema_extensions(&extension_content, extension_text_source)
        .map_err(|with_span| with_span.to_with_location(extension_text_source))?;

    Ok((schema_extension_path.relative_path, schema_extensions))
}
