use std::path::PathBuf;

use common_lang_types::{with_span_to_with_location, Location, Span, TextSource, WithLocation};
use graphql_lang_parser::{parse_schema, SchemaParseError};
use intern::string_key::Intern;
use isograph_lang_parser::{parse_iso_fetch, parse_iso_literal, IsographLiteralParseError};
use isograph_schema::Schema;
use structopt::StructOpt;
use thiserror::Error;

use crate::{
    config::CompilerConfig,
    generate_artifacts::{generate_artifacts, GenerateArtifactsError},
    isograph_literals::{
        extract_iso_fetch_from_file_content, extract_iso_literal_from_file_content,
        read_files_in_folder, IsoFetchExtraction, IsoLiteralExtraction,
    },
    schema::read_schema_file,
};

/// Options if we're doing a batch compilation
#[derive(Debug, StructOpt)]
pub(crate) struct BatchCompileCliOptions {
    /// Compile using this config file. If not provided, searches for a config in
    /// package.json under the `isograph` key.
    #[structopt(long)]
    config: Option<PathBuf>,
}

pub(crate) fn handle_compile_command(opt: BatchCompileCliOptions) -> Result<(), BatchCompileError> {
    let config = CompilerConfig::create(opt.config);

    let content = read_schema_file(&config.schema)?;
    let schema_text_source = TextSource {
        path: config.schema.to_str().unwrap().intern().into(),
        span: None,
    };
    let type_system_document = parse_schema(&content, schema_text_source)
        .map_err(|with_span| with_span_to_with_location(with_span, schema_text_source))?;
    let mut schema = Schema::new();

    schema.process_type_system_document(type_system_document)?;

    let canonicalized_root_path = {
        let current_dir = std::env::current_dir().expect("current_dir should exist");
        let joined = current_dir.join(&config.project_root);
        joined
            .canonicalize()
            .map_err(|message| BatchCompileError::UnableToLoadSchema {
                path: joined.clone(),
                message,
            })?
    };

    // TODO return an iterator
    let project_files = read_files_in_folder(&canonicalized_root_path)?;
    for (file_path, file_content) in project_files {
        // TODO don't intern unless there's a match
        let interned_file_path = file_path.to_string_lossy().into_owned().intern().into();

        let file_name = canonicalized_root_path
            .join(file_path)
            .to_str()
            .expect("file_path should be a valid string")
            .intern()
            .into();

        for iso_literal_extraction in extract_iso_literal_from_file_content(&file_content) {
            let IsoLiteralExtraction {
                iso_literal_text,
                iso_literal_start_index,
                has_associated_js_function,
            } = iso_literal_extraction;

            let text_source = TextSource {
                path: file_name,
                span: Some(Span::new(
                    iso_literal_start_index as u32,
                    (iso_literal_start_index + iso_literal_text.len()) as u32,
                )),
            };

            if !has_associated_js_function {
                return Err(BatchCompileError::UnableToParseIsographLiteral {
                    message: WithLocation::new(
                        IsographLiteralParseError::ExpectedAssociatedJsFunction,
                        Location::new(text_source, Span::todo_generated()),
                    ),
                });
            }

            let resolver_declaration =
                parse_iso_literal(&iso_literal_text, interned_file_path, text_source)?;
            schema.process_resolver_declaration(resolver_declaration, text_source)?;
        }

        for iso_fetch_extaction in extract_iso_fetch_from_file_content(&file_content) {
            let IsoFetchExtraction {
                iso_fetch_text,
                iso_fetch_start_index,
            } = iso_fetch_extaction;
            let text_source = TextSource {
                path: file_name,
                span: Some(Span::new(
                    iso_fetch_start_index as u32,
                    (iso_fetch_start_index + iso_fetch_text.len()) as u32,
                )),
            };

            let fetch_declaration = parse_iso_fetch(iso_fetch_text, text_source)?;
            schema
                .fetchable_resolvers
                .push((text_source, fetch_declaration));
        }
    }

    let validated_schema = Schema::validate_and_construct(schema)?;

    generate_artifacts(
        &validated_schema,
        &config.project_root,
        &config.artifact_directory,
    )?;

    Ok(())
}

