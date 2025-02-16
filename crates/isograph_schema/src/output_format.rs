use std::{fmt::Debug, hash::Hash};

use common_lang_types::{QueryOperationName, QueryText};

use crate::{MergedSelectionMap, RootOperationName, ValidatedSchema, ValidatedVariableDefinition};

pub trait OutputFormat:
    Debug + Clone + Copy + Eq + PartialEq + Ord + PartialOrd + Hash + Default
where
    Self: Sized,
{
    // These two types should be combined into a single type, this is a
    // GraphQL-ism leaking in
    type TypeSystemDocument: Debug;
    type TypeSystemExtensionDocument: Debug;

    fn generate_query_text<'a>(
        query_name: QueryOperationName,
        schema: &ValidatedSchema<Self>,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
    ) -> QueryText;
}
