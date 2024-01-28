use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use colored::Colorize;
use common_lang_types::{
    Location, ResolverDefinitionPath, ScalarFieldName, SourceFileName, Span, TextSource,
    WithLocation, WithSpan,
};
use graphql_schema_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use intern::{string_key::Intern, Lookup};
use isograph_lang_parser::{
    parse_iso_literal, IsoLiteralExtractionResult, IsographLiteralParseError,
};
use isograph_lang_types::{EntrypointTypeAndField, ResolverDeclaration};
use isograph_schema::{
    CompilerConfig, ProcessGraphQLDocumentOutcome, ProcessResolverDeclarationError, Schema,
    UnvalidatedSchema,
};
use pretty_duration::pretty_duration;
use thiserror::Error;

use crate::{
    generate_artifacts::{generate_and_write_artifacts, GenerateArtifactsError},
    isograph_literals::{
        extract_iso_literal_from_file_content, read_files_in_folder, IsoLiteralExtraction,
    },
    schema::read_schema_file,
};

pub(crate) struct CompilationStats {
    pub iso_resolver_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}
pub(crate) struct WithDuration<T> {
    pub elapsed_time: Duration,
    pub item: T,
}

impl<T> WithDuration<T> {
    pub fn new(calculate: impl FnOnce() -> T) -> WithDuration<T> {
        let start = Instant::now();
        let item = calculate();
        WithDuration {
            elapsed_time: start.elapsed(),
            item,
        }
    }
}

pub(crate) fn compile_and_print(
    config: &CompilerConfig,
) -> Result<CompilationStats, BatchCompileError> {
    eprintln!("{}", "Starting to compile.".cyan());

    let result = handle_compile_command(config);
    let elapsed_time = result.elapsed_time;

    match result.item {
        Ok(stats) => {
            eprintln!(
                    "{}",
                    format!(
                        "Successfully compiled {} resolvers and {} entrypoints, and wrote {} artifacts, in {}.\n",
                        stats.iso_resolver_count,
                        stats.entrypoint_count,
                        stats.total_artifacts_written,
                        pretty_duration(&elapsed_time, None)
                    )
                    .bright_green()
                );
            Ok(stats)
        }
        Err(err) => {
            eprintln!(
                "{}\n{}\n{}",
                "Error when compiling.\n".bright_red(),
                err,
                format!("Compilation took {}.", pretty_duration(&elapsed_time, None)).bright_red()
            );
            Err(err)
        }
    }
}

