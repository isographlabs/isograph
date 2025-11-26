use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{IsographDatabase, NetworkProtocol};
use common_lang_types::{DiagnosticVecResult, RelativePathToSourceFile, TextSource};
use isograph_lang_parser::IsoLiteralExtractionResult;
use pico_macros::memo;
use prelude::Postfix;

use crate::parse_iso_literal_in_source;

// This should not be used. It returns an Err variant if there is a single parse error.
#[memo]
pub fn parse_iso_literals<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> DiagnosticVecResult<ParsedIsoLiteralsMap> {
    // TODO we are not checking the open file map here. This will probably be fixed when we
    // fully rewrite everything to be incremental.
    let mut contains_iso = ParsedIsoLiteralsMap::default();
    let mut iso_literal_parse_errors = vec![];
    for (relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        for literal in parse_iso_literal_in_source(db, *iso_literals_source_id)
            .to_owned()
            .note_todo("Do not clone. Use a MemoRef.")
        {
            match literal {
                Ok(iso_literal) => {
                    contains_iso
                        .entry(*relative_path)
                        .or_default()
                        .push(iso_literal);
                }
                Err(e) => {
                    iso_literal_parse_errors.push(e);
                }
            }
        }
    }
    if iso_literal_parse_errors.is_empty() {
        Ok(contains_iso)
    } else {
        Err(iso_literal_parse_errors)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ParsedIsoLiteralsMap {
    pub files: HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>,
}

impl ParsedIsoLiteralsMap {
    pub fn stats(&self) -> ContainsIsoStats {
        let mut client_field_count: usize = 0;
        let mut client_pointer_count: usize = 0;
        let mut entrypoint_count: usize = 0;
        for iso_literals in self.values() {
            for (iso_literal, ..) in iso_literals {
                match iso_literal {
                    IsoLiteralExtractionResult::ClientFieldDeclaration(_) => {
                        client_field_count += 1
                    }
                    IsoLiteralExtractionResult::EntrypointDeclaration(_) => entrypoint_count += 1,
                    IsoLiteralExtractionResult::ClientPointerDeclaration(_) => {
                        client_pointer_count += 1
                    }
                }
            }
        }
        ContainsIsoStats {
            client_field_count,
            entrypoint_count,
            client_pointer_count,
        }
    }
}

impl Deref for ParsedIsoLiteralsMap {
    type Target = HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for ParsedIsoLiteralsMap {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct ContainsIsoStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    pub client_pointer_count: usize,
}
