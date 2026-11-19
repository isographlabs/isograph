use common_lang_types::RelativePathToSourceFile;
use isograph_parser::{ParseError, ParsedIsoLiteral, parse_iso_literal};
use pico_macros::memo;
use prelude::Postfix;
use span::{WithSpan, WithSpanPostfix};

use crate::host_language::{CompilationProfile, HostLanguage, IsoLiteralExtraction};
use crate::{ClientId, IsographDb, file_contents};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

#[memo]
pub fn parsed_iso_literal<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    iso_literal_text: String,
) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}

#[memo]
pub fn extract_iso_literals<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<TProfile::HostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    TProfile::HostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}

#[memo]
pub fn parsed_iso_literals_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<
    Vec<(
        IsoLiteralExtraction<TProfile::HostLanguage>,
        ParsedIsoLiteral,
    )>,
> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    extractions
        .iter()
        .map(|extraction| {
            (
                extraction.clone(),
                parsed_iso_literal(db, extraction.iso_literal_text.clone()).clone(),
            )
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// Parse errors of every iso literal in `path` as `client` reads it, as file-absolute spans.
#[memo]
pub fn iso_literal_parse_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<ParseError>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| {
            let offset = extraction.iso_literal_start_index.0 as u32;
            parsed.errors.iter().map(move |error| {
                error
                    .item
                    .clone()
                    .with_span(error.location.with_offset(offset))
            })
        })
        .collect::<Vec<_>>()
        .wrap_some()
}

/// Host language errors of every iso literal in `path` as `client` reads it, as file-absolute
/// spans.
#[memo]
pub fn iso_literal_host_errors_in_file<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<WithSpan<<TProfile::HostLanguage as HostLanguage>::Error>>> {
    let literals = parsed_iso_literals_in_file(db, client, path).as_ref()?;
    literals
        .iter()
        .flat_map(|(extraction, parsed)| TProfile::HostLanguage::literal_errors(extraction, parsed))
        .collect::<Vec<_>>()
        .wrap_some()
}

#[memo]
pub fn text_through_last_iso_literal<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<String> {
    let extractions = extract_iso_literals(db, client, path).as_ref()?;
    let contents = file_contents(db, client, path).as_ref()?;
    let end = extractions
        .last()
        .map(|last| last.span().as_usize_range().end)
        .unwrap_or(0);
    contents[..end].to_owned().wrap_some()
}

#[memo]
pub fn iso_literal_extraction<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<TProfile::HostLanguage>> {
    let extractions = extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}

pub fn parsed_iso_literal_by_id<TProfile: CompilationProfile>(
    db: &IsographDb<TProfile>,
    literal_id: LiteralId,
) -> Option<&ParsedIsoLiteral> {
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    parsed_iso_literal(db, extraction.iso_literal_text.clone()).wrap_some()
}
