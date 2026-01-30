use std::{collections::BTreeSet, fmt::Debug, hash::Hash};

use common_lang_types::{
    EmbeddedLocation, EntityName, SelectableName, WithEmbeddedLocation, WithLocationPostfix,
};
use isograph_lang_types::{
    EmptyDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet, Selection, SelectionSet,
    SelectionType, VariableNameWrapper,
};

use crate::{
    ID_FIELD_NAME, MergedSelectionMap, WrappedMergedSelectionMap, WrappedSelectionMapSelection,
    get_reachable_variables, selection_map_wrapped,
};

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum RefetchStrategy {
    UseRefetchField(UseRefetchFieldRefetchStrategy),
    RefetchFromRoot,
}

impl RefetchStrategy {
    pub fn refetch_selection_set(&self) -> Option<&WithEmbeddedLocation<SelectionSet>> {
        match self {
            RefetchStrategy::UseRefetchField(used_refetch_field) => {
                Some(&used_refetch_field.refetch_selection_set)
            }
            RefetchStrategy::RefetchFromRoot => None,
        }
    }
}
pub fn generate_refetch_field_strategy(
    refetch_selection_set: WithEmbeddedLocation<SelectionSet>,
    root_fetchable_type_name: EntityName,
    subfields: Vec<WrappedSelectionMapSelection>,
) -> UseRefetchFieldRefetchStrategy {
    UseRefetchFieldRefetchStrategy {
        refetch_selection_set,
        root_fetchable_type_name,
        generate_refetch_query: GenerateRefetchQueryImpl { subfields },
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct UseRefetchFieldRefetchStrategy {
    /// If this field is fetched imperatively, what fields do we need to
    /// select in the parent query?
    pub refetch_selection_set: WithEmbeddedLocation<SelectionSet>,
    /// Query, Mutation, etc.
    pub root_fetchable_type_name: EntityName,

    /// Given the content one needs to refetch (which can be empty?), generate
    /// the merged selection map and variables representing the entire query.
    ///
    /// A root_fetchable_type + a query name + variables + a MergedSelectionMap
    /// is enough to generate the query text, for example.
    ///
    /// N.B. TODO consider make this a Box<dyn ...>. Why did we do that in the
    /// first place?
    pub generate_refetch_query: GenerateRefetchQueryImpl,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GenerateRefetchQueryImpl {
    pub subfields: Vec<WrappedSelectionMapSelection>,
}

impl GenerateRefetchQueryImpl {
    pub fn generate_refetch_query(
        &self,
        inner_selection_map: MergedSelectionMap,
    ) -> (WrappedMergedSelectionMap, BTreeSet<VariableNameWrapper>) {
        let new_selection_map = selection_map_wrapped(inner_selection_map, self.subfields.clone());

        // TODO this seems like a bunch of extra work, and we shouldn't need to do it
        let variables = get_reachable_variables(&new_selection_map.0).collect();

        (new_selection_map, variables)
    }
}

pub fn id_selection() -> WithEmbeddedLocation<Selection> {
    SelectionType::Scalar(ScalarSelection {
        name: ID_FIELD_NAME
            .unchecked_conversion::<SelectableName>()
            .with_location(EmbeddedLocation::todo_generated()),
        reader_alias: None,
        scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        arguments: vec![],
    })
    .with_location(EmbeddedLocation::todo_generated())
}
