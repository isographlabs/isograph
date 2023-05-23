use std::path::PathBuf;

use boulton_lang_parser::parse_bdeclare_literal;
use graphql_lang_parser::parse_schema;
use structopt::StructOpt;
use thiserror::Error;

use crate::{
    boulton_literals::{extract_b_declare_literal_from_file_content, read_files_in_folder},
    schema::{process_schema_def, read_schema_file},
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
    let schema_def = parse_schema(&content)?;
    let mut schema = process_schema_def(schema_def);

    let project_files = read_files_in_folder(opt.project_root)?;
    for file_content in project_files {
        let b_declare_literals = extract_b_declare_literal_from_file_content(&file_content);
        for b_declare_literal in b_declare_literals {
            let resolver_declaration = parse_bdeclare_literal(&b_declare_literal)?;
            schema.process_resolver_declaration(resolver_declaration);
        }
    }

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