pub(crate) fn handle_compile_command(
    config: &CompilerConfig,
) -> WithDuration<Result<CompilationStats, BatchCompileError>> {
    WithDuration::new(|| {
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

        let mut schema = UnvalidatedSchema::new();

        let mut process_graphql_outcome =
            schema.process_graphql_type_system_document(type_system_document, config.options)?;

        // TODO validate here! We should not allow a situation in which a base schema is invalid,
        // but is made valid by the presence of schema extensions.

        for extension_document in type_extension_document {
            let ProcessGraphQLDocumentOutcome {
                mutation_id,
                type_refinement_maps: _,
            } = schema
                .process_graphql_type_extension_document(extension_document, config.options)?;
            // TODO extend the process_graphql_outcome.type_refinement_map and the one from the extensions?

            match (mutation_id, process_graphql_outcome.mutation_id) {
                (None, _) => {}
                (Some(mutation_id), None) => {
                    process_graphql_outcome.mutation_id = Some(mutation_id)
                }
                (Some(_), Some(_)) => return Err(BatchCompileError::MutationObjectDefinedTwice),
            }
        }

        // TODO the ordering should be:
        // - process schema
        // - validate
        // - process schema extension
        // - validate
        // - add mutation fields
        // - process parsed literals
        // - validate resolvers
        if let Some(mutation_id) = process_graphql_outcome.mutation_id {
            schema.create_magic_mutation_fields(
                mutation_id,
                config.options,
                &process_graphql_outcome
                    .type_refinement_maps
                    .supertype_to_subtype_map,
            )?;
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

        let (parsed_resolvers, parsed_entrypoints) =
            extract_iso_literals(project_files, canonicalized_root_path)
                .map_err(BatchCompileError::from)?;
        let resolver_count = parsed_resolvers.len();
        let entrypoint_count = parsed_entrypoints.len();

        process_parsed_resolvers_and_entrypoints(
            &mut schema,
            parsed_resolvers,
            parsed_entrypoints,
        )?;

        let validated_schema = Schema::validate_and_construct(schema)?;

        let total_artifacts_written = generate_and_write_artifacts(
            &validated_schema,
            &config.project_root,
            &config.artifact_directory,
        )?;

        Ok(CompilationStats {
            iso_resolver_count: resolver_count,
            entrypoint_count,
            total_artifacts_written,
        })
    })
}

fn process_parsed_resolvers_and_entrypoints(
    schema: &mut UnvalidatedSchema,
    resolvers: Vec<(WithSpan<ResolverDeclaration>, TextSource)>,
    entrypoints: Vec<(WithSpan<EntrypointTypeAndField>, TextSource)>,
) -> Result<(), Vec<WithLocation<ProcessResolverDeclarationError>>> {
    let mut errors = vec![];
    for (resolver_declaration, text_source) in resolvers {
        if let Err(e) = schema.process_resolver_declaration(resolver_declaration, text_source) {
            errors.push(e);
        }
    }
    for (resolver_fetch, text_source) in entrypoints {
        schema.entrypoints.push((text_source, resolver_fetch))
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
        Vec<(WithSpan<EntrypointTypeAndField>, TextSource)>,
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
                Ok((extraction_result, text_source)) => match extraction_result {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(decl) => {
                        resolver_declarations_and_text_sources.push((decl, text_source))
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(decl) => {
                        resolver_fetch_and_text_sources.push((decl, text_source))
                    }
                },
                Err(e) => isograph_literal_parse_errors.extend(e.into_iter()),
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

fn process_iso_literal_extraction(
    iso_literal_extraction: IsoLiteralExtraction<'_>,
    file_name: SourceFileName,
    interned_file_path: ResolverDefinitionPath,
) -> Result<(IsoLiteralExtractionResult, TextSource), Vec<WithLocation<IsographLiteralParseError>>>
{
    let IsoLiteralExtraction {
        iso_literal_text,
        iso_literal_start_index,
        has_associated_js_function,
        const_export_name,
    } = iso_literal_extraction;
    let text_source = TextSource {
        path: file_name,
        span: Some(Span::new(
            iso_literal_start_index as u32,
            (iso_literal_start_index + iso_literal_text.len()) as u32,
        )),
    };

    let mut errors = vec![];

    // TODO return errors if any occurred, otherwise Ok
    match parse_iso_literal(&iso_literal_text, interned_file_path, text_source) {
        Ok(res) => {
            let extraction_result = res.item;
            if let IsoLiteralExtractionResult::ClientFieldDeclaration(resolver_declaration) =
                &extraction_result
            {
                let exists_and_matches = const_export_name_exists_and_matches(
                    const_export_name,
                    resolver_declaration.item.resolver_field_name.item,
                );
                if !has_associated_js_function {
                    errors.push(WithLocation::new(
                        IsographLiteralParseError::ExpectedAssociatedJsFunction,
                        Location::new(text_source, Span::todo_generated()),
                    ));
                }
                if exists_and_matches {
                    if errors.is_empty() {
                        Ok((extraction_result, text_source))
                    } else {
                        Err(errors)
                    }
                } else {
                    errors.push(WithLocation::new(
                        IsographLiteralParseError::ExpectedLiteralToBeExported {
                            expected_const_export_name: resolver_declaration
                                .item
                                .resolver_field_name
                                .item,
                        },
                        // TODO why does resolver_declaration.span cause a panic here?
                        Location::new(text_source, Span::todo_generated()),
                    ));
                    Err(errors)
                }
            } else {
                Ok((extraction_result, text_source))
            }
        }
        Err(e) => {
            errors.push(e);
            Err(errors)
        }
    }
}

fn const_export_name_exists_and_matches(
    const_export_name: Option<&str>,
    resolver_name: ScalarFieldName,
) -> bool {
    match const_export_name {
        Some(const_export_name) => const_export_name == resolver_name.lookup(),
        None => false,
    }
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
        "{} when validating schema, resolvers and fetch declarations.{}",
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
