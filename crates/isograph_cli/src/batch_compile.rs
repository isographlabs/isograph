use std::path::PathBuf;

use common_lang_types::{
    Location, ResolverDefinitionPath, SourceFileName, Span, TextSource, WithLocation, WithSpan,
};
use graphql_lang_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use intern::string_key::Intern;
use isograph_lang_parser::{parse_iso_fetch, parse_iso_literal, IsographLiteralParseError};
use isograph_lang_types::{ResolverDeclaration, ResolverFetch};
use isograph_schema::{
    ProcessGraphQLDocumentOutcome, ProcessResolverDeclarationError, Schema, UnvalidatedSchema,
};
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
        path: config
            .schema
            .to_str()
            .expect("Expected schema to be valid string")
            .intern()
            .into(),
        span: None,
    };
    let type_system_document = parse_schema(&content, schema_text_source)
        .map_err(|with_span| with_span.to_with_location(schema_text_source))?;

    let type_extension_document = config
        .schema_extensions
        .iter()
        .map(|schema_extension_path| {
            let extension_text_source = TextSource {
                path: schema_extension_path
                    .to_str()
                    .expect("Expected schema extension to be valid string")
                    .intern()
                    .into(),
                span: None,
            };
            let extension_content = read_schema_file(schema_extension_path)?;
            let extension = parse_schema_extensions(&extension_content, extension_text_source)
                .map_err(|with_span| with_span.to_with_location(extension_text_source))?;
            Ok(extension)
        })
        .collect::<Result<Vec<_>, BatchCompileError>>()?;

    let mut schema = Schema::new();

    let mut process_graphql_outcome =
        schema.process_graphql_type_system_document(type_system_document)?;

    for extension_document in type_extension_document {
        let ProcessGraphQLDocumentOutcome { mutation_id } =
            schema.process_graphql_type_extension_document(extension_document)?;

        match (mutation_id, process_graphql_outcome.mutation_id) {
            (None, _) => {}
            (Some(mutation_id), None) => process_graphql_outcome.mutation_id = Some(mutation_id),
            (Some(_), Some(_)) => return Err(BatchCompileError::MutationObjectDefinedTwice),
        }
    }

    // TODO the ordering should be:
    // - process schema
    // - validate
    // - add mutation fields
    // - process parsed literals
    // - validate resolvers
    if let Some(mutation_id) = process_graphql_outcome.mutation_id {
        schema.create_magic_mutation_fields(mutation_id)?;
    }

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

    let (parsed_literals, parsed_fetches) =
        extract_iso_literals(project_files, canonicalized_root_path)
            .map_err(BatchCompileError::from)?;

    process_parsed_literals_and_fetches(&mut schema, parsed_literals, parsed_fetches)?;

    let validated_schema = Schema::validate_and_construct(schema)?;

    generate_artifacts(
        &validated_schema,
        &config.project_root,
        &config.artifact_directory,
    )?;

    Ok(())
}

fn process_parsed_literals_and_fetches(
    schema: &mut UnvalidatedSchema,
    literals: Vec<(WithSpan<ResolverDeclaration>, TextSource)>,
    fetches: Vec<(WithSpan<ResolverFetch>, TextSource)>,
) -> Result<(), Vec<WithLocation<ProcessResolverDeclarationError>>> {
    let mut errors = vec![];
    for (resolver_declaration, text_source) in literals {
        if let Err(e) = schema.process_resolver_declaration(resolver_declaration, text_source) {
            errors.push(e);
        }
    }
    for (resolver_fetch, text_source) in fetches {
        schema
            .fetchable_resolvers
            .push((text_source, resolver_fetch))
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn extract_iso_literals(
    project_files: Vec<(PathBuf, String)>,
    canonicalized_root_path: PathBuf,
) -> Result<
    (
        Vec<(WithSpan<ResolverDeclaration>, TextSource)>,
        Vec<(WithSpan<ResolverFetch>, TextSource)>,
    ),
    Vec<WithLocation<IsographLiteralParseError>>,
> {
    let mut isograph_literal_parse_errors = vec![];
    let mut resolver_declarations_and_text_sources = vec![];
    let mut resolver_fetch_and_text_sources = vec![];

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
            match process_iso_literal_extraction(
                iso_literal_extraction,
                file_name,
                interned_file_path,
            ) {
                Ok((resolver_declaration, text_source)) => {
                    resolver_declarations_and_text_sources.push((resolver_declaration, text_source))
                }
                Err(e) => isograph_literal_parse_errors.push(e),
            }
        }

        for iso_fetch_extaction in extract_iso_fetch_from_file_content(&file_content) {
            match process_iso_fetch_extraction(iso_fetch_extaction, file_name) {
                Ok((fetch_declaration, text_source)) => {
                    resolver_fetch_and_text_sources.push((fetch_declaration, text_source))
                }
                Err(e) => isograph_literal_parse_errors.push(e),
            }
        }
    }

    if isograph_literal_parse_errors.is_empty() {
        Ok((
            resolver_declarations_and_text_sources,
            resolver_fetch_and_text_sources,
        ))
    } else {
        Err(isograph_literal_parse_errors)
    }
}

