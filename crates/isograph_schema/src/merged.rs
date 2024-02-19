use std::collections::{
    hash_map::{Entry, OccupiedEntry, VacantEntry},
    HashMap, HashSet,
};

use common_lang_types::{
    IsographObjectTypeName, LinkedFieldAlias, LinkedFieldName, Location, ScalarFieldAlias,
    ScalarFieldName, SelectableFieldName, Span, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::GraphQLInputValueDefinition;
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    DefinedTypeId, ObjectId, ResolverFieldId, ScalarFieldSelection, Selection,
    SelectionFieldArgument, ServerFieldSelection, VariableDefinition,
};

use crate::{
    magic_mutation_fields::RequiresRefinement, ArgumentKeyAndValue, DefinedField,
    MutationFieldResolverVariant, NameAndArguments, PathToRefetchField, ResolverVariant,
    ValidatedDefinedField, ValidatedLinkedFieldSelection, ValidatedSchema, ValidatedSchemaIdField,
    ValidatedSchemaObject, ValidatedSchemaResolver, ValidatedSelection,
};

type MergedSelectionMap = HashMap<NormalizationKey, WithSpan<MergedServerFieldSelection>>;

#[derive(Debug)]
pub struct RootRefetchedPath {
    pub path: PathToRefetchField,
    pub variables: Vec<VariableName>,
    // TODO This should not be an option
    pub field_name: SelectableFieldName,
}

// TODO add id and typename variants, impl Ord, and get rid of the NormalizationKey enum
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum MergedServerFieldSelection {
    ScalarField(MergedScalarFieldSelection),
    LinkedField(MergedLinkedFieldSelection),
}

impl MergedServerFieldSelection {
    pub fn reachable_variables(&self) -> HashSet<VariableName> {
        match self {
            MergedServerFieldSelection::ScalarField(scalar_field) => {
                get_variable_selections(&scalar_field.arguments)
            }
            MergedServerFieldSelection::LinkedField(linked_field) => {
                let mut reachable_variables = get_variable_selections(&linked_field.arguments);
                for selection in linked_field.selection_set.iter() {
                    reachable_variables.extend(selection.item.reachable_variables());
                }
                reachable_variables
            }
        }
    }
}

