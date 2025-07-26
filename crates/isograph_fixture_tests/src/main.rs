use std::{ffi::OsStr, fs, path::PathBuf};

use clap::Parser;
use common_lang_types::{
    relative_path_from_absolute_and_working_directory, CurrentWorkingDirectory,
};
use intern::{string_key::Intern, Lookup};
use isograph_compiler::parse_iso_literals_in_file_content;
use lazy_static::lazy_static;
use regex::Regex;

fn main() {
    let args = FixtureOpt::parse();

    if args.dir.is_empty() {
        panic!("At least one directory must be provided.");
    }

    let current_dir = std::env::current_dir().expect("current_dir should exist");

    let current_working_directory = current_dir.to_str().unwrap().intern().into();

    for fixture_dir in args.dir {
        let canonicalized_folder = current_dir
            .join(fixture_dir.clone())
            .canonicalize()
            .unwrap_or_else(|_| {
                panic!("Failed to canonicalize {fixture_dir:?}");
            });

        if !canonicalized_folder.is_dir() {
            panic!("Expected {fixture_dir:?} to be a directory");
        }

        generate_fixtures_for_files_in_folder(canonicalized_folder, current_working_directory);
    }
}

#[derive(Debug, Parser)]
#[command(version, about, long_about=None)]
struct FixtureOpt {
    /// In what directories should we look for .input and .expected
    /// files? You must pass at least one.
    #[arg(long)]
    dir: Vec<PathBuf>,
}

lazy_static! {
    static ref INPUT_SUFFIX: Regex = Regex::new(r"(.+)\.input\.js$").unwrap();
}
const OUTPUT_SUFFIX: &str = r"output";

fn generate_fixtures_for_files_in_folder(
    folder: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) {
    for read_dir in folder.read_dir().expect("read_dir call failed") {
        match read_dir {
            Ok(entry) => {
                let path = entry.path();

                if !path.is_file() {
                    panic!("Expected {path:?} to be a file");
                }
                let path_as_str = path
                    .to_str()
                    .expect("Expected path to be able to be converted to string")
                    .to_string();

                if let Some(capture) = INPUT_SUFFIX.captures(&path_as_str) {
                    let mut output_file = PathBuf::from(capture.get(1).unwrap().as_str());
                    output_file.set_extension(OUTPUT_SUFFIX);
                    process_input_file(path, output_file, current_working_directory);
                } else if path.extension() == Some(OsStr::new(OUTPUT_SUFFIX)) {
                    // Great, ignore it.
                } else {
                    panic!(
                        "Invalid file {:?}. Files in this folder should either \
                        end in .{} or .{}",
                        path, *INPUT_SUFFIX, OUTPUT_SUFFIX
                    );
                }
            }
            Err(_) => panic!("Failed to read an item in {folder:?}"),
        }
    }
}

fn process_input_file(
    input_file: PathBuf,
    output_file: PathBuf,
    current_working_directory: CurrentWorkingDirectory,
) {
    let file_content = String::from_utf8(
        fs::read(input_file.clone())
            .unwrap_or_else(|_| panic!("Expected file {input_file:?} to be readable")),
    )
    .unwrap_or_else(|_| panic!("Content cannot be turned into string (path: {input_file:?})"));

    // N.B. for now, we are just parsing and printing those results.
    // But, we actually want to either just parse iso literals, or
    // parse the GraphQL schema, or parse and validate, or parse and
    // validate and generate artifacts, or something else entirely.
    //
    // So, we will need to make this a bit more flexible.
    let results =
        generate_content_for_output_file(input_file, file_content, current_working_directory);

    fs::write(output_file.clone(), results)
        .unwrap_or_else(|_| panic!("Failed to write to {output_file:?}"));
}

fn generate_content_for_output_file(
    input_file: PathBuf,
    content: String,
    current_working_directory: CurrentWorkingDirectory,
) -> String {
    let canonicalized_root_path = &PathBuf::from(current_working_directory.lookup());
    let absolute_path = canonicalized_root_path.join(&input_file);
    let relative_path_to_source_file = relative_path_from_absolute_and_working_directory(
        current_working_directory,
        &absolute_path,
    );
    match parse_iso_literals_in_file_content(
        relative_path_to_source_file,
        &content,
        current_working_directory,
    ) {
        Ok(item) => {
            let item: Result<_, ()> = Ok(item);
            format!("{item:#?}")
        }
        Err(errs) => {
            let mut s = String::new();
            for err in errs {
                let err_printed = format!("{err}");
                let wrapped_err: Result<(), _> = Err(err);
                s.push_str(&format!("{wrapped_err:#?}\n\n{err_printed}\n---\n"));
            }
            s
        }
    }
}
