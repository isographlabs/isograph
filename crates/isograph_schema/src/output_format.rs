use std::{error::Error, fmt::Debug, hash::Hash};

use common_lang_types::{QueryOperationName, QueryText};
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::SchemaSource;
use pico::{Database, MemoRef, SourceId};

use crate::{
    EncounteredRootTypes, MergedSelectionMap, RootOperationName, TypeRefinementMaps,
    UnvalidatedSchema, ValidatedSchema, ValidatedVariableDefinition,
};

pub trait OutputFormat:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // These two types should be combined into a single type, this is a
    // GraphQL-ism leaking in
    type TypeSystemDocument: Debug + Clone + 'static;
    type TypeSystemExtensionDocument: Debug + Clone + 'static;

    type SchemaObjectAssociatedData: Debug;

    fn parse_type_system_document(
        db: &Database,
        schema_source_id: SourceId<SchemaSource>,
    ) -> Result<MemoRef<Self::TypeSystemDocument>, Box<dyn Error>>;

    fn parse_type_system_extension_document(
        db: &Database,
        schema_extension_source_id: SourceId<SchemaSource>,
    ) -> Result<MemoRef<Self::TypeSystemExtensionDocument>, Box<dyn Error>>;

    // TODO refactor this to return a Vec or Iterator of IsographObjectDefinition or the like,
    // instead of mutating the UnvalidatedSchema
    fn process_type_system_document(
        schema: &mut UnvalidatedSchema<Self>,
        type_system_document: Self::TypeSystemDocument,
        options: &CompilerConfigOptions,
    ) -> Result<ProcessTypeSystemDocumentOutcome, Box<dyn Error>>;
    fn process_type_system_extension_document(
        schema: &mut UnvalidatedSchema<Self>,
        type_system_extension_document: Self::TypeSystemExtensionDocument,
        options: &CompilerConfigOptions,
    ) -> Result<ProcessTypeSystemDocumentOutcome, Box<dyn Error>>;

    fn generate_query_text<'a>(
        query_name: QueryOperationName,
        schema: &ValidatedSchema<Self>,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
    ) -> QueryText;
}

pub struct ProcessTypeSystemDocumentOutcome {
    pub type_refinement_maps: TypeRefinementMaps,
    pub root_types: EncounteredRootTypes,
}
