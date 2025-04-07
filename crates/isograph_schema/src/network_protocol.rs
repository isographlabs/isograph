use std::{collections::BTreeMap, error::Error, fmt::Debug, hash::Hash};

use common_lang_types::{QueryOperationName, QueryText, RelativePathToSourceFile};
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::SchemaSource;
use pico::{Database, MemoRef, SourceId};

use crate::{
    EncounteredRootTypes, MergedSelectionMap, RootOperationName, Schema, TypeRefinementMaps,
    ValidatedVariableDefinition,
};

pub trait NetworkProtocol:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // These two types should be combined into a single type, this is a
    // GraphQL-ism leaking in
    type TypeSystemDocument: Debug + Clone + 'static;
    type TypeSystemExtensionDocument: Debug + Clone + 'static;

    type SchemaObjectAssociatedData: Debug;

    #[allow(clippy::type_complexity)]
    fn parse_type_system_documents(
        db: &Database,
        schema_source_id: SourceId<SchemaSource>,
        schema_extension_sources: &BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
    ) -> Result<
        (
            MemoRef<Self::TypeSystemDocument>,
            BTreeMap<RelativePathToSourceFile, MemoRef<Self::TypeSystemExtensionDocument>>,
        ),
        Box<dyn Error>,
    >;

    // TODO refactor this to return a Vec or Iterator of IsographObjectDefinition or the like,
    // instead of mutating the Schema
    fn process_type_system_documents(
        schema: &mut Schema<Self>,
        type_system_document: Self::TypeSystemDocument,
        type_system_extension_document: BTreeMap<
            RelativePathToSourceFile,
            MemoRef<Self::TypeSystemExtensionDocument>,
        >,
        options: &CompilerConfigOptions,
    ) -> Result<ProcessTypeSystemDocumentOutcome, Box<dyn Error>>;

    fn generate_query_text<'a>(
        query_name: QueryOperationName,
        schema: &Schema<Self>,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
    ) -> QueryText;
}

pub struct ProcessTypeSystemDocumentOutcome {
    pub type_refinement_maps: TypeRefinementMaps,
    pub root_types: EncounteredRootTypes,
}
