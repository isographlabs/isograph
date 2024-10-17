use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, HashSet};

use common_lang_types::{
    IsographObjectTypeName, LinkedFieldName, Location, QueryOperationName, ScalarFieldName,
    SelectableFieldName, Span, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldId, IsographSelectionVariant, NonConstantValue,
    RefetchQueryIndex, SelectableServerFieldId, Selection, SelectionFieldArgument,
    ServerFieldSelection, ServerObjectId, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    categorize_field_loadability, create_transformed_name_and_arguments,
    expose_field_directive::RequiresRefinement, transform_arguments_with_child_context,
    transform_name_and_arguments_with_child_variable_context, FieldDefinitionLocation,
    ImperativelyLoadedFieldVariant, Loadability, NameAndArguments, PathToRefetchField,
    RootOperationName, SchemaObject, UnvalidatedVariableDefinition, ValidatedClientField,
    ValidatedIsographSelectionVariant, ValidatedScalarFieldSelection, ValidatedSchema,
    ValidatedSchemaIdField, ValidatedSelection, VariableContext,
};

pub type MergedSelectionMap = BTreeMap<NormalizationKey, MergedServerSelection>;

// Maybe this should be FNVHashMap? We don't really need stable iteration order
pub type ClientFieldToCompletedMergeTraversalStateMap =
    BTreeMap<ClientFieldId, ClientFieldTraversalResult>;

#[derive(Clone, Debug)]
pub struct ClientFieldTraversalResult {
    pub traversal_state: ScalarClientFieldTraversalState,
    /// This is used to generate the normalization AST and query text
    pub merged_selection_map: MergedSelectionMap,
    // TODO change this to Option<SelectionSet>?
    pub was_ever_selected_loadably: bool,
}

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

fn get_variables(arguments: &[ArgumentKeyAndValue]) -> impl Iterator<Item = VariableName> + '_ {
    arguments.iter().flat_map(|arg| match arg.value {
        isograph_lang_types::NonConstantValue::Variable(v) => Some(v),
        _ => None,
    })
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    // TODO no location
    pub name: ScalarFieldName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

