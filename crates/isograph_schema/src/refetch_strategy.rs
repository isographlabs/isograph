use std::{collections::BTreeSet, fmt::Debug};

use common_lang_types::{
    ServerObjectEntityName, VariableName, WithLocation, WithSpan, WithSpanPostfix,
};
use isograph_lang_types::{
    EmptyDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet, SelectionSet,
    SelectionTypeContainingSelections,
};

use crate::{
    IsographDatabase, MergedSelectionMap, NetworkProtocol, UnprocessedSelection,
    WrappedSelectionMapSelection, get_reachable_variables, selection_map_wrapped,
};

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum RefetchStrategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
> {
    UseRefetchField(
        UseRefetchFieldRefetchStrategy<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    ),
    RefetchFromRoot,
}

impl<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
>
    RefetchStrategy<
        TSelectionTypeSelectionScalarFieldAssociatedData,
        TSelectionTypeSelectionLinkedFieldAssociatedData,
    >
{
    pub fn refetch_selection_set(
        &self,
    ) -> Option<
        &WithSpan<
            SelectionSet<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    > {
        match self {
            RefetchStrategy::UseRefetchField(used_refetch_field) => {
                Some(&used_refetch_field.refetch_selection_set)
            }
            RefetchStrategy::RefetchFromRoot => None,
        }
    }
}
pub fn generate_refetch_field_strategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
>(
    refetch_selection_set: WithSpan<
        SelectionSet<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >,
    root_fetchable_type_name: ServerObjectEntityName,
    subfields: Vec<WrappedSelectionMapSelection>,
) -> UseRefetchFieldRefetchStrategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
> {
    UseRefetchFieldRefetchStrategy {
        refetch_selection_set,
        root_fetchable_type_name,
        generate_refetch_query: GenerateRefetchQueryImpl { subfields },
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct UseRefetchFieldRefetchStrategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
> {
    /// If this field is fetched imperatively, what fields do we need to
    /// select in the parent query?
    pub refetch_selection_set: WithSpan<
        SelectionSet<
            TSelectionTypeSelectionScalarFieldAssociatedData,
            TSelectionTypeSelectionLinkedFieldAssociatedData,
        >,
    >,
    /// Query, Mutation, etc.
    pub root_fetchable_type_name: ServerObjectEntityName,

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

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GenerateRefetchQueryImpl {
    pub subfields: Vec<WrappedSelectionMapSelection>,
}

impl GenerateRefetchQueryImpl {
    pub fn generate_refetch_query(
        &self,
        inner_selection_map: MergedSelectionMap,
    ) -> (MergedSelectionMap, BTreeSet<VariableName>) {
        let new_selection_map = selection_map_wrapped(inner_selection_map, self.subfields.clone());

        // TODO this seems like a bunch of extra work, and we shouldn't need to do it
        let variables = get_reachable_variables(&new_selection_map).collect();

        (new_selection_map, variables)
    }
}

pub fn id_selection<TNetworkProtocol: NetworkProtocol>(
    db: &IsographDatabase<TNetworkProtocol>,
    entity_name: ServerObjectEntityName,
) -> UnprocessedSelection {
    SelectionTypeContainingSelections::Scalar(ScalarSelection {
        name: WithLocation::new_generated(
            TNetworkProtocol::get_id_field_name(db, &entity_name).unchecked_conversion(),
        ),
        reader_alias: None,
        scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        associated_data: (),
        arguments: vec![],
    })
    .with_generated_span()
}
