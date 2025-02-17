use std::{error::Error, fmt::Debug, hash::Hash};

use common_lang_types::{QueryOperationName, QueryText, RelativePathToSourceFile};
use isograph_config::{AbsolutePathAndRelativePath, CompilerConfig, CompilerConfigOptions};

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
    type TypeSystemDocument: Debug + Clone;
    type TypeSystemExtensionDocument: Debug + Clone;

    fn read_and_parse_type_system_document(
        config: &CompilerConfig,
    ) -> Result<Self::TypeSystemDocument, Box<dyn Error>>;
    fn read_and_parse_type_system_extension_document(
        schema_extension_path: &AbsolutePathAndRelativePath,
        config: &CompilerConfig,
    ) -> Result<(RelativePathToSourceFile, Self::TypeSystemExtensionDocument), Box<dyn Error>>;

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
