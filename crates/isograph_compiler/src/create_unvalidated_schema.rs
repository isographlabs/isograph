use std::{
    collections::HashMap,
    error::Error,
    ops::{Deref, DerefMut},
};

use common_lang_types::{CurrentWorkingDirectory, RelativePathToSourceFile, TextSource};
use isograph_config::CompilerConfig;
use isograph_lang_parser::IsoLiteralExtractionResult;
use isograph_lang_types::{IsoLiteralsSource, SchemaSource};
use isograph_schema::{OutputFormat, UnvalidatedSchema};
use pico::{Database, SourceId};

use crate::{
    batch_compile::BatchCompileError,
    isograph_literals::{parse_iso_literal_in_source, process_iso_literals},
    refetch_fields::add_refetch_fields_to_objects,
    source_files::SourceFiles,
};

pub fn create_unvalidated_schema<TOutputFormat: OutputFormat>(
    db: &Database,
    source_files: &SourceFiles,
    config: &CompilerConfig,
) -> Result<(UnvalidatedSchema<TOutputFormat>, ContainsIsoStats), Box<dyn Error>> {
    let mut unvalidated_isograph_schema = UnvalidatedSchema::<TOutputFormat>::new();
    let schema = TOutputFormat::parse_type_system_document(db, source_files.schema)?.to_owned();
    let outcome = TOutputFormat::process_type_system_document(
        &mut unvalidated_isograph_schema,
        schema,
        &config.options,
    )?;
    let schema_extensions =
        parse_schema_extensions::<TOutputFormat>(db, &source_files.schema_extensions)?;
    for extension_document in schema_extensions.into_values() {
        TOutputFormat::process_type_system_extension_document(
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
    process_iso_literals(&mut unvalidated_isograph_schema, contains_iso)?;
    process_exposed_fields(&mut unvalidated_isograph_schema)?;
    unvalidated_isograph_schema
        .add_fields_to_subtypes(&outcome.type_refinement_maps.supertype_to_subtype_map)?;
    unvalidated_isograph_schema.add_link_fields()?;
    unvalidated_isograph_schema
        .add_pointers_to_supertypes(&outcome.type_refinement_maps.subtype_to_supertype_map)?;
    add_refetch_fields_to_objects(&mut unvalidated_isograph_schema)?;
    Ok((unvalidated_isograph_schema, contains_iso_stats))
}

fn parse_schema_extensions<TOutputFormat: OutputFormat>(
    db: &Database,
    schema_extensions_sources: &HashMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
) -> Result<
    HashMap<RelativePathToSourceFile, TOutputFormat::TypeSystemExtensionDocument>,
    Box<dyn Error>,
> {
    let mut schema_extensions = HashMap::new();
    for (relative_path, schema_extension_source_id) in schema_extensions_sources.iter() {
        let extensions_document =
            TOutputFormat::parse_type_system_extension_document(db, *schema_extension_source_id)?
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
            .deref()
        {
            Ok(iso_literals) => {
                if !iso_literals.is_empty() {
                    contains_iso.insert(*relative_path, iso_literals.to_owned());
                }
            }
            Err(e) => {
                iso_literal_parse_errors.extend(e.to_owned());
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
fn process_exposed_fields<TOutputFormat: OutputFormat>(
    schema: &mut UnvalidatedSchema<TOutputFormat>,
) -> Result<(), BatchCompileError> {
    let fetchable_types: Vec<_> = schema.fetchable_types.keys().copied().collect();
    for fetchable_object_id in fetchable_types.into_iter() {
        schema.add_exposed_fields_to_parent_object_types(fetchable_object_id)?;
    }
    Ok(())
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
