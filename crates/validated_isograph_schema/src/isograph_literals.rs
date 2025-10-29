use common_lang_types::{
    CurrentWorkingDirectory, Location, RelativePathToSourceFile, Span, TextSource, WithLocation,
    WithSpan,
};
use isograph_lang_parser::{
    IsoLiteralExtractionResult, IsographLiteralParseError, parse_iso_literal,
};
use isograph_lang_types::{EntrypointDeclaration, SelectionType};
use isograph_schema::{
    IsoLiteralsSource, IsographDatabase, NetworkProtocol, ProcessClientFieldDeclarationError,
    Schema, UnprocessedItem,
};
use lazy_static::lazy_static;
use pico::SourceId;
use pico_macros::legacy_memo;
use regex::Regex;
use std::ops::Deref;

use crate::process_iso_literals::ParsedIsoLiteralsMap;

// TODO this should return a Vec of Results, since a file can contain
// both valid and invalid iso literals.
pub fn parse_iso_literals_in_file_content<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path_to_source_file: RelativePathToSourceFile,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<
    Vec<(IsoLiteralExtractionResult, TextSource)>,
    Vec<WithLocation<IsographLiteralParseError>>,
> {
    let mut extraction_results = vec![];
    let mut isograph_literal_parse_errors = vec![];

    for iso_literal_extraction in
        extract_iso_literals_from_file_content(db, relative_path_to_source_file)
            .deref()
            .iter()
    {
        match process_iso_literal_extraction(
            db,
            iso_literal_extraction,
            relative_path_to_source_file,
            current_working_directory,
        ) {
            Ok(result) => extraction_results.push(result),
            Err(e) => isograph_literal_parse_errors.push(e),
        }
    }

    if isograph_literal_parse_errors.is_empty() {
        Ok(extraction_results)
    } else {
        Err(isograph_literal_parse_errors)
    }
}

// TODO this (and the previous function) smell
pub fn parse_iso_literals_in_file_content_and_return_all<
    TNetworkProtocol: NetworkProtocol + 'static,
>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path_to_source_file: RelativePathToSourceFile,
    current_working_directory: CurrentWorkingDirectory,
) -> Vec<Result<(IsoLiteralExtractionResult, TextSource), WithLocation<IsographLiteralParseError>>>
{
    extract_iso_literals_from_file_content(db, relative_path_to_source_file)
        .deref()
        .iter()
        .map(|iso_literal_extraction| {
            process_iso_literal_extraction(
                db,
                iso_literal_extraction,
                relative_path_to_source_file,
                current_working_directory,
            )
        })
        .collect()
}

#[legacy_memo]
pub fn parse_iso_literal_in_source<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    iso_literals_source_id: SourceId<IsoLiteralsSource>,
) -> Result<
    Vec<(IsoLiteralExtractionResult, TextSource)>,
    Vec<WithLocation<IsographLiteralParseError>>,
> {
    let memo_ref = read_iso_literals_source(db, iso_literals_source_id);
    let IsoLiteralsSource {
        relative_path,
        content: _,
    } = memo_ref.deref();

    parse_iso_literals_in_file_content(db, *relative_path, db.get_current_working_directory())
}

#[legacy_memo]
pub fn read_iso_literals_source_from_relative_path<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path_to_source_file: RelativePathToSourceFile,
) -> Option<IsoLiteralsSource> {
    let iso_literals_source_id = db.get_iso_literal(relative_path_to_source_file)?;
    Some(read_iso_literals_source(db, iso_literals_source_id).to_owned())
}

/// We should (probably) never directly read SourceId<IsoLiteralsSource>, since if we do so,
/// we will ignore open files.
#[legacy_memo]
pub fn read_iso_literals_source<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    iso_literals_source_id: SourceId<IsoLiteralsSource>,
) -> IsoLiteralsSource {
    let IsoLiteralsSource {
        relative_path,
        content,
    } = db.get(iso_literals_source_id);

    let open_file = db.get_open_file(*relative_path);

    // Use the open file's content, if it exists, otherwise use the content of the file from the file system
    let content = open_file.map(|x| &db.get(x).content).unwrap_or(content);

    IsoLiteralsSource {
        relative_path: *relative_path,
        content: content.clone(),
    }
}

#[expect(clippy::type_complexity)]
pub(crate) fn process_iso_literals<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    schema: &mut Schema<TNetworkProtocol>,
    contains_iso: ParsedIsoLiteralsMap,
) -> Result<
    (
        Vec<UnprocessedItem>,
        Vec<(TextSource, WithSpan<EntrypointDeclaration>)>,
    ),
    Vec<WithLocation<ProcessClientFieldDeclarationError>>,
