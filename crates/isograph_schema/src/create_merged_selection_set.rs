use std::collections::{
    hash_map::{Entry, OccupiedEntry, VacantEntry},
    HashMap, HashSet,
};

use common_lang_types::{
    IsographObjectTypeName, LinkedFieldAlias, LinkedFieldName, Location, QueryOperationName,
    ScalarFieldAlias, ScalarFieldName, SelectableFieldName, Span, VariableName, WithLocation,
    WithSpan,
};
use graphql_lang_types::{GraphQLTypeAnnotation, NamedTypeAnnotation, NonNullTypeAnnotation};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    ClientFieldId, NonConstantValue, RefetchQueryIndex, SelectableServerFieldId, Selection,
    SelectionFieldArgument, ServerFieldSelection, ServerObjectId, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    expose_field_directive::RequiresRefinement, ArgumentKeyAndValue, ClientFieldVariant,
    FieldDefinitionLocation, ImperativelyLoadedFieldVariant, NameAndArguments, PathToRefetchField,
    RootOperationName, ValidatedClientField, ValidatedLinkedFieldSelection,
    ValidatedScalarFieldSelection, ValidatedSchema, ValidatedSchemaIdField, ValidatedSchemaObject,
    ValidatedSelection,
};

type MergedSelectionMap = HashMap<NormalizationKey, WithSpan<MergedServerFieldSelection>>;

lazy_static! {
    pub static ref NODE_FIELD_NAME: LinkedFieldName = "node".intern().into();
    pub static ref TYPENAME_FIELD_NAME: ScalarFieldName = "__typename".intern().into();
}

#[derive(Debug)]
pub struct RootRefetchedPath {
    pub path: PathToRefetchField,
    pub variables: Vec<VariableName>,
    // TODO is this always the same as .path.field_name?
    pub field_name: SelectableFieldName,
}

