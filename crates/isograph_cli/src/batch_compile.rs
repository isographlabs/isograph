use std::{
    path::PathBuf,
    time::{Duration, Instant},
};

use colored::Colorize;
use common_lang_types::{
    Location, ResolverDefinitionPath, SourceFileName, Span, TextSource, WithLocation, WithSpan,
};
use graphql_schema_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use isograph_lang_parser::{
    parse_iso_literal, IsoLiteralExtractionResult, IsographLiteralParseError,
};
use isograph_lang_types::{EntrypointTypeAndField, ResolverDeclaration};
use isograph_schema::{
    ProcessResolverDeclarationError, Schema, UnvalidatedSchema, ValidateSchemaError,
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
                        "Successfully compiled {} client fields and {} entrypoints, and wrote {} artifacts, in {}.\n",
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

        let original_outcome =
            schema.process_graphql_type_system_document(type_system_document, config.options)?;

        // TODO validate here! We should not allow a situation in which a base schema is invalid,
        // but is made valid by the presence of schema extensions.

        for extension_document in type_extension_document {
            let _extension_outcome = schema
                .process_graphql_type_extension_document(extension_document, config.options)?;
            // TODO extend the process_graphql_outcome.type_refinement_map and the one
            // from the extensions? Does that even make sense?
            // TODO validate that we didn't define any new root types (as they are ignored)
        }

        // TODO the ordering should be:
        // - process schema
        // - validate
        // - process schema extension
        // - validate
        // - add mutation fields
        // - process parsed literals
        // - validate resolvers
        if let Some(mutation_id) = &original_outcome.root_types.mutation {
            schema.create_magic_mutation_fields(*mutation_id, config.options)?;
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

        schema.add_fields_to_subtypes(
            &original_outcome
                .type_refinement_maps
                .supertype_to_subtype_map,
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

fn process_iso_literal_extraction(
    iso_literal_extraction: IsoLiteralExtraction<'_>,
    file_name: SourceFileName,
    interned_file_path: ResolverDefinitionPath,
) -> Result<(IsoLiteralExtractionResult, TextSource), WithLocation<IsographLiteralParseError>> {
    let IsoLiteralExtraction {
        iso_literal_text,
        iso_literal_start_index,
        has_associated_js_function,
        const_export_name,
        has_paren,
    } = iso_literal_extraction;
    let text_source = TextSource {
        path: file_name,
        span: Some(Span::new(
            iso_literal_start_index as u32,
            (iso_literal_start_index + iso_literal_text.len()) as u32,
        )),
    };

    if !has_paren {
        return Err(WithLocation::new(
            IsographLiteralParseError::ExpectedParenthesesAroundIsoLiteral,
            Location::new(text_source, Span::todo_generated()),
        ));
    }

    // TODO return errors if any occurred, otherwise Ok
    let iso_literal_extraction_result = parse_iso_literal(
        &iso_literal_text,
        interned_file_path,
        const_export_name,
        text_source,
    )?;

    if matches!(
        &iso_literal_extraction_result,
        IsoLiteralExtractionResult::ClientFieldDeclaration(_)
    ) {
        if !has_associated_js_function {
            return Err(WithLocation::new(
                IsographLiteralParseError::ExpectedAssociatedJsFunction,
                Location::new(text_source, Span::todo_generated()),
            ));
        }
    }

    Ok((iso_literal_extraction_result, text_source))
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

    #[error("Unable to traverse directory.\nReason: {0}")]
    UnableToTraverseDirectory(#[from] std::io::Error),

    #[error("Unable to convert schema to string.\nReason: {0}")]
    UnableToConvertToString(#[from] std::str::Utf8Error),

    #[error("Unable to parse schema.\n\n{0}")]
    UnableToParseSchema(#[from] WithLocation<SchemaParseError>),

    #[error(
        "{}{}",
        if messages.len() == 1 { "Unable to parse Isograph literal:" } else { "Unable to parse Isograph literals:" },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    UnableToParseIsographLiterals {
        messages: Vec<WithLocation<IsographLiteralParseError>>,
    },

    #[error("Unable to create schema.\nReason: {0}")]
    UnableToCreateSchema(#[from] WithLocation<isograph_schema::ProcessTypeDefinitionError>),

    #[error(
        "{}{}",
        if messages.len() == 1 {
            "Error when processing a client field declaration:"
        } else {
            "Errors when processing client field declarations:"
        },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    ErrorWhenProcessingResolverDeclaration {
        messages: Vec<WithLocation<isograph_schema::ProcessResolverDeclarationError>>,
    },

    #[error("Error when processing an entrypoint declaration.\nReason: {0}")]
    ErrorWhenProcessingEntrypointDeclaration(
        #[from] WithLocation<isograph_schema::ValidateEntrypointDeclarationError>,
    ),

    #[error("Unable to strip prefix.\nReason: {0}")]
    UnableToStripPrefix(#[from] std::path::StripPrefixError),

    #[error(
        "{} when validating schema, client fields and entrypoint declarations.{}",
        if messages.len() == 1 { "Error" } else { "Errors" },
        messages.into_iter().map(|x| format!("\n\n{x}")).collect::<String>()
    )]
    UnableToValidateSchema {
        messages: Vec<WithLocation<isograph_schema::ValidateSchemaError>>,
    },

    #[error("Unable to print.\nReason: {0}")]
    UnableToPrint(#[from] GenerateArtifactsError),
}

impl From<Vec<WithLocation<IsographLiteralParseError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<IsographLiteralParseError>>) -> Self {
        BatchCompileError::UnableToParseIsographLiterals { messages }
    }
}

impl From<Vec<WithLocation<ValidateSchemaError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ValidateSchemaError>>) -> Self {
        BatchCompileError::UnableToValidateSchema { messages }
    }
}

impl From<Vec<WithLocation<ProcessResolverDeclarationError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ProcessResolverDeclarationError>>) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverDeclaration { messages }
    }
}