fn process_iso_fetch_extraction(
    iso_fetch_extaction: IsoFetchExtraction<'_>,
    file_name: SourceFileName,
) -> Result<(WithSpan<ResolverFetch>, TextSource), WithLocation<IsographLiteralParseError>> {
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
    Ok((fetch_declaration, text_source))
}

fn process_iso_literal_extraction(
    iso_literal_extraction: IsoLiteralExtraction<'_>,
    file_name: SourceFileName,
    interned_file_path: ResolverDefinitionPath,
) -> Result<(WithSpan<ResolverDeclaration>, TextSource), WithLocation<IsographLiteralParseError>> {
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
        return Err(WithLocation::new(
            IsographLiteralParseError::ExpectedAssociatedJsFunction,
            Location::new(text_source, Span::todo_generated()),
        ));
    }

    let resolver_declaration =
        parse_iso_literal(&iso_literal_text, interned_file_path, text_source)?;

    Ok((resolver_declaration, text_source))
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

    #[error(
        "{}{}",
        if messages.len() == 1 { "Unable to parse Isograph literal:" } else { "Unable to parse Isograph literals:" },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    UnableToParseIsographLiterals {
        messages: Vec<WithLocation<IsographLiteralParseError>>,
    },

    #[error("Unable to create schema.\nReason: {message}")]
    UnableToCreateSchema {
        message: WithLocation<isograph_schema::ProcessTypeDefinitionError>,
    },

    #[error(
        "{}{}",
        if messages.len() == 1 {
            "Error when processing a resolver declaration:"
        } else {
            "Errors when processing resolver declarations:"
        },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    ErrorWhenProcessingResolverDeclaration {
        messages: Vec<WithLocation<isograph_schema::ProcessResolverDeclarationError>>,
    },

    #[error("Error when processing a resolver fetch declaration.\nReason: {message}")]
    ErrorWhenProcessingResolverFetchDeclaration {
        message: WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>,
    },

    #[error("Unable to strip prefix.\nReason: {message}")]
    UnableToStripPrefix {
        message: std::path::StripPrefixError,
    },

    #[error(
        "{} when validating schema, resolvers and fetch declarations.\n{}",
        if messages.len() == 1 { "Error" } else { "Errors" },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    UnableToValidateSchema {
        messages: Vec<WithLocation<isograph_schema::ValidateSchemaError>>,
    },

    #[error("Unable to print.\nReason: {message}")]
    UnableToPrint { message: GenerateArtifactsError },

    #[error("Mutation object defined twice")]
    MutationObjectDefinedTwice,
}

impl From<WithLocation<SchemaParseError>> for BatchCompileError {
    fn from(message: WithLocation<SchemaParseError>) -> Self {
        BatchCompileError::UnableToParseSchema { message }
    }
}

impl From<Vec<WithLocation<IsographLiteralParseError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<IsographLiteralParseError>>) -> Self {
        BatchCompileError::UnableToParseIsographLiterals { messages }
    }
}

impl From<WithLocation<isograph_schema::ProcessTypeDefinitionError>> for BatchCompileError {
    fn from(message: WithLocation<isograph_schema::ProcessTypeDefinitionError>) -> Self {
        BatchCompileError::UnableToCreateSchema { message }
    }
}

impl From<Vec<WithLocation<isograph_schema::ProcessResolverDeclarationError>>>
    for BatchCompileError
{
    fn from(messages: Vec<WithLocation<isograph_schema::ProcessResolverDeclarationError>>) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverDeclaration { messages }
    }
}

impl From<WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>>
    for BatchCompileError
{
    fn from(message: WithLocation<isograph_schema::ValidateResolverFetchDeclarationError>) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverFetchDeclaration { message }
    }
}

impl From<std::path::StripPrefixError> for BatchCompileError {
    fn from(message: std::path::StripPrefixError) -> Self {
        BatchCompileError::UnableToStripPrefix { message }
    }
}

impl From<Vec<WithLocation<isograph_schema::ValidateSchemaError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<isograph_schema::ValidateSchemaError>>) -> Self {
        BatchCompileError::UnableToValidateSchema { messages }
    }
}

impl From<GenerateArtifactsError> for BatchCompileError {
    fn from(message: GenerateArtifactsError) -> Self {
        BatchCompileError::UnableToPrint { message }
    }
}
