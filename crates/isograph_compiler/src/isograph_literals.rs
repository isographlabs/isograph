use std::{
    fs::{self, DirEntry},
    io,
    path::{Path, PathBuf},
};

use common_lang_types::{FilePath, Location, SourceFileName, Span, TextSource, WithLocation};
use intern::string_key::Intern;
use isograph_lang_parser::{
    parse_iso_literal, IsoLiteralExtractionResult, IsographLiteralParseError,
};
use isograph_schema::UnvalidatedSchema;
use lazy_static::lazy_static;
use regex::Regex;

use crate::{
    batch_compile::BatchCompileError,
    field_directives::{validate_isograph_field_directives, validate_isograph_pointer_directives},
    source_files::ContainsIso,
};

pub(crate) fn read_files_in_folder(
    folder: &Path,
    canonicalized_root_path: &Path,
) -> Result<Vec<(PathBuf, String)>, BatchCompileError> {
    if !canonicalized_root_path.is_dir() {
        return Err(BatchCompileError::ProjectRootNotADirectory {
            // TODO avoid cloning
            path: canonicalized_root_path.to_path_buf(),
        });
    }

    read_dir_recursive(folder)?
        .into_iter()
        .filter(|p| {
            let extension = p.extension().and_then(|x| x.to_str());

            matches!(
                extension,
                Some("ts") | Some("tsx") | Some("js") | Some("jsx")
            )
        })
        .filter(|p| {
            !p.to_str()
                .expect("Expected path to be stringable")
                .contains("__isograph")
        })
        .map(|path| read_file(path, canonicalized_root_path))
        .collect()
}

pub fn read_file(
    path: PathBuf,
    canonicalized_root_path: &Path,
) -> Result<(PathBuf, String), BatchCompileError> {
    // This isn't ideal. We can avoid a clone if we changed .map_err to match
    let path_2 = path.clone();

    // N.B. we have previously ensured that path is a file
    let contents = std::fs::read(&path).map_err(|message| BatchCompileError::UnableToReadFile {
        path: path_2,
        message,
    })?;

    let contents = std::str::from_utf8(&contents)
        .map_err(|e| BatchCompileError::UnableToConvertToString {
            path: path.clone(),
            reason: e,
        })?
        .to_owned();

    Ok((
        path.strip_prefix(canonicalized_root_path)?.to_path_buf(),
        contents,
    ))
}

fn read_dir_recursive(root_js_path: &Path) -> Result<Vec<PathBuf>, BatchCompileError> {
    let mut paths = vec![];

    visit_dirs_skipping_isograph(root_js_path, &mut |dir_entry| {
        paths.push(dir_entry.path());
    })
    .map_err(BatchCompileError::from)?;

    Ok(paths)
}

// Thanks https://doc.rust-lang.org/stable/std/fs/fn.read_dir.html
fn visit_dirs_skipping_isograph(dir: &Path, cb: &mut dyn FnMut(&DirEntry)) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            if !dir.ends_with(ISOGRAPH_FOLDER) {
                visit_dirs_skipping_isograph(&path, cb)?;
            }
        } else {
            cb(&entry);
        }
    }
    Ok(())
}

#[allow(clippy::type_complexity)]
pub(crate) fn read_and_parse_iso_literals(
    file_path: PathBuf,
    file_content: String,
    canonicalized_root_path: &Path,
) -> Result<
    (
        SourceFileName,
        Vec<(IsoLiteralExtractionResult, TextSource)>,
    ),
    Vec<WithLocation<IsographLiteralParseError>>,
> {
    // TODO don't intern unless there's a match
    let interned_file_path = file_path.to_string_lossy().into_owned().intern().into();

    let file_name = canonicalized_root_path
        .join(file_path)
        .to_str()
        .expect("file_path should be a valid string")
        .intern()
        .into();

    let mut extraction_results = vec![];
    let mut isograph_literal_parse_errors = vec![];

    for iso_literal_extraction in extract_iso_literals_from_file_content(&file_content) {
        match process_iso_literal_extraction(iso_literal_extraction, file_name, interned_file_path)
        {
            Ok(result) => extraction_results.push(result),
            Err(e) => isograph_literal_parse_errors.push(e),
        }
    }

    if isograph_literal_parse_errors.is_empty() {
        Ok((file_name, extraction_results))
    } else {
        Err(isograph_literal_parse_errors)
    }
}

pub(crate) fn process_iso_literals(
    schema: &mut UnvalidatedSchema,
    contains_iso: ContainsIso,
) -> Result<(), BatchCompileError> {
    let mut errors = vec![];
    for iso_literals in contains_iso.0.into_values() {
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
                IsoLiteralExtractionResult::ClientPointerDeclaration(
                    client_pointer_declaration,
                ) => {
                    match validate_isograph_pointer_directives(client_pointer_declaration) {
                        Ok(validated_client_pointer_declaration) => {
                            if let Err(e) = schema.process_client_pointer_declaration(
                                validated_client_pointer_declaration,
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

pub fn process_iso_literal_extraction(
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

pub(crate) static ISOGRAPH_FOLDER: &str = "__isograph";
lazy_static! {
    static ref EXTRACT_ISO_LITERAL: Regex =
        Regex::new(r"(export const ([^ ]+) =\s+)?iso(\()?`([^`]+)`(\))?(\()?").unwrap();
}

pub struct IsoLiteralExtraction<'a> {
    pub const_export_name: Option<&'a str>,
    pub iso_literal_text: &'a str,
    pub iso_literal_start_index: usize,
    pub has_associated_js_function: bool,
    pub has_paren: bool,
}

pub fn extract_iso_literals_from_file_content(
    content: &str,
) -> impl Iterator<Item = IsoLiteralExtraction> + '_ {
    EXTRACT_ISO_LITERAL.captures_iter(content).map(|captures| {
        let iso_literal_match = captures.get(4).unwrap();
        IsoLiteralExtraction {
            const_export_name: captures.get(1).map(|_| captures.get(2).unwrap().as_str()),
            iso_literal_text: iso_literal_match.as_str(),
            iso_literal_start_index: iso_literal_match.start(),
            has_associated_js_function: captures.get(6).is_some(),
            has_paren: captures.get(3).is_some(),
        }
    })
}
