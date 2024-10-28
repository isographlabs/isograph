use std::{collections::BTreeSet, fmt::Debug};

use common_lang_types::{
    IsographObjectTypeName, LinkedFieldName, Location, QueryOperationName, Span, VariableName,
    WithLocation, WithSpan,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, IsographSelectionVariant, ScalarFieldSelection, Selection,
    ServerFieldSelection, ServerObjectId,
};

use crate::{
    get_reachable_variables, selection_map_wrapped, MergedSelectionMap, RequiresRefinement,
};

#[derive(Debug)]
pub enum RefetchStrategy<
    TClientFieldSelectionScalarFieldAssociatedData,
    TClientFieldSelectionLinkedFieldAssociatedData,
> {
    UseRefetchField(
        UseRefetchFieldRefetchStrategy<
            TClientFieldSelectionScalarFieldAssociatedData,
            TClientFieldSelectionLinkedFieldAssociatedData,
        >,
    ),
    // RefetchFromRoot
}

impl<
        TClientFieldSelectionScalarFieldAssociatedData,
        TClientFieldSelectionLinkedFieldAssociatedData,
    >
    RefetchStrategy<
        TClientFieldSelectionScalarFieldAssociatedData,
        TClientFieldSelectionLinkedFieldAssociatedData,
    >
{
    pub fn refetch_selection_set(
        &self,
    ) -> &Vec<
        WithSpan<
            Selection<
                TClientFieldSelectionScalarFieldAssociatedData,
                TClientFieldSelectionLinkedFieldAssociatedData,
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

pub fn generate_refetch_field_strategy<
    TClientFieldSelectionScalarFieldAssociatedData,
    TClientFieldSelectionLinkedFieldAssociatedData,
>(
    refetch_selection_set: Vec<
        WithSpan<
            Selection<
                TClientFieldSelectionScalarFieldAssociatedData,
                TClientFieldSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,
    root_fetchable_type: ServerObjectId,
    refetch_query_name: QueryOperationName,
    top_level_field_name: LinkedFieldName,
    top_level_arguments: Vec<ArgumentKeyAndValue>,
    top_level_field_concrete_type: Option<IsographObjectTypeName>,
    refine_to_type: RequiresRefinement,
    subfield: Option<LinkedFieldName>,
    subfield_concrete_type: Option<IsographObjectTypeName>,
) -> UseRefetchFieldRefetchStrategy<
    TClientFieldSelectionScalarFieldAssociatedData,
    TClientFieldSelectionLinkedFieldAssociatedData,
> {
    UseRefetchFieldRefetchStrategy {
        refetch_selection_set,
        root_fetchable_type,
        generate_refetch_query: Box::new(GenerateRefetchQueryImpl {
            top_level_field_name,
            top_level_arguments,
            top_level_field_concrete_type,
            refine_to_type,
            subfield,
            subfield_concrete_type,
        }),
        refetch_query_name,
    }
}

#[derive(Debug)]
pub struct UseRefetchFieldRefetchStrategy<
    TClientFieldSelectionScalarFieldAssociatedData,
    TClientFieldSelectionLinkedFieldAssociatedData,
> {
    /// If this field is fetched imperatively, what fields do we need to
    /// select in the parent query?
    pub refetch_selection_set: Vec<
        WithSpan<
            Selection<
                TClientFieldSelectionScalarFieldAssociatedData,
                TClientFieldSelectionLinkedFieldAssociatedData,
            >,
        >,
    >,
    /// Query, Mutation, etc.
    pub root_fetchable_type: ServerObjectId,

    /// Given the content one needs to refetch (which can be empty?), generate
    /// the merged selection map and variables representing the entire query.
    ///
    /// A root_fetchable_type + a query name + variables + a MergedSelectionMap
    /// is enough to generate the query text, for example.
    pub generate_refetch_query: Box<dyn GenerateRefetchQueryFn>,
    pub refetch_query_name: QueryOperationName,
}

pub trait GenerateRefetchQueryFn: Debug {
    fn generate_refetch_query(
        &self,
        inner: MergedSelectionMap,
    ) -> (MergedSelectionMap, BTreeSet<VariableName>);

    fn clone_box(&self) -> Box<dyn GenerateRefetchQueryFn>;
}

#[derive(Debug)]
struct GenerateRefetchQueryImpl {
    top_level_field_name: LinkedFieldName,
    top_level_arguments: Vec<ArgumentKeyAndValue>,
    /// Some if the object is concrete; None otherwise.
    top_level_field_concrete_type: Option<IsographObjectTypeName>,
    refine_to_type: RequiresRefinement,
    subfield: Option<LinkedFieldName>,
    /// Some if the object is concrete; None otherwise.
    subfield_concrete_type: Option<IsographObjectTypeName>,
}

impl GenerateRefetchQueryFn for GenerateRefetchQueryImpl {
    fn generate_refetch_query(
        &self,
        inner_selection_map: MergedSelectionMap,
    ) -> (MergedSelectionMap, BTreeSet<VariableName>) {
        let new_selection_map = selection_map_wrapped(
            inner_selection_map,
            self.top_level_field_name,
            // TODO consume and don't clone?
            self.top_level_arguments.clone(),
            self.top_level_field_concrete_type,
            self.subfield,
            self.subfield_concrete_type,
            self.refine_to_type,
        );

        // TODO this seems like a bunch of extra work, and we shouldn't need to do it
        let variables = get_reachable_variables(&new_selection_map);

        (new_selection_map, variables)
    }

    fn clone_box(&self) -> Box<dyn GenerateRefetchQueryFn> {
        Box::new(Self {
            top_level_field_name: self.top_level_field_name,
            top_level_arguments: self.top_level_arguments.clone(),
            top_level_field_concrete_type: self.top_level_field_concrete_type,
            refine_to_type: self.refine_to_type,
            subfield: self.subfield,
            subfield_concrete_type: self.subfield_concrete_type,
        })
    }
}

pub fn id_selection() -> WithSpan<Selection<IsographSelectionVariant, IsographSelectionVariant>> {
    WithSpan::new(
        Selection::ServerField(ServerFieldSelection::ScalarField(ScalarFieldSelection {
            name: WithLocation::new("id".intern().into(), Location::generated()),
            reader_alias: None,
            associated_data: IsographSelectionVariant::Regular,
            unwraps: vec![],
            arguments: vec![],
            directives: vec![],
        })),
        Span::todo_generated(),
    )
}