pub fn get_variable_selections(
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> HashSet<VariableName> {
    arguments
        .iter()
        .flat_map(|argument| argument.item.value.item.reachable_variables())
        .collect()
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    pub name: WithLocation<ScalarFieldName>,
    // TODO calculate this when needed
    pub normalization_alias: Option<WithLocation<ScalarFieldAlias>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedLinkedFieldSelection {
    pub name: WithLocation<LinkedFieldName>,
    // TODO calculate this when needed
    pub normalization_alias: Option<WithLocation<LinkedFieldAlias>>,
    pub selection_set: Vec<WithSpan<MergedServerFieldSelection>>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
}

/// A merged selection set is an input for generating:
/// - query texts
/// - normalization ASTs
/// - raw response types (TODO)
///
/// For regular and refetch queries.
#[derive(Clone, Debug)]
pub struct MergedSelectionSet(Vec<WithSpan<MergedServerFieldSelection>>);

impl std::ops::Deref for MergedSelectionSet {
    type Target = Vec<WithSpan<MergedServerFieldSelection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MergedSelectionSet {
    fn new(
        mut unsorted_vec: Vec<(NormalizationKey, WithSpan<MergedServerFieldSelection>)>,
    ) -> Self {
        unsorted_vec.sort_by(|(k1, _), (k2, _)| k1.cmp(k2));
        MergedSelectionSet(unsorted_vec.into_iter().map(|(_, value)| value).collect())
    }

    fn reachable_variables(&self) -> HashSet<VariableName> {
        self.0
            .iter()
            .flat_map(|x| x.item.reachable_variables())
            .collect()
    }
}
fn find_by_path(
    mut root: &[WithSpan<MergedServerFieldSelection>],
    path: &PathToRefetchField,
) -> MergedSelectionSet {
    for item in path.linked_fields.iter() {
        let linked_field_selection = root
            .iter()
            .find_map(|linked_field_selection| {
                if let MergedServerFieldSelection::LinkedField(linked_field) =
                    &linked_field_selection.item
                {
                    let linked_field_name: SelectableFieldName = linked_field.name.item.into();
                    if linked_field_name == item.name {
                        return Some(linked_field);
                    }
                }
                None
            })
            .expect("Linked field not found. This is indicative of a bug in Isograph.");

        root = &linked_field_selection.selection_set;
    }

    // TODO is this already sorted?
    MergedSelectionSet(root.to_vec())
}

impl Into<Vec<WithSpan<MergedServerFieldSelection>>> for MergedSelectionSet {
    fn into(self) -> Vec<WithSpan<MergedServerFieldSelection>> {
        self.0
    }
}

#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord, Hash)]
enum NormalizationKey {
    // __typename,
    Id,
    ServerField(NameAndArguments),
}

#[derive(Debug)]
pub enum ArtifactQueueItem {
    RefetchField(RefetchFieldResolverInfo),
    MutationField(MutationFieldResolverInfo),
}

#[derive(Debug, Clone)]
pub struct RefetchFieldResolverInfo {
    pub merged_selection_set: MergedSelectionSet,
    /// Used to look up what type to narrow on in the generated refetch query,
    /// among other things.
    pub refetch_field_parent_id: ObjectId,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<DefinedTypeId>>>,
    pub root_parent_object: IsographObjectTypeName,
    pub root_fetchable_field: SelectableFieldName,
    // TODO wrap in a newtype
    pub refetch_query_index: usize,
}

#[derive(Debug, Clone)]
pub struct MutationFieldResolverInfo {
    pub merged_selection_set: MergedSelectionSet,
    /// Used to look up what type to narrow on in the generated refetch query,
    /// among other things.
    pub refetch_field_parent_id: ObjectId,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<DefinedTypeId>>>,
    pub root_parent_object: IsographObjectTypeName,
    pub root_fetchable_field: SelectableFieldName,
    // TODO wrap in a newtype
    pub refetch_query_index: usize,
    // TODO make MutationFieldResolverInfo and RefetchFieldResolverInfo
    // the same struct, with everything below wrapped in an option:
    // Mutation name
    pub mutation_field_name: SelectableFieldName,
    pub mutation_primary_field_name: SelectableFieldName,
    pub mutation_field_arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    pub requires_refinement: RequiresRefinement,
}

/// This struct contains everything that is available when we start
/// generating a merged selection set for a given fetchable resolver root.
/// A mutable reference to this struct is passed down to all children.
#[allow(unused)]
#[derive(Debug)]
struct MergeTraversalState<'a> {
    resolver: &'a ValidatedSchemaResolver,
    paths_to_refetch_fields: Vec<(PathToRefetchField, ObjectId, ResolverVariant)>,
    /// As we traverse selection sets, we need to keep track of the path we have
    /// taken so far. This is because when we encounter a refetch query, we need
    /// to take note of the path we took to reach that query, but continue
    /// generating the merged selection set.
    ///
    /// Finally, once we have completed generating the merged selection set,
    /// we re-traverse the paths to get the complete merged selection sets
    /// needed for each refetch query. At this point, we have enough information
    /// to generate the refetch query.
    current_path: PathToRefetchField,
    encountered_resolver_ids: Option<&'a mut HashSet<ResolverFieldId>>,
}

impl<'a> MergeTraversalState<'a> {
    pub fn new(
        resolver: &'a ValidatedSchemaResolver,
        encountered_resolver_ids: Option<&'a mut HashSet<ResolverFieldId>>,
    ) -> Self {
        Self {
            resolver,
            paths_to_refetch_fields: Default::default(),
            current_path: Default::default(),
            encountered_resolver_ids,
        }
    }
}

pub fn create_merged_selection_set(
    schema: &ValidatedSchema,
    parent_type: &ValidatedSchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    // TODO consider ways to get rid of these parameters.
    artifact_queue: Option<&mut Vec<ArtifactQueueItem>>,
    encountered_resolver_ids: Option<&mut HashSet<ResolverFieldId>>,
    // N.B. we call this for non-fetchable resolvers now, but that is a smell
    root_fetchable_resolver: &ValidatedSchemaResolver,
) -> (MergedSelectionSet, Vec<RootRefetchedPath>) {
    let mut merge_traversal_state =
        MergeTraversalState::new(root_fetchable_resolver, encountered_resolver_ids);
    let merged_selection_set = create_merged_selection_set_with_merge_traversal_state(
        schema,
        parent_type,
        validated_selections,
        &mut merge_traversal_state,
    );

    match artifact_queue {
        Some(artifact_queue) => {
            let val: Vec<_> = merge_traversal_state
                .paths_to_refetch_fields
                .into_iter()
                .enumerate()
                .map(
                    |(
                        index,
                        (path_to_refetch_field, refetch_field_parent_id, resolver_variant),
                    )| {
                        let nested_merged_selection_set =
                            find_by_path(&merged_selection_set, &path_to_refetch_field);

                        // TODO we can pre-calculate this instead of re-iterating here
                        let reachable_variables = nested_merged_selection_set.reachable_variables();

                        let definitions_of_used_variables = reachable_variables
                            .iter()
                            .map(|variable_name| {
                                root_fetchable_resolver
                                    .variable_definitions
                                    .iter()
                                    .find(|definition| definition.item.name.item == *variable_name)
                                    // TODO make this an error, don't panic
                                    .expect(&format!(
                                        "Did not find matching variable definition. \
                                This might not be validated yet. For now, each resolver \
                                containing a __refetch field must re-defined all used variables. \
                                Resolver {} is missing variable definition {}",
                                        root_fetchable_resolver.name, variable_name
                                    ))
                                    .clone()
                            })
                            .collect();

                        let field_name = match resolver_variant {
                            ResolverVariant::RefetchField => {
                                artifact_queue.push(ArtifactQueueItem::RefetchField(
                                    RefetchFieldResolverInfo {
                                        merged_selection_set: nested_merged_selection_set,
                                        refetch_field_parent_id,
                                        variable_definitions: definitions_of_used_variables,
                                        root_parent_object: schema
                                            .schema_data
                                            .object(root_fetchable_resolver.parent_object_id)
                                            .name,
                                        root_fetchable_field: root_fetchable_resolver.name,
                                        refetch_query_index: index,
                                    },
                                ));
                                "__refetch".intern().into()
                            }
                            ResolverVariant::MutationField(MutationFieldResolverVariant {
                                mutation_field_name,
                                mutation_primary_field_name,
                                mutation_field_arguments,
                                filtered_mutation_field_arguments: _,
                                mutation_primary_field_return_type_object_id,
                            }) => {
                                let requires_refinement =
                                    if mutation_primary_field_return_type_object_id
                                        == refetch_field_parent_id
                                    {
                                        RequiresRefinement::No
                                    } else {
                                        RequiresRefinement::Yes(
                                            schema.schema_data.object(refetch_field_parent_id).name,
                                        )
                                    };

                                artifact_queue.push(ArtifactQueueItem::MutationField(
                                    MutationFieldResolverInfo {
                                        merged_selection_set: nested_merged_selection_set,
                                        refetch_field_parent_id,
                                        variable_definitions: definitions_of_used_variables,
                                        root_parent_object: schema
                                            .schema_data
                                            .object(root_fetchable_resolver.parent_object_id)
                                            .name,
                                        root_fetchable_field: root_fetchable_resolver.name,
                                        refetch_query_index: index,
                                        mutation_field_name,
                                        mutation_primary_field_name,
                                        mutation_field_arguments: mutation_field_arguments.clone(),
                                        requires_refinement,
                                    },
                                ));
                                mutation_field_name
                            }
                            _ => panic!("invalid resolver variant"),
                        };

                        let mut reachable_variables_vec: Vec<_> =
                            reachable_variables.into_iter().collect();
                        reachable_variables_vec.sort();

                        RootRefetchedPath {
                            path: path_to_refetch_field,
                            variables: reachable_variables_vec,
                            field_name,
                        }
                    },
                )
                .collect();

            (merged_selection_set, val)
        }
        None => {
            let val: Vec<_> = merge_traversal_state
                .paths_to_refetch_fields
                .into_iter()
                .map(|(path_to_refetch_field, _, resolver_variant)| {
                    let nested_merged_selection_set =
                        find_by_path(&merged_selection_set, &path_to_refetch_field);

                    // TODO we can pre-calculate this instead of re-iterating here
                    let reachable_variables = nested_merged_selection_set.reachable_variables();

                    let field_name = match resolver_variant {
                        ResolverVariant::RefetchField => "__refetch".intern().into(),
                        ResolverVariant::MutationField(MutationFieldResolverVariant {
                            mutation_field_name,
                            ..
                        }) => mutation_field_name,
                        _ => panic!("invalid resolver variant"),
                    };

                    let mut reachable_variables_vec: Vec<_> =
                        reachable_variables.into_iter().collect();
                    reachable_variables_vec.sort();

                    RootRefetchedPath {
                        path: path_to_refetch_field,
                        variables: reachable_variables_vec,
                        field_name,
                    }
                })
                .collect();

            (merged_selection_set, val)
        }
    }
}

fn create_merged_selection_set_with_merge_traversal_state(
    schema: &ValidatedSchema,
    parent_type: &ValidatedSchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    merge_traversal_state: &mut MergeTraversalState<'_>,
) -> MergedSelectionSet {
    let mut merged_selection_map = HashMap::new();

    merge_selections_into_set(
        schema,
        &mut merged_selection_map,
        parent_type,
        validated_selections,
        merge_traversal_state,
    );

    select_typename_and_id_fields_in_merged_selection(
        schema,
        &mut merged_selection_map,
        parent_type,
    );

    let merged = MergedSelectionSet::new(merged_selection_map.into_iter().collect());

    merged
}

fn merge_selections_into_set(
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    parent_type: &ValidatedSchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    merge_traversal_state: &mut MergeTraversalState<'_>,
) {
    for validated_selection in validated_selections.iter().filter(filter_id_fields) {
        let span = validated_selection.span;
        match &validated_selection.item {
            Selection::ServerField(validated_server_field) => match validated_server_field {
                ServerFieldSelection::ScalarField(scalar_field) => {
                    match &scalar_field.associated_data {
                        DefinedField::ServerField(_) => {
                            merge_scalar_server_field(scalar_field, merged_selection_map, span);
                        }
                        DefinedField::ResolverField(resolver_field_id) => {
                            if let Some(ref mut encountered_resolver_ids) =
                                merge_traversal_state.encountered_resolver_ids
                            {
                                encountered_resolver_ids.insert(*resolver_field_id);
                            }
                            merge_scalar_resolver_field(
                                parent_type,
                                schema,
                                merged_selection_map,
                                merge_traversal_state,
                                *resolver_field_id,
                            )
                        }
                    };
                }
                ServerFieldSelection::LinkedField(new_linked_field) => {
                    let normalization_key = NormalizationKey::ServerField(name_and_arguments(
                        new_linked_field.name.item.into(),
                        &new_linked_field.arguments,
                    ));
                    merge_traversal_state
                        .current_path
                        .linked_fields
                        .push(NameAndArguments {
                            name: new_linked_field.name.item.into(),
                            arguments: new_linked_field
                                .arguments
                                .iter()
                                .map(|argument| ArgumentKeyAndValue {
                                    key: argument.item.name.item,
                                    value: argument.item.value.item.clone(),
                                })
                                .collect(),
                        });

                    match merged_selection_map.entry(normalization_key) {
                        Entry::Vacant(vacant_entry) => merge_linked_field_into_vacant_entry(
                            vacant_entry,
                            new_linked_field,
                            schema,
                            span,
                            merge_traversal_state,
                        ),
                        Entry::Occupied(occupied) => merge_linked_field_into_occupied_entry(
                            occupied,
                            new_linked_field,
                            schema,
                            merge_traversal_state,
                        ),
                    };

                    merge_traversal_state.current_path.linked_fields.pop();
                }
            },
        }
    }
}

fn filter_id_fields(field: &&WithSpan<ValidatedSelection>) -> bool {
    // filter out id fields, and eventually other always-selected fields like __typename
    match &field.item {
        Selection::ServerField(server_field) => match server_field {
            ServerFieldSelection::ScalarField(scalar_field) => {
                // -------- HACK --------
                // Here, we check whether the field is named "id", but we should really
                // know whether it is an id field in some other way. There can be non-id fields
                // named id and id fields not named "id".
                scalar_field.name.item != "id".intern().into()
                // ------ END HACK ------
            }
            ServerFieldSelection::LinkedField(_) => true,
        },
    }
}

fn merge_linked_field_into_vacant_entry(
    vacant_entry: VacantEntry<'_, NormalizationKey, WithSpan<MergedServerFieldSelection>>,
    new_linked_field: &ValidatedLinkedFieldSelection,
    schema: &ValidatedSchema,
    span: Span,
    merge_traversal_state: &mut MergeTraversalState<'_>,
) {
    vacant_entry.insert(WithSpan::new(
        MergedServerFieldSelection::LinkedField(MergedLinkedFieldSelection {
            name: new_linked_field.name,
            selection_set: {
                let type_id = new_linked_field.associated_data.parent_object_id;
                let linked_field_parent_type = schema.schema_data.object(type_id);
                let merged_set = create_merged_selection_set_with_merge_traversal_state(
                    schema,
                    linked_field_parent_type,
                    &new_linked_field.selection_set,
                    merge_traversal_state,
                );
                merged_set.into()
            },
            arguments: new_linked_field.arguments.clone(),
            normalization_alias: new_linked_field.normalization_alias,
        }),
        span,
    ));
}

fn merge_linked_field_into_occupied_entry(
    mut occupied: OccupiedEntry<'_, NormalizationKey, WithSpan<MergedServerFieldSelection>>,
    new_linked_field: &ValidatedLinkedFieldSelection,
    schema: &ValidatedSchema,
    merge_traversal_state: &mut MergeTraversalState<'_>,
) {
    let existing_selection = occupied.get_mut();
    match &mut existing_selection.item {
        MergedServerFieldSelection::ScalarField(_) => {
            panic!("expected linked, probably a bug in Isograph")
        }
        MergedServerFieldSelection::LinkedField(existing_linked_field) => {
            let type_id = new_linked_field.associated_data.parent_object_id;
            let linked_field_parent_type = schema.schema_data.object(type_id);
            HACK__merge_linked_fields(
                schema,
                &mut existing_linked_field.selection_set,
                &new_linked_field.selection_set,
                linked_field_parent_type,
                merge_traversal_state,
            );
        }
    }
}

fn merge_scalar_resolver_field(
    parent_type: &ValidatedSchemaObject,
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    merge_traversal_state: &mut MergeTraversalState<'_>,
    resolver_field_id: ResolverFieldId,
) {
    let resolver_field = schema.resolver(resolver_field_id);
    if let Some((ref selection_set, _)) = resolver_field.selection_set_and_unwraps {
        merge_selections_into_set(
            schema,
            merged_selection_map,
            parent_type,
            selection_set,
            merge_traversal_state,
        );
    } else {
        panic!("unsupported resolver without selection set");
    }

    // HACK... we can model this data better
    if resolver_field.variant == ResolverVariant::RefetchField {
        merge_traversal_state.paths_to_refetch_fields.push((
            merge_traversal_state.current_path.clone(),
            parent_type.id,
            ResolverVariant::RefetchField,
        ));
    } else if let ResolverVariant::MutationField(MutationFieldResolverVariant {
        mutation_primary_field_name,
        mutation_field_arguments,
        filtered_mutation_field_arguments,
        mutation_field_name: _,
        mutation_primary_field_return_type_object_id,
    }) = &resolver_field.variant
    {
        merge_traversal_state.paths_to_refetch_fields.push((
            merge_traversal_state.current_path.clone(),
            parent_type.id,
            ResolverVariant::MutationField(MutationFieldResolverVariant {
                mutation_field_name: resolver_field.name,
                mutation_primary_field_name: *mutation_primary_field_name,
                mutation_field_arguments: mutation_field_arguments.clone(),
                filtered_mutation_field_arguments: filtered_mutation_field_arguments.clone(),
                mutation_primary_field_return_type_object_id:
                    *mutation_primary_field_return_type_object_id,
            }),
        ));
    }
}

fn merge_scalar_server_field(
    scalar_field: &ScalarFieldSelection<ValidatedDefinedField>,
    merged_selection_set: &mut MergedSelectionMap,
    span: Span,
) {
    let normalization_key = NormalizationKey::ServerField(name_and_arguments(
        scalar_field.name.item.into(),
        &scalar_field.arguments,
    ));
    match merged_selection_set.entry(normalization_key) {
        Entry::Occupied(occupied) => {
            match occupied.get().item {
                MergedServerFieldSelection::ScalarField(_) => {
                    // TODO check that the existing server field matches the one we
                    // would create.
                }
                MergedServerFieldSelection::LinkedField(_) => {
                    panic!("Unexpected linked field, probably a bug in Isograph")
                }
            };
        }
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(WithSpan::new(
                MergedServerFieldSelection::ScalarField(MergedScalarFieldSelection {
                    name: scalar_field.name,
                    arguments: scalar_field.arguments.clone(),
                    normalization_alias: scalar_field.normalization_alias,
                }),
                span,
            ));
        }
    }
}

fn name_and_arguments(
    name: SelectableFieldName,
    arguments: &[WithLocation<SelectionFieldArgument>],
) -> NameAndArguments {
    NameAndArguments {
        name,
        arguments: arguments
            .iter()
            .map(|selection_field_argument| ArgumentKeyAndValue {
                key: selection_field_argument.item.name.item,
                value: selection_field_argument.item.value.item.clone(),
            })
            .collect(),
    }
}

/// In this function, we convert the Vec to a HashMap, do the merging, then
/// convert back. Blah!
#[allow(non_snake_case)]
fn HACK__merge_linked_fields(
    schema: &ValidatedSchema,
    existing_selection_set: &mut Vec<WithSpan<MergedServerFieldSelection>>,
    new_selection_set: &[WithSpan<ValidatedSelection>],
    linked_field_parent_type: &ValidatedSchemaObject,
    merge_traversal_state: &mut MergeTraversalState<'_>,
) {
    let mut merged_selection_set = HashMap::new();
    for item in existing_selection_set.iter() {
        let span = item.span;
        match &item.item {
            MergedServerFieldSelection::ScalarField(scalar_field) => {
                // N.B. if you have a field named "id" which is a linked field, this will probably
                // work incorrectly!
                let normalization_key = NormalizationKey::ServerField(name_and_arguments(
                    scalar_field.name.item.into(),
                    &scalar_field.arguments,
                ));

                merged_selection_set.insert(
                    normalization_key,
                    WithSpan::new(
                        MergedServerFieldSelection::ScalarField(scalar_field.clone()),
                        span,
                    ),
                )
            }
            MergedServerFieldSelection::LinkedField(linked_field) => {
                let normalization_key = NormalizationKey::ServerField(name_and_arguments(
                    linked_field.name.item.into(),
                    &linked_field.arguments,
                ));
                merged_selection_set.insert(
                    normalization_key,
                    WithSpan::new(
                        MergedServerFieldSelection::LinkedField(linked_field.clone()),
                        span,
                    ),
                )
            }
        };
    }

    merge_selections_into_set(
        schema,
        &mut merged_selection_set,
        linked_field_parent_type,
        new_selection_set,
        merge_traversal_state,
    );

    let mut merged_fields: Vec<_> = merged_selection_set
        .into_iter()
        .map(|(_key, value)| value)
        .collect();
    merged_fields.sort();

    *existing_selection_set = merged_fields;
}

fn select_typename_and_id_fields_in_merged_selection(
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    parent_type: &ValidatedSchemaObject,
) {
    // TODO add __typename field or whatnot

    let id_field: Option<ValidatedSchemaIdField> = parent_type
        .id_field
        .map(|id_field_id| schema.id_field(id_field_id));

    // If the type has an id field, we must select it.
    if let Some(id_field) = id_field {
        match merged_selection_map.entry(NormalizationKey::Id) {
            Entry::Occupied(occupied) => {
                match occupied.get().item {
                    MergedServerFieldSelection::ScalarField(_) => {
                        // TODO check that the existing server field matches the one we
                        // would create.
                    }
                    MergedServerFieldSelection::LinkedField(_) => {
                        panic!("Unexpected linked field for id, probably a bug in Isograph")
                    }
                };
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(WithSpan::new(
                    MergedServerFieldSelection::ScalarField(MergedScalarFieldSelection {
                        // major HACK alert
                        name: WithLocation::new(
                            id_field.name.item.lookup().intern().into(),
                            Location::generated(),
                        ),
                        arguments: vec![],
                        // This indicates that there should be a separate MergedServerFieldSelection variant
                        normalization_alias: None,
                    }),
                    Span::todo_generated(),
                ));
            }
        }
    }
}
