use std::{
    collections::{BTreeMap, HashMap},
    error::Error,
    ops::{Deref, DerefMut},
};

use common_lang_types::{
    CurrentWorkingDirectory, RelativePathToSourceFile, TextSource, WithLocation,
};
use isograph_config::CompilerConfig;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{IsoLiteralsSource, SchemaSource};
use isograph_schema::{validate_entrypoints, NetworkProtocol, Schema, UnprocessedItem};
use pico::{Database, SourceId};

use crate::{
    add_selection_sets::add_selection_sets_to_client_selectables,
    batch_compile::BatchCompileError,
    isograph_literals::{parse_iso_literal_in_source, process_iso_literals},
    refetch_fields::add_refetch_fields_to_objects,
    source_files::SourceFiles,
};

pub fn create_unvalidated_schema<TNetworkProtocol: NetworkProtocol>(
    db: &Database,
    source_files: &SourceFiles,
    config: &CompilerConfig,
) -> Result<(Schema<TNetworkProtocol>, ContainsIsoStats), Box<dyn Error>> {
    let mut unvalidated_isograph_schema = Schema::<TNetworkProtocol>::new();
    let type_system_document =
        TNetworkProtocol::parse_type_system_document(db, source_files.schema)?.to_owned();
    let outcome = TNetworkProtocol::process_type_system_document(
        &mut unvalidated_isograph_schema,
        type_system_document,
        &config.options,
    )?;
    let schema_extensions =
        parse_schema_extensions::<TNetworkProtocol>(db, &source_files.schema_extensions)?;
    for extension_document in schema_extensions.into_values() {
        TNetworkProtocol::process_type_system_extension_document(
            &mut unvalidated_isograph_schema,
            extension_document,
            &config.options,
        )?;
    }
    let contains_iso = parse_iso_literals(
        db,
        &source_files.iso_literals,
        config.current_working_directory,
    )?;
    let contains_iso_stats = contains_iso.stats();

    // Step one: we can create client selectables. However, we must create all
    // client selectables before being able to create their selection sets, because
    // selection sets refer to client selectables. We hold onto these selection sets
    // (both reader selection sets and refetch selection sets) in the unprocess_items
    // vec, then process it later.
    let mut unprocessed_items = vec![];

    let (unprocessed_client_types, unprocessed_entrypoints) =
        process_iso_literals(&mut unvalidated_isograph_schema, contains_iso)?;
    unprocessed_items.extend(unprocessed_client_types);

    unprocessed_items.extend(process_exposed_fields(&mut unvalidated_isograph_schema)?);

    unvalidated_isograph_schema.transfer_supertype_client_selectables_to_subtypes(
        &outcome.type_refinement_maps.supertype_to_subtype_map,
    )?;
    unvalidated_isograph_schema.add_link_fields()?;
    unvalidated_isograph_schema.add_object_selectable_to_subtype_on_supertypes(
        &outcome.type_refinement_maps.subtype_to_supertype_map,
    )?;

    unprocessed_items.extend(add_refetch_fields_to_objects(
        &mut unvalidated_isograph_schema,
    )?);

    unvalidated_isograph_schema.entrypoints = validate_entrypoints(
        &unvalidated_isograph_schema,
        unprocessed_entrypoints,
    )
    .map_err(|e| BatchCompileError::MultipleErrorsWithLocations {
        messages: e
            .into_iter()
            .map(|x| WithLocation::new(Box::new(x.item) as Box<dyn std::error::Error>, x.location))
            .collect(),
    })?;

    // Step two: now, we can create the selection sets. Creating a selection set involves
    // looking up client selectables, to:
    // - determine if the selectable exists,
    // - to determine if we are selecting it appropriately (e.g. client fields as scalars, etc)
    // - to validate arguments (e.g. no missing arguments, etc.)
    // - validate loadability/updatability, and
    // - to store the selectable id,
    add_selection_sets_to_client_selectables(&mut unvalidated_isograph_schema, unprocessed_items)
        .map_err(|messages| BatchCompileError::MultipleErrorsWithLocations {
        messages: messages
            .into_iter()
            .map(|x| WithLocation::new(Box::new(x.item) as Box<dyn std::error::Error>, x.location))
            .collect(),
    })?;

    Ok((unvalidated_isograph_schema, contains_iso_stats))
}

fn parse_schema_extensions<TNetworkProtocol: NetworkProtocol>(
    db: &Database,
    schema_extensions_sources: &BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
) -> Result<
    HashMap<RelativePathToSourceFile, TNetworkProtocol::TypeSystemExtensionDocument>,
    Box<dyn Error>,
> {
    let mut schema_extensions = HashMap::new();
    for (relative_path, schema_extension_source_id) in schema_extensions_sources.iter() {
        let extensions_document = TNetworkProtocol::parse_type_system_extension_document(
            db,
            *schema_extension_source_id,
        )?
        .to_owned();
        schema_extensions.insert(*relative_path, extensions_document);
    }
    Ok(schema_extensions)
}

fn parse_iso_literals(
    db: &Database,
    iso_literals_sources: &HashMap<RelativePathToSourceFile, SourceId<IsoLiteralsSource>>,
    current_working_directory: CurrentWorkingDirectory,
) -> Result<ContainsIso, BatchCompileError> {
    let mut contains_iso = ContainsIso::default();
    let mut iso_literal_parse_errors = vec![];
    for (relative_path, iso_literals_source_id) in iso_literals_sources.iter() {
        match parse_iso_literal_in_source(db, *iso_literals_source_id, current_working_directory)
            .to_owned()
        {
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
        Err(iso_literal_parse_errors.into())
    }
}

/// Here, we are processing exposeAs fields. Note that we only process these
/// directives on root objects (Query, Mutation, Subscription) and we should
/// validate that no other types have exposeAs directives.
fn process_exposed_fields<TNetworkProtocol: NetworkProtocol>(
    schema: &mut Schema<TNetworkProtocol>,
) -> Result<Vec<UnprocessedItem>, BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    let mut unprocessed_items = vec![];
    for fetchable_object_entity_id in fetchable_types.into_iter() {
        let unprocessed_client_field_item =
            schema.add_exposed_fields_to_parent_object_types(fetchable_object_entity_id)?;
        unprocessed_items.extend(
            unprocessed_client_field_item
                .into_iter()
                .map(UnprocessedItem::Scalar),
        );
    }
    Ok(unprocessed_items)
}

#[derive(Clone, Debug, Eq, PartialEq, Default)]
pub struct ContainsIso {
    pub files: HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>,
}

impl ContainsIso {
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

impl Deref for ContainsIso {
    type Target = HashMap<RelativePathToSourceFile, Vec<(IsoLiteralExtractionResult, TextSource)>>;

    fn deref(&self) -> &Self::Target {
        &self.files
    }
}

impl DerefMut for ContainsIso {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.files
    }
}

pub struct ContainsIsoStats {
    pub client_field_count: usize,
    pub entrypoint_count: usize,
    #[allow(unused)]
    pub client_pointer_count: usize,
}
