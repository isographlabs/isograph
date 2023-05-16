use std::path::PathBuf;

use graphql_lang_parser::parse_schema;
use structopt::StructOpt;
use thiserror::Error;

/// Options if we're doing a batch compilation
#[derive(Debug, StructOpt)]
struct BatchCompileCliOptions {
    /// Source schema file
    #[structopt(long)]
    schema: PathBuf,
}

fn main() {
    let opt = BatchCompileCliOptions::from_args();
    let result = handle_compile_command(opt);

    match result {
        Ok(_) => eprintln!("Done."),
        Err(err) => {
            eprintln!("{}", err);
            std::process::exit(1);
        }
    }
}

fn handle_compile_command(opt: BatchCompileCliOptions) -> Result<(), BatchCompileErrors> {
    let content = read_schema_file(opt.schema)?;

    let _schema_def = parse_schema(&content);

    Ok(())
}

#[derive(Error, Debug)]
enum BatchCompileErrors {
    #[error("Unable to load schema file at path {path:?}.\nMessage: {message:?}")]
    UnableToLoadSchema {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Attempted to load the graphql schema at the following path: {path:?}, but that is not a file.")]
    SchemaNotAFile { path: PathBuf },

    #[error("Unable to read the schema at the following path: {path:?}.\nMessage: {message:?}")]
    UnableToReadSchema {
        path: PathBuf,
        message: std::io::Error,
    },

    #[error("Unable to convert schema to string.\nMessage: {message:?}")]
    UnableToConvertToString { message: std::str::Utf8Error },
}

/// Read schema file
fn read_schema_file(path: PathBuf) -> Result<String, BatchCompileErrors> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(path);
    let canonicalized_existing_path =
        joined
            .canonicalize()
            .map_err(|message| BatchCompileErrors::UnableToLoadSchema {
                path: joined,
                message,
            })?;

    if !canonicalized_existing_path.is_file() {
        return Err(BatchCompileErrors::SchemaNotAFile {
            path: canonicalized_existing_path,
        });
    }

    let contents = std::fs::read(canonicalized_existing_path.clone()).map_err(|message| {
        BatchCompileErrors::UnableToReadSchema {
            path: canonicalized_existing_path,
            message,
        }
    })?;

    let contents = std::str::from_utf8(&contents)
        .map_err(|message| BatchCompileErrors::UnableToConvertToString { message })?
        .to_owned();

    Ok(contents)
}
