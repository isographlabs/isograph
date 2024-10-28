use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};

use common_lang_types::{FilePath, Location, SourceFileName, Span, TextSource, WithLocation};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use graphql_schema_parser::{parse_schema, parse_schema_extensions};
use intern::string_key::Intern;
use isograph_config::CompilerConfig;
use isograph_lang_parser::{
    parse_iso_literal, IsoLiteralExtractionResult, IsographLiteralParseError,
};

use crate::{
    batch_compile::BatchCompileError, extract_iso_literals_from_file_content,
    isograph_literals::read_files_in_folder, schema::read_schema_file, IsoLiteralExtraction,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParsedFiles {
    pub schema: GraphQLTypeSystemDocument,
    pub schema_extensions: HashMap<FilePath, GraphQLTypeSystemExtensionDocument>,
    pub contains_iso: HashMap<FilePath, Vec<(IsoLiteralExtractionResult, TextSource)>>,
}

impl ParsedFiles {
    pub fn new(config: &CompilerConfig) -> Result<Self, BatchCompileError> {
        let schema = read_and_parse_graphql_schema(&config.schema)?;
        let mut schema_extensions = HashMap::new();
        for schema_extension_path in config.schema_extensions.iter() {
            let (file_path, extensions_document) =
                read_and_parse_schema_extensions(schema_extension_path)?;
            schema_extensions.insert(file_path, extensions_document);
        }
        let canonicalized_root_path = get_canonicalized_root_path(&config.project_root)?;
        let mut contains_iso = HashMap::new();
        let mut iso_literal_parse_errors = vec![];
        for (file_path, file_content) in read_files_in_folder(&canonicalized_root_path)? {
            match read_and_parse_iso_literals(file_path, file_content, &canonicalized_root_path) {
                Ok((file_path, iso_literals)) => {
                    contains_iso.insert(file_path, iso_literals);
                }
                Err(e) => {
                    iso_literal_parse_errors.extend(e);
                }
            };
        }
        if !iso_literal_parse_errors.is_empty() {
            return Err(iso_literal_parse_errors.into());
        }
        Ok(Self {
            schema,
            schema_extensions,
            contains_iso,
        })
    }

    pub fn iso_literal_stats(&self) -> IsoLiteralParseStats {
        let mut client_field_count: usize = 0;
        let mut entrypoint_count: usize = 0;
        for iso_literals in self.contains_iso.values() {
            for (iso_literal, ..) in iso_literals {
                match iso_literal {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(_) => {
                        client_field_count += 1
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => entrypoint_count += 1,
                }
            }
        }
        IsoLiteralParseStats {
            client_field_count,
            entrypoint_count,
        }
    }
}

pub fn read_and_parse_iso_literals(
    file_path: PathBuf,
    file_content: String,
    canonicalized_root_path: &Path,
) -> Result<
    (FilePath, Vec<(IsoLiteralExtractionResult, TextSource)>),
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
        Ok((interned_file_path, extraction_results))
    } else {
        Err(isograph_literal_parse_errors)
    }
}

pub fn read_and_parse_graphql_schema(
    schema_path: &PathBuf,
) -> Result<GraphQLTypeSystemDocument, BatchCompileError> {
    let content = read_schema_file(schema_path)?;
    let schema_text_source = TextSource {
        path: schema_path
            .to_str()
            .expect("Expected schema to be valid string")
            .intern()
            .into(),
        span: None,
    };
    let schema = parse_schema(&content, schema_text_source)
        .map_err(|with_span| with_span.to_with_location(schema_text_source))?;
    Ok(schema)
}

pub fn read_and_parse_schema_extensions(
    schema_extension_path: &PathBuf,
) -> Result<(FilePath, GraphQLTypeSystemExtensionDocument), BatchCompileError> {
    let interned_file_path = schema_extension_path
        .to_string_lossy()
        .into_owned()
        .intern()
        .into();
    let extension_content = read_schema_file(schema_extension_path)?;
    let extension_text_source = TextSource {
        path: schema_extension_path
            .to_str()
            .expect("Expected schema extension to be valid string")
            .intern()
            .into(),
        span: None,
    };

    let schema_extensions = parse_schema_extensions(&extension_content, extension_text_source)
        .map_err(|with_span| with_span.to_with_location(extension_text_source))?;

    Ok((interned_file_path, schema_extensions))
}

fn get_canonicalized_root_path(project_root: &PathBuf) -> Result<PathBuf, BatchCompileError> {
    let current_dir = std::env::current_dir().expect("current_dir should exist");
    let joined = current_dir.join(project_root);
    joined
        .canonicalize()
        .map_err(|message| BatchCompileError::UnableToLoadSchema {
            path: joined.clone(),
            message,
        })
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

pub struct IsoLiteralParseStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
}
