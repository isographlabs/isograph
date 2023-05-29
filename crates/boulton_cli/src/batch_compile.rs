use std::path::PathBuf;

use boulton_lang_parser::parse_bdeclare_literal;
use boulton_schema::Schema;
use graphql_lang_parser::parse_schema;
use intern::string_key::Intern;
use structopt::StructOpt;
use thiserror::Error;

use crate::{
    boulton_literals::{extract_b_declare_literal_from_file_content, read_files_in_folder},
    generate_artifacts::{generate_query_artifacts, PrintError},
    schema::read_schema_file,
};

/// Options if we're doing a batch compilation
#[derive(Debug, StructOpt)]
pub(crate) struct BatchCompileCliOptions {
    /// Source schema file
    #[structopt(long)]
    schema: PathBuf,

    /// Source JS directory
    #[structopt(long)]
    #[allow(unused)]
    project_root: PathBuf,
}

pub(crate) fn handle_compile_command(opt: BatchCompileCliOptions) -> Result<(), BatchCompileError> {
    let content = read_schema_file(opt.schema)?;
    let type_system_document = parse_schema(&content)?;
    let mut schema = Schema::new();

    schema.process_type_system_document(type_system_document)?;

    // TODO return an iterator
    let project_files = read_files_in_folder(&opt.project_root)?;
    for (file_path, file_content) in project_files {
        // TODO don't intern unless there's a match
        let interned_file_path = file_path.to_string_lossy().into_owned().intern().into();

        let b_declare_literals = extract_b_declare_literal_from_file_content(&file_content);
        for b_declare_literal in b_declare_literals {
            let resolver_declaration =
                parse_bdeclare_literal(&b_declare_literal, interned_file_path)?;
            schema.process_resolver_declaration(resolver_declaration)?;
        }
    }

    let validated_schema = Schema::validate_and_construct(schema)?;

    generate_query_artifacts(&validated_schema, &opt.project_root)?;
    // dbg!(validated_schema);

    Ok(())
}

#[derive(Error, Debug)]
pub(crate) enum BatchCompileError {
    #[error("Unable to load schema file at path {path:?}.\nMessage: {message:?}")]
    UnableToLoadSchema {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Attempted to load the graphql schema at the following path: {path:?}, but that is not a file.")]
    SchemaNotAFile { path: PathBuf },

    #[error("The project root at the following path: \"{path:?}\", is not a directory.")]
    ProjectRootNotADirectory { path: PathBuf },

    #[error("Unable to read the file at the following path: {path:?}.\nMessage: {message:?}")]
    UnableToReadFile {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Unable to traverse directory.\nMessage: {message:?}")]
    UnableToTraverseDirectory { message: std::io::Error },

    #[error("Unable to convert schema to string.\nMessage: {message:?}")]
    UnableToConvertToString { message: std::str::Utf8Error },

    #[error("Unable to parse schema.\nMessage: {message:?}")]
    UnableToParseSchema {
        message: graphql_lang_parser::SchemaParseError,
    },

    #[error("Unable to parse boulton literal.\nMessage: {message}")]
    UnableToParseBoultonLiteral {
        message: boulton_lang_parser::BoultonLiteralParseError,
    },

    #[error("Unable to create schema.\nMessage: {message:?}")]
    UnableToCreateSchema {
        message: boulton_schema::ProcessTypeDefinitionError,
    },

    #[error("Error when processing resolver declaration.\nMessage: {message:?}")]
    ErrorWhenProcessingResolverDeclaration {
        message: boulton_schema::ProcessResolverDeclarationError,
    },

    #[error("Unable to strip prefix.\nMessage: {message:?}")]
    UnableToStripPrefix {
        message: std::path::StripPrefixError,
    },

    #[error("Unable to validate schema.\nMessage: {message:?}")]
    UnableToValidateSchema {
        message: boulton_schema::ValidateSchemaError,
    },

    #[error("Unable to print.\nMessage: {message:?}")]
    UnableToPrint { message: PrintError },
}

impl From<graphql_lang_parser::SchemaParseError> for BatchCompileError {
    fn from(value: graphql_lang_parser::SchemaParseError) -> Self {
        BatchCompileError::UnableToParseSchema { message: value }
    }
}

impl From<boulton_lang_parser::BoultonLiteralParseError> for BatchCompileError {
    fn from(value: boulton_lang_parser::BoultonLiteralParseError) -> Self {
        BatchCompileError::UnableToParseBoultonLiteral { message: value }
    }
}

impl From<boulton_schema::ProcessTypeDefinitionError> for BatchCompileError {
    fn from(value: boulton_schema::ProcessTypeDefinitionError) -> Self {
        BatchCompileError::UnableToCreateSchema { message: value }
    }
}

impl From<boulton_schema::ProcessResolverDeclarationError> for BatchCompileError {
    fn from(value: boulton_schema::ProcessResolverDeclarationError) -> Self {
        BatchCompileError::ErrorWhenProcessingResolverDeclaration { message: value }
    }
}

impl From<std::path::StripPrefixError> for BatchCompileError {
    fn from(value: std::path::StripPrefixError) -> Self {
        BatchCompileError::UnableToStripPrefix { message: value }
    }
}

impl From<boulton_schema::ValidateSchemaError> for BatchCompileError {
    fn from(value: boulton_schema::ValidateSchemaError) -> Self {
        BatchCompileError::UnableToValidateSchema { message: value }
    }
}

impl From<PrintError> for BatchCompileError {
    fn from(value: PrintError) -> Self {
        BatchCompileError::UnableToPrint { message: value }
    }
}
