use std::{path::PathBuf, str::Utf8Error};

use colored::Colorize;
use common_lang_types::{
    FilePath, Location, SourceFileName, Span, TextSource, WithLocation, WithSpan,
};
use graphql_artifact_generation::get_artifact_path_and_content;
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions, SchemaParseError};
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use isograph_lang_parser::{
    parse_iso_literal, IsoLiteralExtractionResult, IsographLiteralParseError,
};
use isograph_lang_types::{
    ClientFieldDeclarationWithUnvalidatedDirectives, ClientFieldDeclarationWithValidatedDirectives,
    EntrypointTypeAndField,
};
use isograph_schema::{
    ProcessClientFieldDeclarationError, ProcessGraphQLDocumentOutcome, Schema, UnvalidatedSchema,
    ValidateSchemaError,
};
use pretty_duration::pretty_duration;
use thiserror::Error;

use crate::{
    field_directives::validate_isograph_field_directives,
    isograph_literals::{
        extract_iso_literals_from_file_content, read_files_in_folder, IsoLiteralExtraction,
    },
    refetch_fields::add_refetch_fields_to_objects,
    schema::read_schema_file,
    with_duration::WithDuration,
    write_artifacts::{write_artifacts_to_disk, GenerateArtifactsError},
};

pub struct CompilationStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub total_artifacts_written: usize,
}