impl MergedScalarFieldSelection {
    pub fn normalization_alias(&self) -> Option<String> {
        // None if the alias is the same as the name (i.e. there are no args)
        if self.arguments.is_empty() {
            None
        } else {
            Some(get_aliased_mutation_field_name(
                self.name.into(),
                &self.arguments,
            ))
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MergedLinkedFieldSelection {
    // TODO no location
    pub name: LinkedFieldName,
    pub selection_map: MergedSelectionMap,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

impl MergedLinkedFieldSelection {
    pub fn normalization_alias(&self) -> Option<String> {
        // None if the alias is the same as the name (i.e. there are no args)
        if self.arguments.is_empty() {
            None
        } else {
            Some(get_aliased_mutation_field_name(
                self.name.into(),
                &self.arguments,
            ))
        }
    }
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
    // TODO this should not have NameAndArguments, but LinkedFieldNameAndArguments
    ServerField(NameAndArguments),
    InlineFragment(IsographObjectTypeName),
}

impl NormalizationKey {
    fn transform_with_parent_variable_context(
        &self,
        parent_variable_context: &VariableContext,
    ) -> Self {
        // from_selection_field_argument_and_context(arg, variable_context)
        match &self {
            NormalizationKey::Discriminator => NormalizationKey::Discriminator,
            NormalizationKey::Id => NormalizationKey::Id,
            NormalizationKey::ServerField(s) => NormalizationKey::ServerField(
                transform_name_and_arguments_with_child_variable_context(
                    s.clone(),
                    parent_variable_context,
                ),
            ),
            NormalizationKey::InlineFragment(o) => NormalizationKey::InlineFragment(*o),
        }
    }
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
    pub client_field_id: ClientFieldId,
}

pub type RefetchedPathsMap =
    BTreeMap<(PathToRefetchField, IsographSelectionVariant), RootRefetchedPath>;

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
    pub refetch_paths: RefetchedPathsMap,

    // TODO this should not be NormalizationKey, since a NormalizationKey can represent
    // a scalar field, and a path cannot include scalar fields. So, it should be a two-
    // variant enum, with variants for linked field and for inline fragment.
    /// The (mutable) path from the current client field to wherever we are iterating
    traversal_path: Vec<NormalizationKey>,

    /// Client fields that are directly accessed by this client field
    pub accessible_client_fields: HashSet<ClientFieldId>,
}

impl ScalarClientFieldTraversalState {
    fn new() -> Self {
        Self {
            refetch_paths: BTreeMap::new(),
            traversal_path: vec![],
            accessible_client_fields: HashSet::new(),
        }
    }

    // TODO should this be two separate functions?
    fn incorporate_results_of_iterating_into_child(
        &mut self,
        child_traversal_state: &ScalarClientFieldTraversalState,
        transformed_child_variable_context: &VariableContext,
    ) {
        // TODO self.path_since_client_field should be a parameter to this function
        self.refetch_paths
            .extend(child_traversal_state.refetch_paths.iter().map(
                |((untransformed_path_in_child, selection_variant), root_refetched_path)| {
                    let mut path = untransformed_path_in_child.clone();

                    // self.traversal_path is already transformed, i.e. uses the correct variables
                    let mut complete_path = self.traversal_path.clone();

                    complete_path.extend(path.linked_fields.into_iter().map(|normalization_key| {
                        normalization_key.transform_with_parent_variable_context(
                            transformed_child_variable_context,
                        )
                    }));
                    path.linked_fields = complete_path;

                    ((path, *selection_variant), root_refetched_path.clone())
                },
            ));
    }
}

// This is unused, and should be deleted. It's unused because:
// we already have passed the correct nested selection map, so we don't need
// to follow the traversal path from the "root" selection map to the nested
// one
pub fn current_target_merged_selections<'a>(
    traversal_path: &[NormalizationKey],
    mut parent_selection_map: &'a MergedSelectionMap,
) -> &'a MergedSelectionMap {
    for normalization_key in traversal_path {
        match parent_selection_map
            .get(normalization_key)
            .expect("Expected linked field to exist by now. This is indicate of a bug in Isograph.")
        {
            MergedServerSelection::ScalarField(_) => {
                panic!("Expected a linked field, found scalar. This is indicative of a bug in Isograph.")
            }
            MergedServerSelection::LinkedField(ref linked_field) => {
                parent_selection_map = &linked_field.selection_map;
            }
            MergedServerSelection::InlineFragment(ref inline_fragment) => {
                parent_selection_map = &inline_fragment.selection_map;
            }
        }
    }
    parent_selection_map
}

fn transform_and_merge_child_selection_map_into_parent_map(
    parent_map: &mut MergedSelectionMap,
    untransformed_child_map: &MergedSelectionMap,
    parent_variable_context: &VariableContext,
) {
    for (normalization_key, new_server_field_selection) in untransformed_child_map.iter() {
        let transformed_normalization_key =
            normalization_key.transform_with_parent_variable_context(parent_variable_context);

        match parent_map.entry(transformed_normalization_key.clone()) {
            Entry::Vacant(vacant) => {
                let selection = new_server_field_selection.clone();
                let transformed = match selection {
                    MergedServerSelection::ScalarField(scalar_field_selection) => {
                        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
                            name: scalar_field_selection.name,
                            arguments: transform_arguments_with_child_context(
                                scalar_field_selection.arguments.into_iter(),
                                parent_variable_context,
                            ),
                        })
                    }
                    MergedServerSelection::LinkedField(linked_field_selection) => {
                        MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                            name: linked_field_selection.name,
                            selection_map: transform_child_map_with_parent_context(
                                &linked_field_selection.selection_map,
                                parent_variable_context,
                            ),
                            arguments: transform_arguments_with_child_context(
                                linked_field_selection.arguments.into_iter(),
                                parent_variable_context,
                            ),
                        })
                    }
                    MergedServerSelection::InlineFragment(inline_fragment_selection) => {
                        MergedServerSelection::InlineFragment(MergedInlineFragmentSelection {
                            type_to_refine_to: inline_fragment_selection.type_to_refine_to,
                            selection_map: transform_child_map_with_parent_context(
                                &inline_fragment_selection.selection_map,
                                parent_variable_context,
                            ),
                        })
                    }
                };
                vacant.insert(transformed);
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
                            transform_and_merge_child_selection_map_into_parent_map(
                                &mut target_linked_field.selection_map,
                                &child_linked_field.selection_map,
                                parent_variable_context,
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
                            transform_and_merge_child_selection_map_into_parent_map(
                                &mut target_inline_fragment.selection_map,
                                &child_inline_fragment.selection_map,
                                parent_variable_context,
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

fn transform_child_map_with_parent_context(
    selection_map: &MergedSelectionMap,
    parent_variable_context: &VariableContext,
) -> BTreeMap<NormalizationKey, MergedServerSelection> {
    let mut transformed_child_map = BTreeMap::new();
    transform_and_merge_child_selection_map_into_parent_map(
        &mut transformed_child_map,
        selection_map,
        parent_variable_context,
    );
    transformed_child_map
}

pub fn create_merged_selection_map_for_client_field_and_insert_into_global_map(
    schema: &ValidatedSchema,
    parent_type: &SchemaObject,
    validated_selections: &[WithSpan<ValidatedSelection>],
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
    root_client_field: &ValidatedClientField,
    variable_context: &VariableContext,
    // TODO return Cow?
) -> ClientFieldTraversalResult {
    // TODO move this check outside of this function
    match global_client_field_map.get_mut(&root_client_field.id) {
        Some(traversal_result) => traversal_result.clone(),
        None => {
            let mut merge_traversal_state = ScalarClientFieldTraversalState::new();
            let merged_selection_map = create_selection_map_with_merge_traversal_state(
                schema,
                parent_type,
                validated_selections,
                &mut merge_traversal_state,
                global_client_field_map,
                variable_context,
            );

            // N.B. global_client_field_map might actually have an item stored in root_object.id,
            // if we have some sort of recursion. That probably stack overflows right now.
            global_client_field_map.insert(
                root_client_field.id,
                ClientFieldTraversalResult {
                    traversal_state: merge_traversal_state.clone(),
                    merged_selection_map: merged_selection_map.clone(),
                    was_ever_selected_loadably: false,
                },
            );

            // TODO we don't always use this return value, so we shouldn't always clone above
            ClientFieldTraversalResult {
                traversal_state: merge_traversal_state,
                merged_selection_map,
                was_ever_selected_loadably: false,
            }
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
        client_field_id,
    } = path_to_refetch_field_info;

    let client_field = schema.client_field(client_field_id);

    process_imperatively_loaded_field(
        schema,
        imperatively_loaded_field_variant,
        refetch_field_parent_id,
        nested_selection_map,
        entrypoint,
        index,
        reachable_variables,
        client_field,
    )
}

pub fn get_reachable_variables(selection_map: &MergedSelectionMap) -> BTreeSet<VariableName> {
    selection_map
        .values()
        .flat_map(|x| x.reachable_variables())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn process_imperatively_loaded_field(
    schema: &ValidatedSchema,
    variant: ImperativelyLoadedFieldVariant,
    refetch_field_parent_id: ServerObjectId,
    selection_map: &MergedSelectionMap,
    entrypoint: &ValidatedClientField,
    index: usize,
    reachable_variables: &BTreeSet<VariableName>,
    client_field: &ValidatedClientField,
) -> ImperativelyLoadedFieldArtifactInfo {
    let ImperativelyLoadedFieldVariant {
        client_field_scalar_selection_name,
        top_level_schema_field_name,
        top_level_schema_field_arguments,
        primary_field_info,
        root_object_id,
    } = variant;
    // This could be Pet
    let refetch_field_parent_type = schema.server_field_data.object(refetch_field_parent_id);

    let mut definitions_of_used_variables =
        get_used_variable_definitions(reachable_variables, client_field);

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
    let wrapped_selection_map = selection_map_wrapped(
        selection_map.clone(),
        // TODO why are these types different
        top_level_schema_field_name.lookup().intern().into(),
        top_level_schema_field_arguments
            .iter()
            // TODO don't clone
            .cloned()
            .map(|x| {
                let variable_name = x.name;
                definitions_of_used_variables.push(WithSpan {
                    item: VariableDefinition {
                        name: variable_name,
                        type_: x.type_.clone().map(|type_name| {
                            *schema
                                .server_field_data
                                .defined_types
                                .get(&type_name)
                                .expect(
                                    "Expected type to be found, \
                                    this indicates a bug in Isograph",
                                )
                        }),
                        default_value: x.default_value,
                    },
                    span: Span::todo_generated(),
                });

                ArgumentKeyAndValue {
                    key: x.name.item.lookup().intern().into(),
                    value: NonConstantValue::Variable(x.name.item),
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
                        .unwrap_or_else(|| {
                            panic!(
                                "Did not find matching variable definition. \
                                This might not be validated yet. For now, each client field \
                                containing a __refetch field must re-defined all used variables. \
                                Client field {} is missing variable definition {}",
                                entrypoint.name, variable_name
                            )
                        })
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
    variable_context: &VariableContext,
) -> MergedSelectionMap {
    let mut merged_selection_map = BTreeMap::new();
    merge_validated_selections_into_selection_map(
        schema,
        &mut merged_selection_map,
        parent_type,
        validated_selections,
        merge_traversal_state,
        global_client_field_map,
        variable_context,
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
    variable_context: &VariableContext,
) {
    for validated_selection in validated_selections.iter().filter(filter_id_fields) {
        match &validated_selection.item {
            Selection::ServerField(validated_server_field) => {
                match validated_server_field {
                    ServerFieldSelection::ScalarField(scalar_field_selection) => {
                        match &scalar_field_selection.associated_data.location {
                            FieldDefinitionLocation::Server(_) => {
                                merge_scalar_server_field(
                                    scalar_field_selection,
                                    parent_map,
                                    variable_context,
                                );
                            }
                            FieldDefinitionLocation::Client(client_field_id) => {
                                let newly_encountered_scalar_client_field =
                                    schema.client_field(*client_field_id);

                                // If the field is selected loadably or is imperative, we must note the refetch path,
                                // because this results in an artifact being generated.
                                match categorize_field_loadability(
                                    newly_encountered_scalar_client_field,
                                    &scalar_field_selection.associated_data.selection_variant,
                                ) {
                                    Some(Loadability::LoadablySelectedField(_loadable_variant)) => {
                                        create_merged_selection_map_for_client_field_and_insert_into_global_map(
                                            schema,
                                            parent_type,
                                            newly_encountered_scalar_client_field.selection_set_for_parent_query(),
                                            global_client_field_map,
                                            newly_encountered_scalar_client_field,
                                            &newly_encountered_scalar_client_field.initial_variable_context(),
                                        );

                                        let state = global_client_field_map
                                            .get_mut(client_field_id)
                                            .expect(
                                                "Expected field to exist when \
                                                it is encountered loadably",
                                            );
                                        state.was_ever_selected_loadably = true;
                                    }
                                    Some(Loadability::ImperativelyLoadedField(variant)) => {
                                        insert_imperative_field_into_refetch_paths(
                                            schema,
                                            global_client_field_map,
                                            merge_traversal_state,
                                            newly_encountered_scalar_client_field,
                                            parent_type,
                                            variant,
                                        );
                                    }
                                    None => merge_non_loadable_scalar_client_field(
                                        parent_type,
                                        schema,
                                        parent_map,
                                        merge_traversal_state,
                                        newly_encountered_scalar_client_field,
                                        global_client_field_map,
                                        variable_context,
                                        &scalar_field_selection.arguments,
                                    ),
                                }

                                merge_traversal_state
                                    .accessible_client_fields
                                    .insert(*client_field_id);
                            }
                        };
                    }
                    ServerFieldSelection::LinkedField(linked_field_selection) => {
                        let normalization_key = create_transformed_name_and_arguments(
                            linked_field_selection.name.item.into(),
                            &linked_field_selection.arguments,
                            variable_context,
                        )
                        .normalization_key();

                        merge_traversal_state
                            .traversal_path
                            .push(normalization_key.clone());

                        // We are creating the linked field, and inserting it into the parent object
                        // first, because otherwise, when we try to merge the results into the parent
                        // selection_map, we find that the linked field we are about to insert is
                        // missing, and panic.
                        //
                        // This might be indicative of poor modeling.
                        let linked_field =
                            parent_map.entry(normalization_key).or_insert_with(|| {
                                MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                                    name: linked_field_selection.name.item,
                                    selection_map: BTreeMap::new(),
                                    arguments: transform_arguments_with_child_context(
                                        linked_field_selection
                                            .arguments
                                            .iter()
                                            .map(|arg| arg.item.into_key_and_value()),
                                        variable_context,
                                    ),
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
                                let type_id =
                                    linked_field_selection.associated_data.parent_object_id;
                                let linked_field_parent_type =
                                    schema.server_field_data.object(type_id);

                                merge_validated_selections_into_selection_map(
                                    schema,
                                    &mut existing_linked_field.selection_map,
                                    linked_field_parent_type,
                                    &linked_field_selection.selection_set,
                                    merge_traversal_state,
                                    global_client_field_map,
                                    variable_context,
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

fn insert_imperative_field_into_refetch_paths(
    schema: &ValidatedSchema,
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_scalar_client_field: &ValidatedClientField,
    parent_type: &SchemaObject,
    variant: &ImperativelyLoadedFieldVariant,
) {
    let path = PathToRefetchField {
        linked_fields: merge_traversal_state.traversal_path.clone(),
        field_name: newly_encountered_scalar_client_field.name,
    };
    let info = PathToRefetchFieldInfo {
        refetch_field_parent_id: parent_type.id,
        imperatively_loaded_field_variant: variant.clone(),
        extra_selections: BTreeMap::new(),
        client_field_id: newly_encountered_scalar_client_field.id,
    };
    merge_traversal_state.refetch_paths.insert(
        (path, IsographSelectionVariant::Regular),
        RootRefetchedPath {
            field_name: newly_encountered_scalar_client_field.name,
            path_to_refetch_field_info: info,
        },
    );

    // Generate a merged selection set, but using the refetch strategy
    create_merged_selection_map_for_client_field_and_insert_into_global_map(
        schema,
        parent_type,
        newly_encountered_scalar_client_field
            .refetch_strategy
            .as_ref()
            .expect(
                "Expected refetch strategy. \
                    This is indicative of a bug in Isograph.",
            )
            .refetch_selection_set(),
        global_client_field_map,
        newly_encountered_scalar_client_field,
        &newly_encountered_scalar_client_field.initial_variable_context(),
    );
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

#[allow(clippy::too_many_arguments)]
fn merge_non_loadable_scalar_client_field(
    parent_type: &SchemaObject,
    schema: &ValidatedSchema,
    parent_map: &mut MergedSelectionMap,
    parent_merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_scalar_client_field: &ValidatedClientField,
    global_client_field_map: &mut ClientFieldToCompletedMergeTraversalStateMap,
    parent_variable_context: &VariableContext,
    selection_arguments: &[WithLocation<SelectionFieldArgument>],
) {
    // Here, we are doing a bunch of work, just so that we can have the refetched paths,
    // which is really really silly.
    let ClientFieldTraversalResult {
        traversal_state,
        merged_selection_map: child_merged_selection_map,
        ..
    } = create_merged_selection_map_for_client_field_and_insert_into_global_map(
        schema,
        parent_type,
        newly_encountered_scalar_client_field
            .reader_selection_set
            .as_ref()
            .expect(
                "Expected selection set to exist. \
                This is indicative of a bug in Isograph.",
            ),
        global_client_field_map,
        newly_encountered_scalar_client_field,
        &newly_encountered_scalar_client_field.initial_variable_context(),
    );

    let transformed_child_variable_context = parent_variable_context.child_variable_context(
        selection_arguments,
        &newly_encountered_scalar_client_field.variable_definitions,
        &ValidatedIsographSelectionVariant::Regular,
    );
    transform_and_merge_child_selection_map_into_parent_map(
        parent_map,
        &child_merged_selection_map,
        &transformed_child_variable_context,
    );
    parent_merge_traversal_state.incorporate_results_of_iterating_into_child(
        &traversal_state,
        &transformed_child_variable_context,
    );
}

fn merge_scalar_server_field(
    scalar_field: &ValidatedScalarFieldSelection,
    parent_map: &mut MergedSelectionMap,
    variable_context: &VariableContext,
) {
    let normalization_key = NormalizationKey::ServerField(create_transformed_name_and_arguments(
        scalar_field.name.item.into(),
        &scalar_field.arguments,
        variable_context,
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
                    arguments: transform_arguments_with_child_context(
                        scalar_field
                            .arguments
                            .iter()
                            .map(|arg| arg.item.into_key_and_value()),
                        variable_context,
                    ),
                },
            ));
        }
    }
}

fn select_typename_and_id_fields_in_merged_selection(
    schema: &ValidatedSchema,
    merged_selection_map: &mut MergedSelectionMap,
    parent_type: &SchemaObject,
) {
    if parent_type.concrete_type.is_none() {
        maybe_add_typename_selection(merged_selection_map)
    };

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
                    },
                ));
            }
        }
    }
}

pub fn selection_map_wrapped(
    mut inner_selection_map: MergedSelectionMap,
    top_level_field: LinkedFieldName,
    top_level_field_arguments: Vec<ArgumentKeyAndValue>,
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
            maybe_add_typename_selection(&mut inner_selection_map);
            let mut map = BTreeMap::new();
            map.insert(
                NormalizationKey::InlineFragment(type_to_refine_to),
                MergedServerSelection::InlineFragment(MergedInlineFragmentSelection {
                    type_to_refine_to,
                    selection_map: inner_selection_map,
                }),
            );
            map
        }
        RequiresRefinement::No => inner_selection_map,
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
            arguments: top_level_field_arguments.clone(),
        }),
        MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
            name: top_level_field,
            selection_map: selection_set_with_subfield,
            arguments: top_level_field_arguments,
        }),
    );

    top_level_selection_set
}

fn maybe_add_typename_selection(selections: &mut MergedSelectionMap) {
    // If a discriminator exists, this is a no-op
    selections.insert(
        NormalizationKey::Discriminator,
        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
            name: *TYPENAME_FIELD_NAME,
            arguments: vec![],
        }),
    );
}

fn get_aliased_mutation_field_name(
    name: SelectableFieldName,
    parameters: &[ArgumentKeyAndValue],
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

pub fn id_arguments() -> Vec<UnvalidatedVariableDefinition> {
    vec![VariableDefinition {
        name: WithLocation::new("id".intern().into(), Location::generated()),
        type_: GraphQLTypeAnnotation::NonNull(Box::new(GraphQLNonNullTypeAnnotation::Named(
            GraphQLNamedTypeAnnotation(WithSpan::new("ID".intern().into(), Span::todo_generated())),
        ))),
        default_value: None,
    }]
}