#[derive(Error, Debug)]
pub(crate) enum BatchCompileError {
    #[error("Unable to load schema file at path {path:?}.\nReason: {message}")]
    UnableToLoadSchema {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Attempted to load the graphql schema at the following path: {path:?}, but that is not a file.")]
    SchemaNotAFile { path: PathBuf },

    #[error("The project root at the following path: \"{path:?}\", is not a directory.")]
    ProjectRootNotADirectory { path: PathBuf },

    #[error("Unable to read the file at the following path: {path:?}.\nReason: {message}")]
    UnableToReadFile {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Unable to traverse directory.\nReason: {message}")]
    UnableToTraverseDirectory { message: std::io::Error },

    #[error("Unable to convert schema to string.\nReason: {message}")]
    UnableToConvertToString { message: std::str::Utf8Error },

    #[error("Unable to parse schema.\n\n{message}")]
    UnableToParseSchema {
        message: WithLocation<SchemaParseError>,
    },

    #[error("Unable to parse isograph literal.\n\n{message}")]
    UnableToParseIsographLiteral {
        message: WithLocation<IsographLiteralParseError>,
    },

    #[error("Unable to create schema.\nReason: {message}")]
    UnableToCreateSchema {
        message: WithLocation<isograph_schema::ProcessTypeDefinitionError>,
    },

    #[error("Error when processing a resolver declaration.\nReason: {message}")]
    ErrorWhenProcessingResolverDeclaration {
        message: WithLocation<isograph_schema::ProcessResolverDeclarationError>,
    },

    #[error("Error when processing a resolver fetch declaration.\nReason: {message}")]
    ErrorWhenProcessingResolverFetchDeclaration {
        message: WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>,
    },

    #[error("Unable to strip prefix.\nReason: {message}")]
    UnableToStripPrefix {
        message: std::path::StripPrefixError,
    },

    #[error("Error when validating schema, resolvers and fetch declarations.\nReason: {message}")]
    UnableToValidateSchema {
        message: WithLocation<isograph_schema::ValidateSchemaError>,
    },

    #[error("Unable to print.\nReason: {message}")]
    UnableToPrint { message: GenerateArtifactsError },
}

impl From<WithLocation<SchemaParseError>> for BatchCompileError {
    fn from(value: WithLocation<SchemaParseError>) -> Self {
        BatchCompileError::UnableToParseSchema { message: value }
    }
}

impl From<WithLocation<IsographLiteralParseError>> for BatchCompileError {
    fn from(value: WithLocation<IsographLiteralParseError>) -> Self {
        BatchCompileError::UnableToParseIsographLiteral { message: value }
    }
}

impl From<WithLocation<isograph_schema::ProcessTypeDefinitionError>> for BatchCompileError {
    fn from(value: WithLocation<isograph_schema::ProcessTypeDefinitionError>) -> Self {
        BatchCompileError::UnableToCreateSchema { message: value }
    }
}

impl From<WithLocation<isograph_schema::ProcessResolverDeclarationError>> for BatchCompileError {
    fn from(value: WithLocation<isograph_schema::ProcessResolverDeclarationError>) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverDeclaration { message: value }
    }
}

impl From<WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>>
    for BatchCompileError
{
    fn from(value: WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverFetchDeclaration { message: value }
    }
}

impl From<std::path::StripPrefixError> for BatchCompileError {
    fn from(value: std::path::StripPrefixError) -> Self {
        BatchCompileError::UnableToStripPrefix { message: value }
    }
}

impl From<WithLocation<isograph_schema::ValidateSchemaError>> for BatchCompileError {
    fn from(value: WithLocation<isograph_schema::ValidateSchemaError>) -> Self {
        BatchCompileError::UnableToValidateSchema { message: value }
    }
}

impl From<GenerateArtifactsError> for BatchCompileError {
    fn from(value: GenerateArtifactsError) -> Self {
        BatchCompileError::UnableToPrint { message: value }
    }
}
