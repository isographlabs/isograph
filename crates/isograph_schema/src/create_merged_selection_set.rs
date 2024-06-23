use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, HashSet};

use common_lang_types::{
    IsographObjectTypeName, LinkedFieldAlias, LinkedFieldName, QueryOperationName,
    ScalarFieldAlias, ScalarFieldName, SelectableFieldName, Span, VariableName, WithLocation,
    WithSpan,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    ClientFieldId, NonConstantValue, RefetchQueryIndex, SelectableServerFieldId, Selection,
    SelectionFieldArgument, ServerFieldSelection, ServerObjectId, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    expose_field_directive::RequiresRefinement, ArgumentKeyAndValue, ClientFieldVariant,
    FieldDefinitionLocation, ImperativelyLoadedFieldVariant, NameAndArguments, PathToRefetchField,
    RootOperationName, SchemaObject, ValidatedClientField, ValidatedScalarFieldSelection,
    ValidatedSchema, ValidatedSchemaIdField, ValidatedSelection,
};

pub type MergedSelectionMap = BTreeMap<NormalizationKey, MergedServerSelection>;

// Maybe this should be FNVHashMap? We don't really need stable iteration order
pub type ClientFieldToCompletedMergeTraversalStateMap =
    BTreeMap<ClientFieldId, (ScalarClientFieldTraversalState, MergedSelectionMap)>;

