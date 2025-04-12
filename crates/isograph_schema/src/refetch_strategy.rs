use std::{collections::BTreeSet, fmt::Debug};

use common_lang_types::{Location, Span, VariableName, WithLocation, WithSpan};
use intern::string_key::Intern;
use isograph_lang_types::{
    EmptyDirectiveSet, ObjectSelectionDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionTypeContainingSelections, ServerObjectEntityId,
};

use crate::{
    get_reachable_variables, selection_map_wrapped, MergedSelectionMap,
    WrappedSelectionMapSelection,
};

#[derive(Debug)]
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
    // RefetchFromRoot
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
    ) -> &Vec<
        WithSpan<
            SelectionTypeContainingSelections<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    > {
        match self {
            RefetchStrategy::UseRefetchField(used_refetch_field) => {
                &used_refetch_field.refetch_selection_set
            }
        }
    }
}
#[allow(clippy::too_many_arguments)]
pub fn generate_refetch_field_strategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
>(
    refetch_selection_set: Vec<
        WithSpan<
            SelectionTypeContainingSelections<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,
    root_fetchable_type: ServerObjectEntityId,
    subfields: Vec<WrappedSelectionMapSelection>,
) -> UseRefetchFieldRefetchStrategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
> {
    UseRefetchFieldRefetchStrategy {
        refetch_selection_set,
        root_fetchable_type,
        generate_refetch_query: Box::new(GenerateRefetchQueryImpl { subfields }),
    }
}

#[derive(Debug)]
pub struct UseRefetchFieldRefetchStrategy<
    TSelectionTypeSelectionScalarFieldAssociatedData,
    TSelectionTypeSelectionLinkedFieldAssociatedData,
> {
    /// If this field is fetched imperatively, what fields do we need to
    /// select in the parent query?
    pub refetch_selection_set: Vec<
        WithSpan<
            SelectionTypeContainingSelections<
                TSelectionTypeSelectionScalarFieldAssociatedData,
                TSelectionTypeSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,
    /// Query, Mutation, etc.
    pub root_fetchable_type: ServerObjectEntityId,

    /// Given the content one needs to refetch (which can be empty?), generate
    /// the merged selection map and variables representing the entire query.
    ///
    /// A root_fetchable_type + a query name + variables + a MergedSelectionMap
    /// is enough to generate the query text, for example.
    pub generate_refetch_query: Box<dyn GenerateRefetchQueryFn>,
}

pub trait GenerateRefetchQueryFn: Debug {
    fn generate_refetch_query(
        &self,
        inner: MergedSelectionMap,
    ) -> (MergedSelectionMap, BTreeSet<VariableName>);
}

#[derive(Debug)]
struct GenerateRefetchQueryImpl {
    subfields: Vec<WrappedSelectionMapSelection>,
}

impl GenerateRefetchQueryFn for GenerateRefetchQueryImpl {
    fn generate_refetch_query(
        &self,
        inner_selection_map: MergedSelectionMap,
    ) -> (MergedSelectionMap, BTreeSet<VariableName>) {
        let new_selection_map = selection_map_wrapped(inner_selection_map, self.subfields.clone());

        // TODO this seems like a bunch of extra work, and we shouldn't need to do it
        let variables = get_reachable_variables(&new_selection_map);

        (new_selection_map, variables)
    }
}

pub fn id_selection() -> WithSpan<
    SelectionTypeContainingSelections<ScalarSelectionDirectiveSet, ObjectSelectionDirectiveSet>,
> {
    WithSpan::new(
        SelectionTypeContainingSelections::Scalar(ScalarSelection {
            name: WithLocation::new("id".intern().into(), Location::generated()),
            reader_alias: None,
            associated_data: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
            arguments: vec![],
        }),
        Span::todo_generated(),
    )
}