pub fn compile_and_print(config: &CompilerConfig) -> Result<CompilationStats, BatchCompileError> {
    eprintln!("{}", "Starting to compile.".cyan());

    let result = WithDuration::new(|| handle_compile_command(config));
    let elapsed_time = result.elapsed_time;

    match result.item {
        Ok(stats) => {
            eprintln!(
                    "{}",
                    format!(
                        "Successfully compiled {} client fields and {} entrypoints, and wrote {} artifacts, in {}.\n",
                        stats.client_field_count,
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

/// This the "workhorse" command of batch compilation. It is currently run in a loop
/// in watch mode.
///
/// ## Overall plan
///
/// When the compiler runs in batch mode, we must do the following things. This
/// description is a bit simplified.
///
/// - Read and parse things:
///   - Read and parse the GraphQL schema
///   - Read and parse the Isograph literals
/// - Combine everything into an UnvalidatedSchema.
/// - Turn the UnvalidatedSchema into a ValidatedSchema
///   - Note: at this point, we do most of the validations, like ensuring that
///     all selected fields exist and are of the correct types, parameters are
///     passed when needed, etc.
/// - Generate an in-memory representation of all of the generated files
///   (called artifacts). This step should not fail. It should panic if any
///   invariant is violated, or represent that invariant in the type system.
/// - Delete and recreate the artifacts on disk.
///
/// ## Additional things we do
///
/// In addition to the things we do above, we also do some specific things like:
///
/// - if a client field is defined on an interface, add it to each concrete
///   type. So, if User implements Actor, you can define Actor.NameDisplay, and
///   select User.NameDisplay
/// - create fields from exposeAs directives
///
/// These are less "core" to the overall mission, and thus invite the question
/// of whether they belong in this function, or at all.
///
/// ## Sequentially written vs Salsa architecture
///
/// Isograph is currently written in a fairly sequential fashion, e.g.:
///
/// let result_1 = step_1()?;
/// let result_2 = step_2()?;
/// step_3(result_1, result_2)?;
///
/// Where each step is completed before the next one starts. This has advantages:
/// namely, it is easy to read. But, we most likely want to report all the errors
/// we can (i.e. from both step_1 and step_2), rather than just the first error
/// encountered (i.e. just step_1).
///
/// In the long term, we want to describe everything as a tree, e.g.
/// `step_3 -> [step_1, step_2]`, and this will "naturally" parallelize everything.
/// This is also necessary to adopt a Rust Analyzer-like (Salsa) architecture, which is
/// important for language server performance. In a Salsa architecture, we invalidate
/// leaves (e.g. a given file changed), and invalidate everything that depends on that
/// leaf. Then, when we need a result (e.g. the errors to show on a given file), we
/// re-evaluate (or re-use the cached value) of everything from that result on down.
pub(crate) fn handle_compile_command(
    config: &CompilerConfig,
) -> Result<CompilationStats, BatchCompileError> {
    let mut unvalidated_schema = UnvalidatedSchema::new();

    let original_outcome = process_graphql_schema_and_extensions(config, &mut unvalidated_schema)?;

    process_exposed_fields(&mut unvalidated_schema)?;

    let (client_field_declarations, entrypoint_declarations) =
        read_project_files_and_extract_iso_literals(config)?;

    let client_field_count = client_field_declarations.len();
    let entrypoint_count = entrypoint_declarations.len();

    add_client_fields_to_schema(&mut unvalidated_schema, client_field_declarations)?;
    add_entrypoints_to_schema(&mut unvalidated_schema, entrypoint_declarations);

    unvalidated_schema.add_fields_to_subtypes(
        &original_outcome
            .type_refinement_maps
            .supertype_to_subtype_map,
    )?;

    add_refetch_fields_to_objects(&mut unvalidated_schema)?;

    let validated_schema = Schema::validate_and_construct(unvalidated_schema)?;

    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.
    let paths_and_contents = get_artifact_path_and_content(
        &validated_schema,
        &config.project_root,
        &config.artifact_directory,
    );

    let total_artifacts_written =
        write_artifacts_to_disk(paths_and_contents, &config.artifact_directory)?;

    Ok(CompilationStats {
        client_field_count,
        entrypoint_count,
        total_artifacts_written,
    })
}

fn read_project_files_and_extract_iso_literals(
    config: &CompilerConfig,
) -> Result<
    (
        Vec<(
            WithSpan<ClientFieldDeclarationWithValidatedDirectives>,
            TextSource,
        )>,
        Vec<(WithSpan<EntrypointTypeAndField>, TextSource)>,
    ),
    BatchCompileError,
> {
    let canonicalized_root_path = get_canonicalized_root_path(config)?;
    let project_files = read_files_in_folder(&canonicalized_root_path)?;
    let (client_field_declarations, parsed_entrypoints) =
        extract_iso_literals(project_files, canonicalized_root_path)
            .map_err(BatchCompileError::from)?;

    // Validate @loadable
    let client_field_declarations = validate_isograph_field_directives(client_field_declarations)?;

    Ok((client_field_declarations, parsed_entrypoints))
}

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields(
    schema: &mut Schema<isograph_schema::UnvalidatedSchemaState>,
) -> Result<(), BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    Ok(for fetchable_object_id in fetchable_types.into_iter() {
        schema.add_exposed_fields_to_parent_object_types(fetchable_object_id)?;
    })
}

fn process_graphql_schema_and_extensions(
    config: &CompilerConfig,
    schema: &mut UnvalidatedSchema,
) -> Result<ProcessGraphQLDocumentOutcome, BatchCompileError> {
    let type_system_document = read_and_parse_graphql_schema(config)?;
    let type_extension_documents = read_and_parse_graphql_schema_extension(config)?;
    let original_outcome =
        schema.process_graphql_type_system_document(type_system_document, config.options)?;
    for extension_document in type_extension_documents {
        let _extension_outcome =
            schema.process_graphql_type_extension_document(extension_document, config.options)?;
        // TODO extend the process_graphql_outcome.type_refinement_map and the one
        // from the extensions? Does that even make sense?
        // TODO validate that we didn't define any new root types (as they are ignored)
    }
    Ok(original_outcome)
}

fn get_canonicalized_root_path(config: &CompilerConfig) -> Result<PathBuf, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(&config.project_root);
    joined
        .canonicalize()
        .map_err(|message| BatchCompileError::UnableToLoadSchema {
            path: joined.clone(),
            message,
        })
}

fn read_and_parse_graphql_schema_extension(
    config: &CompilerConfig,
) -> Result<Vec<GraphQLTypeSystemExtensionDocument>, BatchCompileError> {
    config
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
            let type_extension_document =
                parse_schema_extensions(&extension_content, extension_text_source)
                    .map_err(|with_span| with_span.to_with_location(extension_text_source))?;
            Ok(type_extension_document)
        })
        .collect::<Result<Vec<_>, BatchCompileError>>()
}

fn read_and_parse_graphql_schema(
    config: &CompilerConfig,
) -> Result<GraphQLTypeSystemDocument, BatchCompileError> {
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
    Ok(type_system_document)
}

fn add_client_fields_to_schema(
    schema: &mut UnvalidatedSchema,
    client_field_declarations: Vec<(
        WithSpan<ClientFieldDeclarationWithValidatedDirectives>,
        TextSource,
    )>,
) -> Result<(), Vec<WithLocation<ProcessClientFieldDeclarationError>>> {
    let mut errors = vec![];
    for (client_field_declaration, text_source) in client_field_declarations {
        if let Err(e) =
            schema.process_client_field_declaration(client_field_declaration, text_source)
        {
            errors.push(e);
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn add_entrypoints_to_schema(
    schema: &mut Schema<isograph_schema::UnvalidatedSchemaState>,
    entrypoint_declarations: Vec<(WithSpan<EntrypointTypeAndField>, TextSource)>,
) {
    for (entrypoint_declaration, text_source) in entrypoint_declarations {
        schema
            .entrypoints
            .push((text_source, entrypoint_declaration))
    }
}

#[allow(clippy::complexity)]
fn extract_iso_literals(
    project_files: Vec<(PathBuf, String)>,
    canonicalized_root_path: PathBuf,
) -> Result<
    (
        Vec<(
            WithSpan<ClientFieldDeclarationWithUnvalidatedDirectives>,
            TextSource,
        )>,
        Vec<(WithSpan<EntrypointTypeAndField>, TextSource)>,
    ),
    Vec<WithLocation<IsographLiteralParseError>>,
> {
    let mut isograph_literal_parse_errors = vec![];
    let mut client_field_declarations_and_text_sources = vec![];
    let mut entrypoint_declarations_and_text_sources = vec![];

    for (file_path, file_content) in project_files {
        // TODO don't intern unless there's a match
        let interned_file_path = file_path.to_string_lossy().into_owned().intern().into();

        let file_name = canonicalized_root_path
            .join(file_path)
            .to_str()
            .expect("file_path should be a valid string")
            .intern()
            .into();

        for iso_literal_extraction in extract_iso_literals_from_file_content(&file_content) {
            match process_iso_literal_extraction(
                iso_literal_extraction,
                file_name,
                interned_file_path,
            ) {
                Ok((extraction_result, text_source)) => match extraction_result {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(decl) => {
                        client_field_declarations_and_text_sources.push((decl, text_source))
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(decl) => {
                        entrypoint_declarations_and_text_sources.push((decl, text_source))
                    }
                },
                Err(e) => isograph_literal_parse_errors.push(e),
            }
        }
    }

    if isograph_literal_parse_errors.is_empty() {
        Ok((
            client_field_declarations_and_text_sources,
            entrypoint_declarations_and_text_sources,
        ))
    } else {
        Err(isograph_literal_parse_errors)
    }
}

fn process_iso_literal_extraction(
    iso_literal_extraction: IsoLiteralExtraction<'_>,
    file_name: SourceFileName,
    interned_file_path: FilePath,
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
        iso_literal_text,
        interned_file_path,
        const_export_name,
        text_source,
    )?;

    #[allow(clippy::collapsible_if)]
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
pub enum BatchCompileError {
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

    #[error("Unable to parse schema.\n\n{0}")]
    UnableToParseSchema(#[from] WithLocation<SchemaParseError>),

    #[error(
        "{}{}",
        if messages.len() == 1 { "Unable to parse Isograph literal:" } else { "Unable to parse Isograph literals:" },
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
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
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    ErrorWhenProcessingClientFieldDeclaration {
        messages: Vec<WithLocation<isograph_schema::ProcessClientFieldDeclarationError>>,
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
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x));
            output
        })
    )]
    UnableToValidateSchema {
        messages: Vec<WithLocation<isograph_schema::ValidateSchemaError>>,
    },

    #[error("Unable to print.\nReason: {0}")]
    UnableToPrint(#[from] GenerateArtifactsError),

    #[error("Unable to convert file {path:?} to utf8.\nDetailed reason: {reason}")]
    UnableToConvertToString { path: PathBuf, reason: Utf8Error },

    #[error("The __refetch field was already defined. Isograph creates it automatically; you cannot create it.")]
    DuplicateRefetchField,
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

impl From<Vec<WithLocation<ProcessClientFieldDeclarationError>>> for BatchCompileError {
    fn from(messages: Vec<WithLocation<ProcessClientFieldDeclarationError>>) -> Self {
        BatchCompileError::ErrorWhenProcessingClientFieldDeclaration { messages }
    }
}