lazy_static! {
    pub static ref REFETCH_FIELD_NAME: ScalarFieldName = "__refetch".intern().into();
    pub static ref NODE_FIELD_NAME: LinkedFieldName = "node".intern().into();
    pub static ref TYPENAME_FIELD_NAME: ScalarFieldName = "__typename".intern().into();
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootRefetchedPath {
    pub field_name: SelectableFieldName,
    pub path_to_refetch_field_info: PathToRefetchFieldInfo,
}

// TODO add id and typename variants, impl Ord, and get rid of the NormalizationKey enum
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MergedServerSelection {
    ScalarField(MergedScalarFieldSelection),
    LinkedField(MergedLinkedFieldSelection),
    // TODO does this belong? This is very GraphQL specific.
    InlineFragment(MergedInlineFragmentSelection),
}

impl MergedServerSelection {
    pub fn reachable_variables(&self) -> Vec<VariableName> {
        match self {
            MergedServerSelection::ScalarField(field) => get_variables(&field.arguments).collect(),
            MergedServerSelection::LinkedField(field) => get_variables(&field.arguments)
                .chain(
                    field
                        .selection_map
                        .values()
                        .flat_map(|x| x.reachable_variables()),
                )
                .collect(),
            MergedServerSelection::InlineFragment(_) => vec![],
        }
    }
}

fn get_variables<'a>(
    arguments: &'a [SelectionFieldArgument],
) -> impl Iterator<Item = VariableName> + 'a {
    arguments.iter().flat_map(|arg| match arg.value.item {
        isograph_lang_types::NonConstantValue::Variable(v) => Some(v),
        _ => None,
    })
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    // TODO no location
    pub name: ScalarFieldName,
    // TODO calculate this when needed
    pub normalization_alias: Option<ScalarFieldAlias>,
    pub arguments: Vec<SelectionFieldArgument>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MergedLinkedFieldSelection {
    // TODO no location
    pub name: LinkedFieldName,
    // TODO calculate this when needed
    pub normalization_alias: Option<LinkedFieldAlias>,
    pub selection_map: MergedSelectionMap,
    pub arguments: Vec<SelectionFieldArgument>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MergedInlineFragmentSelection {
    pub type_to_refine_to: IsographObjectTypeName,
    // TODO make this type more precise, this selection map should not contain inline fragments
    pub selection_map: MergedSelectionMap,
}

#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord, Hash)]
pub enum NormalizationKey {
    Discriminator, // AKA typename
    Id,
    ServerField(NameAndArguments),
    InlineFragment(IsographObjectTypeName),
}

#[derive(Debug, Clone)]
pub struct ImperativelyLoadedFieldArtifactInfo {
    pub merged_selection_set: MergedSelectionMap,
    /// Used to look up what type to narrow on in the generated refetch query,
    /// among other things.
    pub variable_definitions: Vec<WithSpan<VariableDefinition<SelectableServerFieldId>>>,
    pub root_parent_object: IsographObjectTypeName,
    pub root_fetchable_field: SelectableFieldName,
    pub refetch_query_index: RefetchQueryIndex,

    pub root_operation_name: RootOperationName,
    pub query_name: QueryOperationName,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathToRefetchFieldInfo {
    refetch_field_parent_id: ServerObjectId,
    pub imperatively_loaded_field_variant: ImperativelyLoadedFieldVariant,
    extra_selections: MergedSelectionMap,
}

/// As we traverse, whenever we enter a new scalar client field (including at the
/// root, with the entrypoint), we create a new one of these and pass it down.
///
/// Then, when we are done traversing that scalar client field, we combine the results
/// of that ScalarClientFieldTraversalState with the parent traversal state (e.g.
/// to note of nested refetch fields).
///
/// N.B. there should be two versions of this struct, an in-progress and completed
/// version, the completed one should not have path_since_client_field. (Or that
/// should be achieved but not as described.)
#[derive(Debug, Clone)]
pub struct ScalarClientFieldTraversalState {
    /// As we traverse, if we encounter a refetch path, we note it here
    pub refetch_paths: BTreeMap<PathToRefetchField, RootRefetchedPath>,

    /// Likewise for reachable variables
    pub reachable_variables: HashSet<VariableName>,

    /// The (mutable) path from the current client field to wherever we are iterating
    traversal_path: Vec<NameAndArguments>,

    /// Client fields that are directly accessed by this client field
    pub accessible_client_fields: HashSet<ClientFieldId>,
}

impl ScalarClientFieldTraversalState {
    fn new() -> Self {
        Self {
            refetch_paths: BTreeMap::new(),
            reachable_variables: HashSet::new(),
            traversal_path: vec![],
            accessible_client_fields: HashSet::new(),
        }
    }

    // TODO should this be two separate functions?
    fn incorporate_results_of_iterating_into_child(
        &mut self,
        child_traversal_state: &ScalarClientFieldTraversalState,
    ) {
        self.reachable_variables
            .extend(child_traversal_state.reachable_variables.iter());

        // TODO self.path_since_client_field should be a parameter to this function
        self.refetch_paths
            .extend(child_traversal_state.refetch_paths.iter().map(
                |(path, root_refetched_path)| {
                    // TODO don't clone
                    let mut path = path.clone();
                    let mut complete_path = self.traversal_path.clone();
                    complete_path.extend(path.linked_fields);
                    path.linked_fields = complete_path;

                    (path, root_refetched_path.clone())
                },
            ));
    }
}

// This is unused, and should be deleted. It's unused because:
// we already have passed the correct nested selection map, so we don't need
// to follow the traversal path from the "root" selection map to the nested
// one
pub fn current_target_merged_selections<'a, 'b>(
    traversal_path: impl Iterator<Item = &'a NameAndArguments> + 'a,
    mut parent_selection_map: &'b MergedSelectionMap,
) -> &'b MergedSelectionMap {
    for linked_field in traversal_path {
        match parent_selection_map
            .get(&linked_field.normalization_key())
            .expect("Expected linked field to exist by now. This is indicate of a bug in Isograph.")
        {
            MergedServerSelection::ScalarField(_) => {
                panic!("Expected a linked field, found scalar. This is indicative of a bug in Isograph.")
            }
            MergedServerSelection::LinkedField(ref linked_field) => {
                parent_selection_map = &linked_field.selection_map;
            }
            MergedServerSelection::InlineFragment(_) => {
                panic!("Expected a linked field, found inline fragment. This is indicative of a bug in Isograph.")
            }
        }
    }
    parent_selection_map
}

fn merge_selections_into_selection_map(
    mutable_destination_map: &mut MergedSelectionMap,
    source_map: &MergedSelectionMap,
) {
    for (new_normalization_key, new_server_field_selection) in source_map.iter() {
        match mutable_destination_map.entry(new_normalization_key.clone()) {
            Entry::Vacant(vacant) => {
                vacant.insert(new_server_field_selection.clone());
            }
            Entry::Occupied(mut occupied) => {
                let inner = occupied.get_mut();
                match inner {
                    MergedServerSelection::ScalarField(_) => {
                        assert!(
                            matches!(
                                new_server_field_selection,
                                MergedServerSelection::ScalarField(_)
                            ),
                            "Error: tried to merge a non-scalar into a scalar. This \
                            is indicative of a bug in Isograph."
                        );
                        // N.B. no action is required, since a scalar has no subselections
                    }
                    MergedServerSelection::LinkedField(target_linked_field) => {
                        if let MergedServerSelection::LinkedField(child_linked_field) =
                            new_server_field_selection
                        {
                            merge_selections_into_selection_map(
                                &mut target_linked_field.selection_map,
                                &child_linked_field.selection_map,
                            )
                        } else {
                            panic!(
                                "Error: tried to merge non-linked field into linked field. This \
                                is indicative of a bug in Isograph."
                            )
                        }
                    }
                    MergedServerSelection::InlineFragment(target_inline_fragment) => {
                        if let MergedServerSelection::InlineFragment(child_inline_fragment) =
                            new_server_field_selection
                        {
                            merge_selections_into_selection_map(
                                &mut target_inline_fragment.selection_map,
                                &child_inline_fragment.selection_map,
                            )
                        } else {
                            panic!(
                                "Error: tried to merge non-inline fragment into inline fragment. \
                                This is indicative of a bug in Isograph."
                            )
                        }
                    }
                }
            }
        };
    }
}

pub fn create_merged_selection_map_and_insert_into_global_map(
    schema: &ValidatedSchema,
    parent_type: &SchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
    root_object: &ValidatedClientField,
) -> (ScalarClientFieldTraversalState, MergedSelectionMap) {
    // TODO move this check outside of this function
    match global_client_field_map.get(&root_object.id) {
        Some(merge_traversal_state_and_selection_map) => {
            merge_traversal_state_and_selection_map.clone()
        }
        None => {
            let mut merge_traversal_state = ScalarClientFieldTraversalState::new();
            let selection_map = create_selection_map_with_merge_traversal_state(
                schema,
                parent_type,
                validated_selections,
                &mut merge_traversal_state,
                global_client_field_map,
            );

            // N.B. global_client_field_map might actually have an item stored in root_object.id,
            // if we have some sort of recursion. That probably stack overflows right now.
            global_client_field_map.insert(
                root_object.id,
                (merge_traversal_state.clone(), selection_map.clone()),
            );

            // TODO we don't always use this return value, so we shouldn't always clone above
            (merge_traversal_state, selection_map)
        }
    }
}

pub fn get_imperatively_loaded_artifact_info(
    schema: &ValidatedSchema,
    entrypoint: &ValidatedClientField,
    root_refetch_path: RootRefetchedPath,
    nested_selection_map: &MergedSelectionMap,
    reachable_variables: &BTreeSet<VariableName>,
    index: usize,
) -> ImperativelyLoadedFieldArtifactInfo {
    let RootRefetchedPath {
        path_to_refetch_field_info,
        ..
    } = root_refetch_path;
    let PathToRefetchFieldInfo {
        refetch_field_parent_id,
        imperatively_loaded_field_variant,
        extra_selections: _,
    } = path_to_refetch_field_info;

    let artifact_info = process_imperatively_loaded_field(
        schema,
        imperatively_loaded_field_variant,
        refetch_field_parent_id,
        nested_selection_map,
        entrypoint,
        index,
        &reachable_variables,
    );

    artifact_info
}

pub fn get_reachable_variables(selection_map: &MergedSelectionMap) -> BTreeSet<VariableName> {
    selection_map
        .values()
        .flat_map(|x| x.reachable_variables())
        .collect()
}

fn process_imperatively_loaded_field(
    schema: &ValidatedSchema,
    variant: ImperativelyLoadedFieldVariant,
    refetch_field_parent_id: ServerObjectId,
    selection_map: &MergedSelectionMap,
    entrypoint: &ValidatedClientField,
    index: usize,
    reachable_variables: &BTreeSet<VariableName>,
) -> ImperativelyLoadedFieldArtifactInfo {
    let ImperativelyLoadedFieldVariant {
        client_field_scalar_selection_name,
        top_level_schema_field_name,
        top_level_schema_field_arguments,
        primary_field_info,
        root_object_id,
    } = variant;
    let client_field_scalar_selection_name = client_field_scalar_selection_name;
    // This could be Pet
    let refetch_field_parent_type = schema.server_field_data.object(refetch_field_parent_id);

    let mut definitions_of_used_variables =
        get_used_variable_definitions(&reachable_variables, entrypoint);

    let requires_refinement = if primary_field_info
        .as_ref()
        .map(|x| x.primary_field_return_type_object_id != refetch_field_parent_id)
        .unwrap_or(true)
    {
        RequiresRefinement::Yes(refetch_field_parent_type.name)
    } else {
        RequiresRefinement::No
    };

    // TODO consider wrapping this when we first create the RootRefetchedPath?
    let wrapped_selection_map = selection_set_wrapped(
        selection_map.clone(),
        // TODO why are these types different
        top_level_schema_field_name.lookup().intern().into(),
        top_level_schema_field_arguments
            .iter()
            // TODO don't clone
            .cloned()
            .map(|x| {
                let variable_name = x.name.map(|value_name| value_name.into());
                definitions_of_used_variables.push(WithSpan {
                    item: VariableDefinition {
                        name: variable_name,
                        type_: x.type_.clone().map(|type_name| {
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

                SelectionFieldArgument {
                    name: WithSpan::new(
                        x.name.item.lookup().intern().into(),
                        Span::todo_generated(),
                    ),
                    value: WithSpan::new(
                        NonConstantValue::Variable(x.name.item.into()),
                        Span::todo_generated(),
                    ),
                }
            })
            .collect(),
        primary_field_info
            .as_ref()
            .map(|x| x.primary_field_name.lookup().intern().into()),
        requires_refinement,
    );

    let root_parent_object = schema
        .server_field_data
        .object(entrypoint.parent_object_id)
        .name;

    let root_operation_name = schema
        .fetchable_types
        .get(&root_object_id)
        .expect(
            "Expected root type to be fetchable here.\
            This is indicative of a bug in Isograph.",
        )
        .clone();

    let query_name = if primary_field_info.is_some() {
        format!(
            "{}__{}",
            root_parent_object, client_field_scalar_selection_name
        )
    } else {
        format!("{}__refetch", refetch_field_parent_type.name)
    }
    .intern()
    .into();

    ImperativelyLoadedFieldArtifactInfo {
        // TODO don't clone, have lifetime parameter
        merged_selection_set: wrapped_selection_map,
        root_parent_object,
        variable_definitions: definitions_of_used_variables,
        root_fetchable_field: entrypoint.name,
        refetch_query_index: RefetchQueryIndex(index as u32),
        root_operation_name,
        query_name,
    }
    // todo!("Process imperatively loaded field")
}

fn get_used_variable_definitions(
    reachable_variables: &BTreeSet<VariableName>,
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

fn create_selection_map_with_merge_traversal_state(
    schema: &ValidatedSchema,
    parent_type: &SchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
) -> MergedSelectionMap {
    let mut merged_selection_map = BTreeMap::new();
    merge_validated_selections_into_selection_map(
        schema,
        &mut merged_selection_map,
        parent_type,
        validated_selections,
        merge_traversal_state,
        global_client_field_map,
    );

    merged_selection_map
}

fn merge_validated_selections_into_selection_map(
    schema: &ValidatedSchema,
    parent_map: &mut MergedSelectionMap,
    parent_type: &SchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
) {
    for validated_selection in validated_selections.iter().filter(filter_id_fields) {
        match &validated_selection.item {
            Selection::ServerField(validated_server_field) => {
                let variables = validated_server_field.variables();
                merge_traversal_state
                    .reachable_variables
                    .extend(variables.into_iter());

                match validated_server_field {
                    ServerFieldSelection::ScalarField(scalar_field) => {
                        match &scalar_field.associated_data.location {
                            FieldDefinitionLocation::Server(_) => {
                                merge_scalar_server_field(scalar_field, parent_map);
                            }
                            FieldDefinitionLocation::Client(client_field_id) => {
                                let newly_encountered_scalar_client_field =
                                    schema.client_field(*client_field_id);
                                if let Some((path, info)) = optional_field_refetch_info(
                                    newly_encountered_scalar_client_field,
                                    merge_traversal_state,
                                    parent_type,
                                ) {
                                    merge_traversal_state.refetch_paths.insert(
                                        path.clone(),
                                        RootRefetchedPath {
                                            field_name: newly_encountered_scalar_client_field.name,
                                            path_to_refetch_field_info: info,
                                        },
                                    );
                                }

                                merge_traversal_state
                                    .accessible_client_fields
                                    .insert(*client_field_id);

                                merge_scalar_client_field(
                                    parent_type,
                                    schema,
                                    parent_map,
                                    merge_traversal_state,
                                    newly_encountered_scalar_client_field,
                                    global_client_field_map,
                                )
                            }
                        };
                    }
                    ServerFieldSelection::LinkedField(new_linked_field) => {
                        let normalization_key = name_and_arguments(
                            new_linked_field.name.item.into(),
                            &new_linked_field.arguments,
                        )
                        .normalization_key();
                        merge_traversal_state.traversal_path.push(NameAndArguments {
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

                        // We are creating the linked field, and inserting it into the parent object
                        // first, because otherwise, when we try to merge the results into the parent
                        // selection_map, we find that the linked field we are about to insert is
                        // missing, and panic.
                        //
                        // This might be indicative of poor modeling.
                        let linked_field =
                            parent_map.entry(normalization_key).or_insert_with(|| {
                                MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                                    name: new_linked_field.name.item,
                                    normalization_alias: new_linked_field
                                        .normalization_alias
                                        .map(|x| x.item),
                                    selection_map: BTreeMap::new(),
                                    arguments: new_linked_field
                                        .arguments
                                        .iter()
                                        .map(|x| x.item.clone())
                                        .collect(),
                                })
                            });
                        match linked_field {
                            MergedServerSelection::ScalarField(_) => {
                                panic!(
                                    "Expected linked field, but encountered scalar. \
                                    This is indicative of a bug in Isograph."
                                )
                            }
                            MergedServerSelection::LinkedField(existing_linked_field) => {
                                let type_id = new_linked_field.associated_data.parent_object_id;
                                let linked_field_parent_type =
                                    schema.server_field_data.object(type_id);

                                merge_validated_selections_into_selection_map(
                                    schema,
                                    &mut existing_linked_field.selection_map,
                                    linked_field_parent_type,
                                    &new_linked_field.selection_set,
                                    merge_traversal_state,
                                    global_client_field_map,
                                );
                            }
                            MergedServerSelection::InlineFragment(_) => {
                                panic!(
                                    "Expected linked field, but encountered inline fragment. \
                                    This is indicative of a bug in Isograph."
                                )
                            }
                        }

                        merge_traversal_state.traversal_path.pop();
                    }
                }
            }
        }
    }

    select_typename_and_id_fields_in_merged_selection(schema, parent_map, parent_type);
}

// TODO remove
fn optional_field_refetch_info(
    client_field: &ValidatedClientField,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    parent_type: &SchemaObject,
) -> Option<(PathToRefetchField, PathToRefetchFieldInfo)> {
    match &client_field.variant {
        ClientFieldVariant::ImperativelyLoadedField(variant) => Some((
            PathToRefetchField {
                linked_fields: merge_traversal_state.traversal_path.clone(),
                field_name: client_field.name,
            },
            PathToRefetchFieldInfo {
                refetch_field_parent_id: parent_type.id,
                imperatively_loaded_field_variant: variant.clone(),
                extra_selections: BTreeMap::new(),
            },
        )),
        _ => None,
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

fn merge_scalar_client_field(
    parent_type: &SchemaObject,
    schema: &ValidatedSchema,
    parent_map: &mut MergedSelectionMap,
    parent_merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_scalar_client_field: &ValidatedClientField,
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
) {
    let (child_traversal_state, child_map) = create_merged_selection_map_and_insert_into_global_map(
        schema,
        parent_type,
        &newly_encountered_scalar_client_field.selection_set,
        global_client_field_map,
        newly_encountered_scalar_client_field,
    );

    parent_merge_traversal_state
        .incorporate_results_of_iterating_into_child(&child_traversal_state);
    merge_selections_into_selection_map(parent_map, &child_map);
}

fn merge_scalar_server_field(
    scalar_field: &ValidatedScalarFieldSelection,
    parent_map: &mut MergedSelectionMap,
) {
    let normalization_key = NormalizationKey::ServerField(name_and_arguments(
        scalar_field.name.item.into(),
        &scalar_field.arguments,
    ));
    match parent_map.entry(normalization_key) {
        Entry::Occupied(occupied) => {
            match occupied.get() {
                MergedServerSelection::ScalarField(_) => {
                    // TODO check that the existing server field matches the one we
                    // would create.
                }
                MergedServerSelection::LinkedField(_) => {
                    panic!("Unexpected linked field, probably a bug in Isograph");
                }
                MergedServerSelection::InlineFragment(_) => {
                    panic!("Unexpected inline fragment, probably a bug in Isograph");
                }
            };
        }
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(MergedServerSelection::ScalarField(
                MergedScalarFieldSelection {
                    name: scalar_field.name.item,
                    arguments: scalar_field
                        .arguments
                        .iter()
                        .map(|x| x.item.clone())
                        .collect(),
                    normalization_alias: scalar_field.normalization_alias.map(|x| x.item),
                },
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

// /// In this function, we convert the Vec to a HashMap, do the merging, then
// /// convert back. Blah!
// #[allow(non_snake_case)]
// fn HACK__merge_linked_fields(
//     schema: &ValidatedSchema,
//     existing_selection_set: &mut Vec<WithSpan<MergedServerSelection>>,
//     new_selection_set: &[WithSpan<ValidatedSelection>],
//     linked_field_parent_type: &SchemaObject,
//     merge_traversal_state: &mut ScalarClientFieldTraversalState,
//     global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
// ) {
//     let mut merged_selection_set = HashMap::new();
//     for item in existing_selection_set.iter() {
//         let span = item.span;
//         match &item.item {
//             MergedServerSelection::ScalarField(scalar_field) => {
//                 // N.B. if you have a field named "id" which is a linked field, this will probably
//                 // work incorrectly!
//                 let normalization_key = NormalizationKey::ServerField(name_and_arguments(
//                     scalar_field.name.item.into(),
//                     &scalar_field.arguments,
//                 ));

//                 merged_selection_set.insert(
//                     normalization_key,
//                     WithSpan::new(
//                         MergedServerSelection::ScalarField(scalar_field.clone()),
//                         span,
//                     ),
//                 )
//             }
//             MergedServerSelection::LinkedField(linked_field) => {
//                 let normalization_key = NormalizationKey::ServerField(name_and_arguments(
//                     linked_field.name.item.into(),
//                     &linked_field.arguments,
//                 ));
//                 merged_selection_set.insert(
//                     normalization_key,
//                     WithSpan::new(
//                         MergedServerSelection::LinkedField(linked_field.clone()),
//                         span,
//                     ),
//                 )
//             }
//             MergedServerSelection::InlineFragment(_inline_fragment) => {
//                 panic!("Unexpectedly encountered inline fragment");
//             }
//         };
//     }

//     merge_selections_into_set(
//         schema,
//         &mut merged_selection_set,
//         linked_field_parent_type,
//         new_selection_set,
//         merge_traversal_state,
//         global_client_field_map,
//     );

//     let mut merged_fields: Vec<_> = merged_selection_set
//         .into_iter()
//         .map(|(_key, value)| value)
//         .collect();
//     merged_fields.sort();

//     *existing_selection_set = merged_fields;
// }

fn select_typename_and_id_fields_in_merged_selection(
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    parent_type: &SchemaObject,
) {
    // TODO add __typename field or whatnot

    let id_field: Option<ValidatedSchemaIdField> = parent_type
        .id_field
        .map(|id_field_id| schema.id_field(id_field_id));

    // If the type has an id field, we must select it.
    if let Some(id_field) = id_field {
        match merged_selection_map.entry(NormalizationKey::Id) {
            Entry::Occupied(occupied) => {
                match occupied.get() {
                    MergedServerSelection::ScalarField(_) => {
                        // TODO check that the existing server field matches the one we
                        // would create.
                    }
                    MergedServerSelection::LinkedField(_) => {
                        panic!("Unexpected linked field for id, probably a bug in Isograph");
                    }
                    MergedServerSelection::InlineFragment(_) => {
                        panic!("Unexpected inline fragment, probably a bug in Isograph");
                    }
                };
            }
            Entry::Vacant(vacant_entry) => {
                vacant_entry.insert(MergedServerSelection::ScalarField(
                    MergedScalarFieldSelection {
                        // major HACK alert
                        name: id_field.name.item.lookup().intern().into(),
                        arguments: vec![],
                        // This indicates that there should be a separate MergedServerFieldSelection variant
                        normalization_alias: None,
                    },
                ));
            }
        }
    }
}

pub fn selection_set_wrapped(
    mut merged_selection_set: MergedSelectionMap,
    top_level_field: LinkedFieldName,
    top_level_field_arguments: Vec<SelectionFieldArgument>,
    // TODO support arguments and vectors of subfields
    subfield: Option<LinkedFieldName>,
    type_to_refine_to: RequiresRefinement,
) -> MergedSelectionMap {
    // We are proceeding inside out, i.e. creating
    // `mutation_name { subfield { ...on Type { existing_selection_set }}}`
    // first by creating the inline fragment, then subfield, etc.

    // Should we wrap the selection set in a type to refine to?
    let selection_set_with_inline_fragment = match type_to_refine_to {
        RequiresRefinement::Yes(type_to_refine_to) => {
            maybe_add_typename_selection(&mut merged_selection_set);
            let mut map = BTreeMap::new();
            map.insert(
                NormalizationKey::InlineFragment(type_to_refine_to),
                MergedServerSelection::InlineFragment(MergedInlineFragmentSelection {
                    type_to_refine_to,
                    selection_map: merged_selection_set,
                }),
            );
            map
        }
        RequiresRefinement::No => merged_selection_set,
    };

    let selection_set_with_subfield = match subfield {
        Some(subfield) => {
            let mut map = BTreeMap::new();
            map.insert(
                NormalizationKey::ServerField(NameAndArguments {
                    name: subfield.into(),
                    arguments: vec![],
                }),
                MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                    name: subfield,
                    // TODO
                    normalization_alias: None,
                    selection_map: selection_set_with_inline_fragment,
                    arguments: vec![],
                }),
            );
            map
        }
        None => selection_set_with_inline_fragment,
    };

    let mut top_level_selection_set = BTreeMap::new();
    top_level_selection_set.insert(
        NormalizationKey::ServerField(NameAndArguments {
            name: top_level_field.into(),
            // TODO provide arguments. They don't matter, because there is only one
            // selection at this level.
            arguments: vec![],
        }),
        MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
            name: top_level_field,
            normalization_alias: Some(
                get_aliased_mutation_field_name(top_level_field.into(), &top_level_field_arguments)
                    .intern()
                    .into(),
            ),

            selection_map: selection_set_with_subfield,
            arguments: top_level_field_arguments,
        }),
    );

    top_level_selection_set
}

fn is_typename_selection<'a>(selection: &'a &MergedServerSelection) -> bool {
    if let MergedServerSelection::ScalarField(s) = &selection {
        s.name == *TYPENAME_FIELD_NAME
    } else {
        false
    }
}

fn maybe_add_typename_selection(selections: &mut MergedSelectionMap) {
    let has_typename = selections.values().find(is_typename_selection).is_some();
    if !has_typename {
        // This should be first, so this a huge bummer
        selections.insert(
            NormalizationKey::Discriminator,
            MergedServerSelection::ScalarField(MergedScalarFieldSelection {
                name: *TYPENAME_FIELD_NAME,
                normalization_alias: None,
                arguments: vec![],
            }),
        );
    }
}

fn get_aliased_mutation_field_name(
    name: SelectableFieldName,
    parameters: &[SelectionFieldArgument],
) -> String {
    let mut s = name.to_string();

    for param in parameters.iter() {
        // TODO NonConstantValue will format to a string like "$name", but we want just "name".
        // There is probably a better way to do this.
        s.push_str("____");
        s.push_str(&param.to_alias_str_chunk());
    }
    s
}