// TODO add id and typename variants, impl Ord, and get rid of the NormalizationKey enum
#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub enum MergedServerFieldSelection {
    ScalarField(MergedScalarFieldSelection),
    LinkedField(MergedLinkedFieldSelection),
    // TODO does this belong? This is very GraphQL specific.
    InlineFragment(MergedInlineFragmentSelection),
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
            MergedServerFieldSelection::InlineFragment(inline_fragment) => {
                let mut reachable_variables = HashSet::new();
                for selection in inline_fragment.selection_set.iter() {
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

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedInlineFragmentSelection {
    // TODO maybe this should be optional
    pub type_to_refine_to: IsographObjectTypeName,
    // TODO make this type more precise
    // pub selections: NonInlineFragmentSelections
    pub selection_set: Vec<WithSpan<MergedServerFieldSelection>>,
}

/// A merged selection set is an input for generating:
/// - query texts
/// - normalization ASTs
/// - raw response types (TODO)
///
/// For regular and refetch queries.
#[derive(Clone, Debug)]
pub struct MergedSelectionSet(pub Vec<WithSpan<MergedServerFieldSelection>>);

impl std::ops::Deref for MergedSelectionSet {
    type Target = Vec<WithSpan<MergedServerFieldSelection>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl MergedSelectionSet {
    pub fn new(
        // TODO make a normalization_key method on MergedServerFieldSelection
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
pub enum NormalizationKey {
    // __typename,
    Id,
    ServerField(NameAndArguments),
    InlineFragment(IsographObjectTypeName),
}

#[derive(Debug)]
pub enum ArtifactQueueItem {
    RefetchField(RefetchFieldArtifactInfo),
    ImperativelyLoadedField(ImperativelyLoadedFieldArtifactInfo),
}

#[derive(Debug, Clone)]
pub struct RefetchFieldArtifactInfo {
    pub merged_selection_set: MergedSelectionSet,
    pub variable_definitions: Vec<WithSpan<VariableDefinition<SelectableServerFieldId>>>,
    pub root_parent_object: IsographObjectTypeName,
    pub root_fetchable_field: SelectableFieldName,
    // TODO wrap in a newtype
    pub refetch_query_index: RefetchQueryIndex,
    pub query_name: QueryOperationName,
}

#[derive(Debug, Clone)]
pub struct ImperativelyLoadedFieldArtifactInfo {
    pub merged_selection_set: MergedSelectionSet,
    /// Used to look up what type to narrow on in the generated refetch query,
    /// among other things.
    pub variable_definitions: Vec<WithSpan<VariableDefinition<SelectableServerFieldId>>>,
    pub root_parent_object: IsographObjectTypeName,
    pub root_fetchable_field: SelectableFieldName,
    // TODO wrap in a newtype
    pub refetch_query_index: RefetchQueryIndex,

    pub root_operation_name: RootOperationName,
    pub query_name: QueryOperationName,
}

/// This struct contains everything that is available when we start
/// generating a merged selection set for a given fetchable resolver root.
/// A mutable reference to this struct is passed down to all children.
#[derive(Debug)]
struct MergeTraversalState<'a> {
    paths_to_refetch_fields: HashSet<(PathToRefetchField, ServerObjectId, ClientFieldVariant)>,
    /// As we traverse selection sets, we need to keep track of the path we have
    /// taken so far. This is because when we encounter a refetch query, we need
    /// to take note of the path we took to reach that query, but continue
    /// generating the merged selection set.
    ///
    /// Finally, once we have completed generating the merged selection set,
    /// we re-traverse the paths to get the complete merged selection sets
    /// needed for each refetch query. At this point, we have enough information
    /// to generate the refetch query.
    current_path: Vec<NameAndArguments>,
    encountered_client_field_ids: Option<&'a mut HashSet<ClientFieldId>>,
}

impl<'a> MergeTraversalState<'a> {
    pub fn new(encountered_client_field_ids: Option<&'a mut HashSet<ClientFieldId>>) -> Self {
        Self {
            paths_to_refetch_fields: Default::default(),
            current_path: vec![],
            encountered_client_field_ids,
        }
    }

    pub fn sorted_paths_to_refetch_fields(
        self,
    ) -> Vec<(PathToRefetchField, ServerObjectId, ClientFieldVariant)> {
        let mut paths = self.paths_to_refetch_fields.into_iter().collect::<Vec<_>>();
        paths.sort();
        paths
    }
}

pub fn create_merged_selection_set(
    schema: &ValidatedSchema,
    parent_type: &ValidatedSchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    // TODO consider ways to get rid of these parameters.
    artifact_queue: Option<&mut Vec<ArtifactQueueItem>>,
    encountered_client_field_ids: Option<&mut HashSet<ClientFieldId>>,
    // N.B. we call this for non-fetchable resolvers now, but that is a smell
    entrypoint: &ValidatedClientField,
) -> (MergedSelectionSet, Vec<RootRefetchedPath>) {
    let mut merge_traversal_state = MergeTraversalState::new(encountered_client_field_ids);
    let merged_selection_set = create_merged_selection_set_with_merge_traversal_state(
        schema,
        parent_type,
        validated_selections,
        &mut merge_traversal_state,
    );

    match artifact_queue {
        Some(artifact_queue) => {
            let root_refetched_paths: Vec<_> = merge_traversal_state
                .sorted_paths_to_refetch_fields()
                .into_iter()
                .enumerate()
                .map(
                    |(
                        index,
                        (path_to_refetch_field, refetch_field_parent_id, client_field_variant),
                    )| {
                        let nested_merged_selection_set =
                            find_by_path(&merged_selection_set, &path_to_refetch_field);

                        let (field_name, reachable_variables) = match client_field_variant {
                            ClientFieldVariant::RefetchField => {
                                let type_to_refine_to = schema
                                    .server_field_data
                                    .object(refetch_field_parent_id)
                                    .name;

                                let id_arguments = vec![WithLocation::new(
                                    SelectionFieldArgument {
                                        name: WithSpan::new(
                                            "id".intern().into(),
                                            Span::todo_generated(),
                                        ),
                                        value: WithSpan::new(
                                            NonConstantValue::Variable("id".intern().into()),
                                            Span::todo_generated(),
                                        ),
                                    },
                                    Location::generated(),
                                )];

                                let merged_selection_set = selection_set_wrapped(
                                    nested_merged_selection_set,
                                    *NODE_FIELD_NAME,
                                    id_arguments,
                                    None,
                                    RequiresRefinement::Yes(type_to_refine_to),
                                );
                                // TODO we can pre-calculate this instead of re-iterating here
                                let reachable_variables =
                                    merged_selection_set.reachable_variables();

                                let mut definitions_of_used_variables =
                                    get_used_variable_definitions(&reachable_variables, entrypoint);

                                // HACK, we also add the id field
                                definitions_of_used_variables.push(WithSpan::new(
                                    VariableDefinition {
                                        name: WithLocation::new(
                                            "id".intern().into(),
                                            Location::generated(),
                                        ),
                                        type_: GraphQLTypeAnnotation::NonNull(Box::new(
                                            NonNullTypeAnnotation::Named(NamedTypeAnnotation(
                                                WithSpan::new(
                                                    SelectableServerFieldId::Scalar(
                                                        schema.id_type_id,
                                                    ),
                                                    Span::todo_generated(),
                                                ),
                                            )),
                                        )),
                                    },
                                    Span::todo_generated(),
                                ));

                                artifact_queue.push(ArtifactQueueItem::RefetchField(
                                    RefetchFieldArtifactInfo {
                                        merged_selection_set,
                                        variable_definitions: definitions_of_used_variables,
                                        root_parent_object: schema
                                            .server_field_data
                                            .object(entrypoint.parent_object_id)
                                            .name,
                                        root_fetchable_field: entrypoint.name,
                                        refetch_query_index: RefetchQueryIndex(index as u32),
                                        query_name: format!("{type_to_refine_to}__refetch")
                                            .intern()
                                            .into(),
                                    },
                                ));
                                ("__refetch".intern().into(), reachable_variables)
                            }
                            ClientFieldVariant::ImperativelyLoadedField(
                                ImperativelyLoadedFieldVariant {
                                    mutation_field_name,
                                    fetchable_type_original_field_name:
                                        server_schema_mutation_field_name,
                                    aliased_exposed_field_name: mutation_primary_field_name,
                                    mutation_field_arguments,
                                    filtered_mutation_field_arguments: _,
                                    mutation_primary_field_return_type_object_id,
                                    field_map: _,
                                    expose_field_fetchable_field_parent_id,
                                },
                            ) => {
                                // TODO we can pre-calculate this instead of re-iterating here
                                let reachable_variables =
                                    merged_selection_set.reachable_variables();

                                let mut definitions_of_used_variables =
                                    get_used_variable_definitions(&reachable_variables, entrypoint);

                                // It's a bit weird that all exposed fields become imperatively
                                // loaded fields. It probably makes sense to think about how we
                                // can name the things in this block better.

                                let requires_refinement =
                                    if mutation_primary_field_return_type_object_id
                                        == refetch_field_parent_id
                                    {
                                        RequiresRefinement::No
                                    } else {
                                        RequiresRefinement::Yes(
                                            schema
                                                .server_field_data
                                                .object(refetch_field_parent_id)
                                                .name,
                                        )
                                    };

                                let merged_selection_set = selection_set_wrapped(
                                    nested_merged_selection_set,
                                    // TODO why are these types different
                                    server_schema_mutation_field_name.lookup().intern().into(),
                                    mutation_field_arguments
                                        .iter()
                                        // TODO don't clone
                                        .cloned()
                                        .map(|x| {
                                            let variable_name =
                                                x.item.name.map(|value_name| value_name.into());
                                            definitions_of_used_variables.push(WithSpan {
                                                item: VariableDefinition {
                                                    name: variable_name,
                                                    type_: x.item.type_.clone().map(|type_name| {
                                                        *schema
                                                            .server_field_data
                                                            .defined_types
                                                            .get(&type_name.into())
                                                            .expect(
                                                                "Expected type to be found, \
                                                                this indicates a bug in Isograph",
                                                            )
                                                    }),
                                                },
                                                span: Span::todo_generated(),
                                            });
                                            x.map(|item| SelectionFieldArgument {
                                                name: WithSpan::new(
                                                    item.name.item.lookup().intern().into(),
                                                    Span::todo_generated(),
                                                ),
                                                value: WithSpan::new(
                                                    NonConstantValue::Variable(
                                                        item.name.item.into(),
                                                    ),
                                                    Span::todo_generated(),
                                                ),
                                            })
                                        })
                                        .collect(),
                                    Some(mutation_primary_field_name.lookup().intern().into()),
                                    requires_refinement,
                                );

                                let root_parent_object = schema
                                    .server_field_data
                                    .object(entrypoint.parent_object_id)
                                    .name;
                                artifact_queue.push(ArtifactQueueItem::ImperativelyLoadedField(
                                    ImperativelyLoadedFieldArtifactInfo {
                                        merged_selection_set,
                                        root_parent_object,
                                        variable_definitions: definitions_of_used_variables,
                                        root_fetchable_field: entrypoint.name,
                                        refetch_query_index: RefetchQueryIndex(index as u32),
                                        root_operation_name: schema
                                            .fetchable_types
                                            .get(&expose_field_fetchable_field_parent_id)
                                            .expect(
                                                "Expected root type to be fetchable here.\
                                                 This is indicative of a bug in Isograph.",
                                            )
                                            .clone(),
                                        query_name: format!(
                                            "{root_parent_object}__{mutation_field_name}"
                                        )
                                        .intern()
                                        .into(),
                                    },
                                ));
                                (mutation_field_name, reachable_variables)
                            }
                            _ => panic!("invalid client field variant"),
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

            (merged_selection_set, root_refetched_paths)
        }
        None => {
            // TODO it is weird that we call this without an artifact queue!
            let root_refetched_paths: Vec<_> = merge_traversal_state
                .sorted_paths_to_refetch_fields()
                .into_iter()
                .map(|(path_to_refetch_field, _, client_field_variant)| {
                    let nested_merged_selection_set =
                        find_by_path(&merged_selection_set, &path_to_refetch_field);

                    // TODO we can pre-calculate this instead of re-iterating here
                    let reachable_variables = nested_merged_selection_set.reachable_variables();

                    let field_name = match client_field_variant {
                        ClientFieldVariant::RefetchField => "__refetch".intern().into(),
                        ClientFieldVariant::ImperativelyLoadedField(
                            ImperativelyLoadedFieldVariant {
                                mutation_field_name,
                                ..
                            },
                        ) => mutation_field_name,
                        _ => panic!("invalid client field variant"),
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

            (merged_selection_set, root_refetched_paths)
        }
    }
}

fn get_used_variable_definitions(
    reachable_variables: &HashSet<VariableName>,
    entrypoint: &ValidatedClientField,
) -> Vec<WithSpan<VariableDefinition<SelectableServerFieldId>>> {
    reachable_variables
        .iter()
        .flat_map(|variable_name| {
            // HACK
            if variable_name == &"id".intern().into() {
                None
            } else {
                Some(
                    entrypoint
                        .variable_definitions
                        .iter()
                        .find(|definition| definition.item.name.item == *variable_name)
                        .expect(&format!(
                            "Did not find matching variable definition. \
                            This might not be validated yet. For now, each client field \
                            containing a __refetch field must re-defined all used variables. \
                            Client field {} is missing variable definition {}",
                            entrypoint.name, variable_name
                        ))
                        .clone(),
                )
            }
        })
        .collect::<Vec<_>>()
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
                    match &scalar_field.associated_data.location {
                        FieldDefinitionLocation::Server(_) => {
                            merge_scalar_server_field(scalar_field, merged_selection_map, span);
                        }
                        FieldDefinitionLocation::Client(client_field_id) => {
                            if let Some(ref mut encountered_client_field_ids) =
                                merge_traversal_state.encountered_client_field_ids
                            {
                                encountered_client_field_ids.insert(*client_field_id);
                            }
                            merge_scalar_client_field(
                                parent_type,
                                schema,
                                merged_selection_map,
                                merge_traversal_state,
                                *client_field_id,
                            )
                        }
                    };
                }
                ServerFieldSelection::LinkedField(new_linked_field) => {
                    let normalization_key = NormalizationKey::ServerField(name_and_arguments(
                        new_linked_field.name.item.into(),
                        &new_linked_field.arguments,
                    ));
                    merge_traversal_state.current_path.push(NameAndArguments {
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

                    merge_traversal_state.current_path.pop();
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
                let linked_field_parent_type = schema.server_field_data.object(type_id);
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
            let linked_field_parent_type = schema.server_field_data.object(type_id);
            HACK__merge_linked_fields(
                schema,
                &mut existing_linked_field.selection_set,
                &new_linked_field.selection_set,
                linked_field_parent_type,
                merge_traversal_state,
            );
        }
        MergedServerFieldSelection::InlineFragment(_) => {
            panic!("Unexpected inline fragment, probably a bug in Isograph");
        }
    }
}

fn merge_scalar_client_field(
    parent_type: &ValidatedSchemaObject,
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    merge_traversal_state: &mut MergeTraversalState<'_>,
    client_field_id: ClientFieldId,
) {
    let client_field = schema.client_field(client_field_id);
    if let Some((ref selection_set, _)) = client_field.selection_set_and_unwraps {
        merge_selections_into_set(
            schema,
            merged_selection_map,
            parent_type,
            selection_set,
            merge_traversal_state,
        );
    } else {
        panic!("unsupported client field without selection set");
    }

    // HACK... we can model this data better
    if let ClientFieldVariant::RefetchField = client_field.variant {
        merge_traversal_state.paths_to_refetch_fields.insert((
            PathToRefetchField {
                linked_fields: merge_traversal_state.current_path.clone(),
                field_name: client_field.name,
            },
            parent_type.id,
            ClientFieldVariant::RefetchField,
        ));
    } else if let ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
        aliased_exposed_field_name: mutation_primary_field_name,
        fetchable_type_original_field_name: server_schema_mutation_field_name,
        mutation_field_arguments,
        filtered_mutation_field_arguments,
        mutation_field_name: _,
        mutation_primary_field_return_type_object_id,
        field_map,
        expose_field_fetchable_field_parent_id: expose_field_parent_id,
    }) = &client_field.variant
    {
        merge_traversal_state.paths_to_refetch_fields.insert((
            PathToRefetchField {
                linked_fields: merge_traversal_state.current_path.clone(),
                field_name: client_field.name,
            },
            parent_type.id,
            ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
                mutation_field_name: client_field.name,
                fetchable_type_original_field_name: *server_schema_mutation_field_name,
                aliased_exposed_field_name: *mutation_primary_field_name,
                mutation_field_arguments: mutation_field_arguments.clone(),
                filtered_mutation_field_arguments: filtered_mutation_field_arguments.clone(),
                mutation_primary_field_return_type_object_id:
                    *mutation_primary_field_return_type_object_id,
                field_map: field_map.clone(),
                expose_field_fetchable_field_parent_id: *expose_field_parent_id,
            }),
        ));
    }
}

fn merge_scalar_server_field(
    scalar_field: &ValidatedScalarFieldSelection,
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
                    panic!("Unexpected linked field, probably a bug in Isograph");
                }
                MergedServerFieldSelection::InlineFragment(_) => {
                    panic!("Unexpected inline fragment, probably a bug in Isograph");
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
            MergedServerFieldSelection::InlineFragment(_inline_fragment) => {
                panic!("Unexpectedly encountered inline fragment");
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
                        panic!("Unexpected linked field for id, probably a bug in Isograph");
                    }
                    MergedServerFieldSelection::InlineFragment(_) => {
                        panic!("Unexpected inline fragment, probably a bug in Isograph");
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

pub fn selection_set_wrapped(
    mut merged_selection_set: MergedSelectionSet,
    top_level_field: LinkedFieldName,
    top_level_field_arguments: Vec<WithLocation<SelectionFieldArgument>>,
    // TODO support arguments and vectors of subfields
    subfield: Option<LinkedFieldName>,
    type_to_refine_to: RequiresRefinement,
) -> MergedSelectionSet {
    // We are proceeding inside out, i.e. creating
    // `mutation_name { subfield { ...on Type { existing_selection_set }}}`
    // first by creating the inline fragment, then subfield, etc.

    // Should we wrap the selection set in a type to refine to?
    let selection_set_with_inline_fragment = match type_to_refine_to {
        RequiresRefinement::Yes(type_to_refine_to) => {
            maybe_add_typename_selection(&mut merged_selection_set);
            MergedSelectionSet::new(vec![(
                NormalizationKey::InlineFragment(type_to_refine_to),
                WithSpan::new(
                    MergedServerFieldSelection::InlineFragment(MergedInlineFragmentSelection {
                        type_to_refine_to,
                        selection_set: merged_selection_set.0,
                    }),
                    Span::todo_generated(),
                ),
            )])
        }
        RequiresRefinement::No => merged_selection_set,
    };

    let selection_set_with_subfield = match subfield {
        Some(subfield) => MergedSelectionSet::new(vec![(
            NormalizationKey::ServerField(NameAndArguments {
                name: subfield.into(),
                arguments: vec![],
            }),
            WithSpan::new(
                MergedServerFieldSelection::LinkedField(MergedLinkedFieldSelection {
                    name: WithLocation::new(subfield, Location::generated()),
                    // TODO
                    normalization_alias: None,
                    selection_set: selection_set_with_inline_fragment.0,
                    arguments: vec![],
                }),
                Span::todo_generated(),
            ),
        )]),
        None => selection_set_with_inline_fragment,
    };

    let top_level_selection_set = MergedSelectionSet::new(vec![
        ((
            NormalizationKey::ServerField(NameAndArguments {
                name: top_level_field.into(),
                // TODO provide arguments. They don't matter, because there is only one
                // selection at this level.
                arguments: vec![],
            }),
            WithSpan::new(
                MergedServerFieldSelection::LinkedField(MergedLinkedFieldSelection {
                    name: WithLocation::new(top_level_field, Location::generated()),
                    normalization_alias: Some(WithLocation::new(
                        get_aliased_mutation_field_name(
                            top_level_field.into(),
                            &top_level_field_arguments,
                        )
                        .intern()
                        .into(),
                        Location::generated(),
                    )),

                    selection_set: selection_set_with_subfield.0,
                    arguments: top_level_field_arguments,
                }),
                Span::todo_generated(),
            ),
        )),
    ]);

    top_level_selection_set
}

fn is_typename_selection<'a>(selection: &'a &WithSpan<MergedServerFieldSelection>) -> bool {
    if let MergedServerFieldSelection::ScalarField(s) = &selection.item {
        s.name.item == *TYPENAME_FIELD_NAME
    } else {
        false
    }
}

fn maybe_add_typename_selection(selections: &mut MergedSelectionSet) {
    let has_typename = selections.iter().find(is_typename_selection).is_some();
    if !has_typename {
        // This should be first, so this a huge bummer
        selections.0.push(WithSpan::new(
            MergedServerFieldSelection::ScalarField(MergedScalarFieldSelection {
                name: WithLocation::new(*TYPENAME_FIELD_NAME, Location::generated()),
                normalization_alias: None,
                arguments: vec![],
            }),
            Span::todo_generated(),
        ));
    }
}

pub fn get_aliased_mutation_field_name(
    name: SelectableFieldName,
    parameters: &[WithLocation<SelectionFieldArgument>],
) -> String {
    let mut s = name.to_string();

    for param in parameters.iter() {
        // TODO NonConstantValue will format to a string like "$name", but we want just "name".
        // There is probably a better way to do this.
        s.push_str("____");
        s.push_str(&param.item.to_alias_str_chunk());
    }
    s
}