> {
    let mut errors = vec![];
    let mut unprocess_client_field_items = vec![];
    let mut unprocessed_entrypoints = vec![];
    for iso_literals in contains_iso.files.into_values() {
        for (extraction_result, text_source) in iso_literals {
            match extraction_result {
                IsoLiteralExtractionResult::ClientFieldDeclaration(client_field_declaration) => {
                    match schema.process_client_field_declaration(
                        db,
                        client_field_declaration,
                        text_source,
                    ) {
                        Ok(unprocessed_client_field_items) => unprocess_client_field_items
                            .push(SelectionType::Scalar(unprocessed_client_field_items)),
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }
                IsoLiteralExtractionResult::ClientPointerDeclaration(
                    client_pointer_declaration,
                ) => {
                    match schema.process_client_pointer_declaration(
                        db,
                        client_pointer_declaration,
                        text_source,
                    ) {
                        Ok(unprocessed_client_pointer_item) => unprocess_client_field_items
                            .push(SelectionType::Object(unprocessed_client_pointer_item)),
                        Err(e) => {
                            errors.push(e);
                        }
                    }
                }

                IsoLiteralExtractionResult::EntrypointDeclaration(entrypoint_declaration) => {
                    unprocessed_entrypoints.push((text_source, entrypoint_declaration))
                }
            }
        }
    }
    if errors.is_empty() {
        Ok((unprocess_client_field_items, unprocessed_entrypoints))
    } else {
        Err(errors)
    }
}

pub fn process_iso_literal_extraction<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    iso_literal_extraction: &IsoLiteralExtraction,
    relative_path_to_source_file: RelativePathToSourceFile,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<(IsoLiteralExtractionResult, TextSource), WithLocation<IsographLiteralParseError>> {
    let IsoLiteralExtraction {
        iso_literal_text,
        iso_literal_start_index,
        has_associated_js_function,
        const_export_name,
        iso_function_called_with_paren: has_paren,
    } = iso_literal_extraction;
    let text_source = TextSource {
        relative_path_to_source_file,
        span: Some(Span::new(
            *iso_literal_start_index as u32,
            (iso_literal_start_index + iso_literal_text.len()) as u32,
        )),
        current_working_directory,
    };

    if !has_paren {
        return Err(WithLocation::new(
            IsographLiteralParseError::ExpectedParenthesesAroundIsoLiteral,
            Location::new(text_source, Span::todo_generated()),
        ));
    }

    let iso_literal_extraction_result = memoized_parse_iso_literal(
        db,
        iso_literal_text.to_string(),
        relative_path_to_source_file,
        const_export_name.clone(),
        text_source,
    )
    .to_owned()?;

    let is_client_field_declaration = matches!(
        &iso_literal_extraction_result,
        IsoLiteralExtractionResult::ClientFieldDeclaration(_)
    );
    if is_client_field_declaration && !has_associated_js_function {
        return Err(WithLocation::new(
            IsographLiteralParseError::ExpectedAssociatedJsFunction,
            Location::new(text_source, Span::todo_generated()),
        ));
    }

    Ok((iso_literal_extraction_result, text_source))
}

lazy_static! {
    static ref EXTRACT_ISO_LITERAL: Regex =
        Regex::new(r"(// )?(export const ([^ ]+) =\s+)?iso(\()?`([^`]+)`(\))?(\()?").unwrap();
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IsoLiteralExtraction {
    pub const_export_name: Option<String>,
    pub iso_literal_text: String,
    pub iso_literal_start_index: usize,
    pub has_associated_js_function: bool,
    /// true if the iso function is called as iso(`...`), and false if it is
    /// called as iso`...`. This is tracked as a separate field because some users
    /// may assume that you write iso literals like you would graphql/gql literals
    /// (which are written as graphql`...`), and having a separate field means
    /// we can display a helpful error message (instead of silently ignoring.)
    pub iso_function_called_with_paren: bool,
}

#[legacy_memo]
pub fn extract_iso_literals_from_file_content<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    relative_path_to_source_file: RelativePathToSourceFile,
) -> Vec<IsoLiteralExtraction> {
    let memo_ref = read_iso_literals_source_from_relative_path(db, relative_path_to_source_file);
    let IsoLiteralsSource {
        relative_path: _,
        content,
    } = memo_ref
        .deref()
        .as_ref()
        .expect("Expected relative path to exist");

    EXTRACT_ISO_LITERAL
        .captures_iter(content)
        .flat_map(|captures| {
            let iso_literal_match = captures.get(5).unwrap();
            if captures.get(1).is_some() {
                // HACK
                // this iso literal is commented out using //, so skip it.
                return None;
            }
            Some(IsoLiteralExtraction {
                const_export_name: captures
                    .get(2)
                    .map(|_| captures.get(3).unwrap().as_str().to_string()),
                iso_literal_text: iso_literal_match.as_str().to_string(),
                iso_literal_start_index: iso_literal_match.start(),
                has_associated_js_function: captures.get(7).is_some(),
                iso_function_called_with_paren: captures.get(4).is_some(),
            })
        })
        .collect()
}

#[legacy_memo]
pub fn memoized_parse_iso_literal<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    iso_literal_text: String,
    definition_file_path: RelativePathToSourceFile,
    const_export_name: Option<String>,
    // TODO we should not pass the text source here! Whenever the iso literal
    // moves around the page, we break memoization, due to this parameter.
    text_source: TextSource,
) -> Result<IsoLiteralExtractionResult, WithLocation<IsographLiteralParseError>> {
    parse_iso_literal(
        iso_literal_text,
        definition_file_path,
        const_export_name,
        text_source,
    )
}
