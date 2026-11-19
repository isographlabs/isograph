use common_lang_types::RelativePathToSourceFile;
use isograph_parser::{ParsedIsoLiteral, parse_iso_literal};
use pico_macros::memo;
use prelude::Postfix;

use crate::host_language::{HostLanguage, IsoLiteralExtraction};
use crate::{ClientId, IsographDb, file_contents};

#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct LiteralId {
    pub client: ClientId,
    pub path: RelativePathToSourceFile,
    pub index: usize,
}

#[memo]
pub fn parsed_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    iso_literal_text: String,
) -> ParsedIsoLiteral {
    parse_iso_literal(iso_literal_text.as_str())
}

#[memo]
pub fn extract_iso_literals<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<IsoLiteralExtraction<THostLanguage>>> {
    let contents = file_contents(db, client, path).as_ref()?;
    THostLanguage::extract_iso_literals_from_text(contents).wrap_some()
}

#[memo]
pub fn parsed_iso_literals_in_file<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    client: ClientId,
    path: RelativePathToSourceFile,
) -> Option<Vec<(IsoLiteralExtraction<THostLanguage>, ParsedIsoLiteral)>> {
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

#[memo]
pub fn text_through_last_iso_literal<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
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
pub fn iso_literal_extraction<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<IsoLiteralExtraction<THostLanguage>> {
    let extractions = extract_iso_literals(db, literal_id.client, literal_id.path).as_ref()?;
    extractions.get(literal_id.index).cloned()
}

pub fn parsed_iso_literal_by_id<THostLanguage: HostLanguage>(
    db: &IsographDb<THostLanguage>,
    literal_id: LiteralId,
) -> Option<&ParsedIsoLiteral> {
    let extraction = iso_literal_extraction(db, literal_id).as_ref()?;
    parsed_iso_literal(db, extraction.iso_literal_text.clone()).wrap_some()
}
