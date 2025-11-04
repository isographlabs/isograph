use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use common_lang_types::{
    RelativePathToSourceFile, SelectableName, ServerObjectEntityName, TextSource, WithLocation,
};
use isograph_lang_parser::{IsoLiteralExtractionResult, IsographLiteralParseError};
use isograph_schema::{
    IsographDatabase, NetworkProtocol, ProcessClientFieldDeclarationError, Schema,
    UnprocessedSelectionSet, ValidateEntrypointDeclarationError, validate_entrypoints,
};
use pico_macros::legacy_memo;
use thiserror::Error;

use crate::{
    add_selection_sets::{AddSelectionSetsError, add_selection_sets_to_client_selectables},
    parse_iso_literal_in_source, process_iso_literals,
};

pub(crate) fn process_iso_literals_for_schema<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
    mut unvalidated_isograph_schema: Schema<TNetworkProtocol>,
    mut unprocessed_selection_sets: Vec<UnprocessedSelectionSet>,
) -> Result<
    (Schema<TNetworkProtocol>, ContainsIsoStats),
    ProcessIsoLiteralsForSchemaError<TNetworkProtocol>,
> {
    let contains_iso = parse_iso_literals(db).to_owned()?;
    let contains_iso_stats = contains_iso.stats();

    let (unprocessed_client_types, unprocessed_entrypoints) =
        process_iso_literals(db, &mut unvalidated_isograph_schema, contains_iso)?;
    unprocessed_selection_sets.extend(unprocessed_client_types);

    unvalidated_isograph_schema.entrypoints =
        validate_entrypoints(db, &unvalidated_isograph_schema, unprocessed_entrypoints)?;

    // Step two: now, we can create the selection sets. Creating a selection set involves
    // looking up client selectables, to:
    // - determine if the selectable exists,
    // - to determine if we are selecting it appropriately (e.g. client fields as scalars, etc)
    // - to validate arguments (e.g. no missing arguments, etc.)
    // - validate loadability/updatability, and
    // - to store the selectable id,
    add_selection_sets_to_client_selectables(
        db,
        &mut unvalidated_isograph_schema,
        unprocessed_selection_sets,
    )?;

    Ok((unvalidated_isograph_schema, contains_iso_stats))
}

#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ProcessIsoLiteralsForSchemaError<TNetworkProtocol: NetworkProtocol + 'static> {
    #[error(
        "{}{}",
        if messages.len() == 1 { "Unable to process Isograph literal:" } else { "Unable to process Isograph literals:" },
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x.for_display()));
            output
        })
    )]
    ProcessIsoLiterals {
        messages: Vec<WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>>,
    },

    #[error(
        "The Isograph compiler attempted to create a field named \
        `{selectable_name}` on type `{parent_object_entity_name}`, but a field with that name already exists."
    )]
    CompilerCreatedFieldExistsOnType {
        selectable_name: SelectableName,
        parent_object_entity_name: ServerObjectEntityName,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x.for_display()));
            output
        })
    )]
    AddSelectionSets {
        messages: Vec<WithLocation<AddSelectionSetsError>>,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x.for_display()));
            output
        })
    )]
    ParseIsoLiteral {
        messages: Vec<WithLocation<IsographLiteralParseError>>,
    },

    #[error(
        "{}",
        messages.iter().fold(String::new(), |mut output, x| {
            output.push_str(&format!("\n\n{}", x.for_display()));
            output
        })
    )]
    ValidateEntrypointDeclaration {
        messages: Vec<WithLocation<ValidateEntrypointDeclarationError>>,
    },
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<Vec<WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>>>
    for ProcessIsoLiteralsForSchemaError<TNetworkProtocol>
{
    fn from(
        messages: Vec<WithLocation<ProcessClientFieldDeclarationError<TNetworkProtocol>>>,
    ) -> Self {
        ProcessIsoLiteralsForSchemaError::ProcessIsoLiterals { messages }
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> From<Vec<WithLocation<IsographLiteralParseError>>>
    for ProcessIsoLiteralsForSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<IsographLiteralParseError>>) -> Self {
        ProcessIsoLiteralsForSchemaError::ParseIsoLiteral { messages }
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static>
    From<Vec<WithLocation<ValidateEntrypointDeclarationError>>>
    for ProcessIsoLiteralsForSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<ValidateEntrypointDeclarationError>>) -> Self {
        ProcessIsoLiteralsForSchemaError::ValidateEntrypointDeclaration { messages }
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> From<Vec<WithLocation<AddSelectionSetsError>>>
    for ProcessIsoLiteralsForSchemaError<TNetworkProtocol>
{
    fn from(messages: Vec<WithLocation<AddSelectionSetsError>>) -> Self {
        ProcessIsoLiteralsForSchemaError::AddSelectionSets { messages }
    }
}

#[legacy_memo]
fn parse_iso_literals<TNetworkProtocol: NetworkProtocol + 'static>(
    db: &IsographDatabase<TNetworkProtocol>,
) -> Result<ParsedIsoLiteralsMap, Vec<WithLocation<IsographLiteralParseError>>> {
    // TODO we are not checking the open file map here. This will probably be fixed when we
    // fully rewrite everything to be incremental.
    let mut contains_iso = ParsedIsoLiteralsMap::default();
    let mut iso_literal_parse_errors = vec![];
    for (relative_path, iso_literals_source_id) in db.get_iso_literal_map().tracked().0.iter() {
        match parse_iso_literal_in_source(db, *iso_literals_source_id).to_owned() {
            Ok(iso_literals) => {
                if !iso_literals.is_empty() {
                    contains_iso.insert(*relative_path, iso_literals);
                }
            }
            Err(e) => {
                iso_literal_parse_errors.extend(e);
            }
        };
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
