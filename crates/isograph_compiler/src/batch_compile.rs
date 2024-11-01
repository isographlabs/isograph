use std::{collections::HashMap, path::PathBuf, str::Utf8Error};

use crate::{
    field_directives::validate_isograph_field_directives,
    parse_files::{IsoLiteralParseStats, ParsedFiles},
    refetch_fields::add_refetch_fields_to_objects,
    with_duration::WithDuration,
    write_artifacts::{write_artifacts_to_disk, GenerateArtifactsError},
};
use colored::Colorize;
use common_lang_types::{FilePath, TextSource, WithLocation};
use graphql_artifact_generation::get_artifact_path_and_content;
use graphql_schema_parser::SchemaParseError;

use isograph_config::CompilerConfig;
use isograph_lang_parser::{IsoLiteralExtractionResult, IsographLiteralParseError};
use isograph_schema::{
    ProcessClientFieldDeclarationError, Schema, UnvalidatedSchema, ValidateSchemaError,
};

use pretty_duration::pretty_duration;
use thiserror::Error;

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
                    "Successfully compiled {} client fields and {} \
                        entrypoints, and wrote {} artifacts, in {}.\n",
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
    // Parse
    let parsed_files = ParsedFiles::new(config)?;
    let IsoLiteralParseStats {
        client_field_count,
        entrypoint_count,
    } = parsed_files.iso_literal_stats();

    // Create schema
    let mut unvalidated_schema = UnvalidatedSchema::new();
    create_unvalidated_schema_from_sources(&mut unvalidated_schema, parsed_files, config)?;

    // Validate
    let validated_schema = Schema::validate_and_construct(unvalidated_schema)?;

    // Note: we calculate all of the artifact paths and contents first, so that writing to
    // disk can be as fast as possible and we minimize the chance that changes to the file
    // system occur while we're writing and we get unpredictable results.
    let paths_and_contents = get_artifact_path_and_content(
        &validated_schema,
        &config.project_root,
        &config.artifact_directory,
        &config.options.generate_file_extensions,
    );

    let total_artifacts_written =
        write_artifacts_to_disk(paths_and_contents, &config.artifact_directory)?;

    Ok(CompilationStats {
        client_field_count,
        entrypoint_count,
        total_artifacts_written,
    })
}

fn create_unvalidated_schema_from_sources(
    schema: &mut UnvalidatedSchema,
    sources: ParsedFiles,
    config: &CompilerConfig,
) -> Result<(), BatchCompileError> {
    let outcome = schema.process_graphql_type_system_document(sources.schema, &config.options)?;
    for extension_document in sources.schema_extensions.into_values() {
        let _extension_outcome =
            schema.process_graphql_type_extension_document(extension_document, &config.options)?;
    }
    process_iso_literals(schema, sources.contains_iso)?;
    process_exposed_fields(schema)?;
    schema.add_fields_to_subtypes(&outcome.type_refinement_maps.supertype_to_subtype_map)?;
    schema.add_pointers_to_supertypes(&outcome.type_refinement_maps.subtype_to_supertype_map)?;
    add_refetch_fields_to_objects(schema)?;
    Ok(())
}

fn process_iso_literals(
    schema: &mut UnvalidatedSchema,
    contains_iso: HashMap<FilePath, Vec<(IsoLiteralExtractionResult, TextSource)>>,
) -> Result<(), BatchCompileError> {
    let mut errors = vec![];
    for iso_literals in contains_iso.into_values() {
        for (extraction_result, text_source) in iso_literals {
            match extraction_result {
                IsoLiteralExtractionResult::ClientFieldDeclaration(client_field_declaration) => {
                    match validate_isograph_field_directives(client_field_declaration) {
                        Ok(validated_client_field_declaration) => {
                            if let Err(e) = schema.process_client_field_declaration(
                                validated_client_field_declaration,
                                text_source,
                            ) {
                                errors.push(e);
                            }
                        }
                        Err(e) => errors.extend(e),
                    };
                }
                IsoLiteralExtractionResult::EntrypointDeclaration(entrypoint_declaration) => schema
                    .entrypoints
                    .push((text_source, entrypoint_declaration)),
            }
        }
    }
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors.into())
    }
}

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields(schema: &mut UnvalidatedSchema) -> Result<(), BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    for fetchable_object_id in fetchable_types.into_iter() {
        schema.add_exposed_fields_to_parent_object_types(fetchable_object_id)?;
    }
    Ok(())
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
