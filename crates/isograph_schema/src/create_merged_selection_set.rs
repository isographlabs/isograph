use std::collections::{btree_map::Entry, BTreeMap, BTreeSet, HashSet};

use common_lang_types::{
    ClientObjectSelectableName, ClientScalarSelectableName, Location, QueryOperationName,
    ScalarSelectableName, SelectableName, ServerObjectEntityName, ServerObjectSelectableName,
    ServerScalarEntityName, ServerScalarSelectableName, Span, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation, GraphQLTypeAnnotation,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, DefinitionLocation, EmptyDirectiveSet, NonConstantValue,
    ObjectSelectionDirectiveSet, RefetchQueryIndex, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionFieldArgument, SelectionType, SelectionTypeContainingSelections, VariableDefinition,
};
use lazy_static::lazy_static;

use crate::{
    create_transformed_name_and_arguments,
    field_loadability::{categorize_field_loadability, Loadability},
    initial_variable_context, transform_arguments_with_child_context,
    transform_name_and_arguments_with_child_variable_context, ClientFieldVariant,
    ClientOrServerObjectSelectable, ClientScalarOrObjectSelectable, ClientScalarSelectable,
    ClientSelectable, ClientSelectableId, ImperativelyLoadedFieldVariant, NameAndArguments,
    NetworkProtocol, PathToRefetchField, RootOperationName, Schema, ServerEntityName,
    ServerObjectEntity, ServerObjectEntityExtraInfo, ServerObjectSelectable,
    ServerObjectSelectableVariant, ValidatedObjectSelection, ValidatedScalarSelection,
    ValidatedSelection, VariableContext,
};

pub type MergedSelectionMap = BTreeMap<NormalizationKey, MergedServerSelection>;

// Maybe this should be FNVHashMap? We don't really need stable iteration order
pub type FieldToCompletedMergeTraversalStateMap = BTreeMap<
    DefinitionLocation<(ServerObjectEntityName, ServerObjectSelectableName), ClientSelectableId>,
    FieldTraversalResult,
>;

#[derive(Clone, Debug)]
pub struct FieldTraversalResult {
    pub traversal_state: ScalarClientFieldTraversalState,
    /// This is used to generate the normalization AST and query text
    pub merged_selection_map: MergedSelectionMap,
    // TODO change this to Option<SelectionSet>?
    pub was_ever_selected_loadably: bool,
}

lazy_static! {
    pub static ref REFETCH_FIELD_NAME: ClientScalarSelectableName = "__refetch".intern().into();
    pub static ref NODE_FIELD_NAME: ServerObjectSelectableName = "node".intern().into();
    pub static ref TYPENAME_FIELD_NAME: ServerScalarSelectableName = "__typename".intern().into();
    pub static ref LINK_FIELD_NAME: ClientScalarSelectableName = "link".intern().into();
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootRefetchedPath {
    pub field_name: ClientScalarSelectableName,
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
        // TODO handle variables in objects and lists
        _ => None,
    })
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    pub name: ScalarSelectableName,
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
    pub name: ServerObjectSelectableName,
    pub selection_map: MergedSelectionMap,
    pub arguments: Vec<ArgumentKeyAndValue>,
    /// Some if the object is concrete; None otherwise.
    pub concrete_type: Option<ServerObjectEntityName>,
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
    pub type_to_refine_to: ServerObjectEntityName,
    // TODO make this type more precise, this selection map should not contain inline fragments
    pub selection_map: MergedSelectionMap,
}

#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord, Hash)]
pub enum NormalizationKey {
    Discriminator, // AKA typename
    Id,
    // TODO this should not have NameAndArguments, but LinkedFieldNameAndArguments
    ServerField(NameAndArguments),
    InlineFragment(ServerObjectEntityName),
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
    pub variable_definitions: Vec<VariableDefinition<ServerEntityName>>,
    pub root_parent_object: ServerObjectEntityName,
    pub root_fetchable_field: ClientScalarSelectableName,
    pub refetch_query_index: RefetchQueryIndex,

    pub root_operation_name: RootOperationName,
    pub query_name: QueryOperationName,
    pub concrete_type: ServerObjectEntityName,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathToRefetchFieldInfo {
    pub refetch_field_parent_object_entity_name: ServerObjectEntityName,
    pub imperatively_loaded_field_variant: ImperativelyLoadedFieldVariant,
    extra_selections: MergedSelectionMap,
    pub client_field_name: ClientScalarSelectableName,
}

pub type RefetchedPathsMap =
    BTreeMap<(PathToRefetchField, ScalarSelectionDirectiveSet), RootRefetchedPath>;

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
    pub accessible_client_fields: HashSet<ClientSelectableId>,
    pub has_updatable: bool,
}

impl ScalarClientFieldTraversalState {
    fn new() -> Self {
        Self {
            refetch_paths: BTreeMap::new(),
            traversal_path: vec![],
            accessible_client_fields: HashSet::new(),
            has_updatable: false,
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
                            concrete_type: linked_field_selection.concrete_type,
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

pub fn create_merged_selection_map_for_field_and_insert_into_global_map<
    TNetworkProtocol: NetworkProtocol,
>(
    schema: &Schema<TNetworkProtocol>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    validated_selections: &[WithSpan<ValidatedSelection>],
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    root_field_id: DefinitionLocation<
        (ServerObjectEntityName, ServerObjectSelectableName),
        ClientSelectableId,
    >,
    variable_context: &VariableContext,
    // TODO return Cow?
) -> FieldTraversalResult {
    // TODO move this check outside of this function

    match encountered_client_type_map.get_mut(&root_field_id) {
        Some(traversal_result) => traversal_result.clone(),
        None => {
            let field_traversal_result = create_field_traversal_result(
                schema,
                parent_object_entity,
                validated_selections,
                encountered_client_type_map,
                variable_context,
            );

            // N.B. encountered_client_type_map might actually have an item stored in root_object.id,
            // if we have some sort of recursion. That probably stack overflows right now.
            encountered_client_type_map.insert(root_field_id, field_traversal_result.clone());

            field_traversal_result
        }
    }
}

fn create_field_traversal_result<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    validated_selections: &[WithSpan<ValidatedSelection>],
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
) -> FieldTraversalResult {
    let mut traversal_state = ScalarClientFieldTraversalState::new();
    let mut merged_selection_map = BTreeMap::new();

    merge_validated_selections_into_selection_map(
        schema,
        &mut merged_selection_map,
        parent_object_entity,
        validated_selections,
        &mut traversal_state,
        encountered_client_type_map,
        variable_context,
    );

    FieldTraversalResult {
        traversal_state,
        merged_selection_map,
        was_ever_selected_loadably: false,
    }
}

pub fn get_imperatively_loaded_artifact_info<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    entrypoint: &ClientScalarSelectable<TNetworkProtocol>,
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
        refetch_field_parent_object_entity_name,
        imperatively_loaded_field_variant,
        extra_selections: _,
        client_field_name,
    } = path_to_refetch_field_info;

    let client_scalar_selectable = schema
        .client_field(refetch_field_parent_object_entity_name, client_field_name)
        .expect(
            "Expected selectable to exist. \
                This is indicative of a bug in Isograph.",
        );

    process_imperatively_loaded_field(
        schema,
        imperatively_loaded_field_variant,
        refetch_field_parent_object_entity_name,
        nested_selection_map,
        entrypoint,
        index,
        reachable_variables,
        client_scalar_selectable,
    )
}

pub fn get_reachable_variables(selection_map: &MergedSelectionMap) -> BTreeSet<VariableName> {
    selection_map
        .values()
        .flat_map(|x| x.reachable_variables())
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn process_imperatively_loaded_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    variant: ImperativelyLoadedFieldVariant,
    // ID of e.g. Pet, Checkin, etc. i.e. the field on which e.g. __refetch or make_super is selected
    refetch_field_parent_object_entity_name: ServerObjectEntityName,
    selection_map: &MergedSelectionMap,
    entrypoint: &ClientScalarSelectable<TNetworkProtocol>,
    index: usize,
    reachable_variables: &BTreeSet<VariableName>,
    client_scalar_selectable: &ClientScalarSelectable<TNetworkProtocol>,
) -> ImperativelyLoadedFieldArtifactInfo {
    let ImperativelyLoadedFieldVariant {
        client_field_scalar_selection_name,
        root_object_entity_name,
        mut subfields_or_inline_fragments,
        top_level_schema_field_arguments,
        ..
    } = variant;

    // If the field (e.g. icheckin) returns an abstract type (ICheckin) that is different than
    // the concrete type we want (Checkin), then we refine to that concrete type.
    // TODO investigate whether this can be done when the ImperativelyLoadedFieldVariant is created
    if refetch_field_parent_object_entity_name != client_scalar_selectable.parent_object_entity_name
    {
        let refetch_field_parent_type_name = schema
            .server_entity_data
            .server_object_entity(refetch_field_parent_object_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name;
        // This could be Pet
        subfields_or_inline_fragments.insert(
            0,
            WrappedSelectionMapSelection::InlineFragment(refetch_field_parent_type_name.item),
        );
    }

    // TODO we need to extend this with variables used in subfields_or_inline_fragments
    let mut definitions_of_used_variables =
        get_used_variable_definitions(reachable_variables, client_scalar_selectable);

    for variable_definition in top_level_schema_field_arguments.iter() {
        definitions_of_used_variables.push(VariableDefinition {
            name: variable_definition.name,
            type_: variable_definition.type_.clone(),
            default_value: variable_definition.default_value.clone(),
        });
    }

    let wrapped_selection_map =
        selection_map_wrapped(selection_map.clone(), subfields_or_inline_fragments);

    let root_parent_object = schema
        .server_entity_data
        .server_object_entity(entrypoint.parent_object_entity_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        )
        .name;

    let root_operation_name = schema
        .fetchable_types
        .get(&root_object_entity_name)
        .expect(
            "Expected root type to be fetchable here.\
            This is indicative of a bug in Isograph.",
        )
        .clone();

    let query_name = format!("{root_parent_object}__{client_field_scalar_selection_name}")
        .intern()
        .into();

    ImperativelyLoadedFieldArtifactInfo {
        // TODO don't clone, have lifetime parameter
        merged_selection_set: wrapped_selection_map,
        root_parent_object: root_parent_object.item,
        variable_definitions: definitions_of_used_variables,
        root_fetchable_field: entrypoint.name.item,
        refetch_query_index: RefetchQueryIndex(index as u32),
        root_operation_name,
        query_name,
        concrete_type: schema
            .server_entity_data
            .server_object_entity(root_object_entity_name)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name
            .item,
    }
}

pub fn imperative_field_subfields_or_inline_fragments(
    top_level_schema_field_name: ServerObjectSelectableName,
    top_level_schema_field_arguments: &[VariableDefinition<ServerEntityName>],
    top_level_schema_field_concrete_type: Option<ServerObjectEntityName>,
) -> WrappedSelectionMapSelection {
    let top_level_schema_field_arguments = top_level_schema_field_arguments
        .iter()
        // TODO don't clone
        .cloned()
        .map(|variable_definition| ArgumentKeyAndValue {
            key: variable_definition.name.item.unchecked_conversion(),
            value: NonConstantValue::Variable(variable_definition.name.item),
        })
        .collect();

    WrappedSelectionMapSelection::LinkedField {
        server_object_selectable_name: top_level_schema_field_name,
        arguments: top_level_schema_field_arguments,
        concrete_type: top_level_schema_field_concrete_type,
    }
}

fn get_used_variable_definitions<TNetworkProtocol: NetworkProtocol>(
    reachable_variables: &BTreeSet<VariableName>,
    entrypoint: &ClientScalarSelectable<TNetworkProtocol>,
) -> Vec<VariableDefinition<ServerEntityName>> {
    reachable_variables
        .iter()
        .flat_map(|variable_name| {
            // HACK
            if *variable_name == "id" {
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
                        .item
                        .clone(),
                )
            }
        })
        .collect::<Vec<_>>()
}

fn merge_validated_selections_into_selection_map<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_map: &mut MergedSelectionMap,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    validated_selections: &[WithSpan<ValidatedSelection>],
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
) {
    for validated_selection in validated_selections.iter() {
        match &validated_selection.item {
            SelectionType::Scalar(scalar_field_selection) => {
                match &scalar_field_selection.associated_data {
                    DefinitionLocation::Server(_) => {
                        merge_server_scalar_field(
                            scalar_field_selection,
                            parent_map,
                            variable_context,
                            merge_traversal_state,
                        );
                    }
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        newly_encountered_scalar_client_selectable_id,
                    )) => {
                        merge_client_scalar_field(
                            schema,
                            parent_map,
                            parent_object_entity,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            scalar_field_selection,
                            parent_object_entity_name,
                            newly_encountered_scalar_client_selectable_id,
                        );
                    }
                };
            }
            SelectionType::Object(object_selection) => {
                let parent_object_entity_name = *schema
                    .object_selectable(object_selection.associated_data)
                    .expect(
                        "Expected selectable to exist. \
                            This is indicative of a bug in Isograph.",
                    )
                    .target_object_entity_name()
                    .inner();
                let object_selection_parent_object = schema
                    .server_entity_data
                    .server_object_entity(parent_object_entity_name);

                match object_selection.associated_data {
                    DefinitionLocation::Client((
                        parent_object_entity_name,
                        newly_encountered_client_object_selectable_id,
                    )) => {
                        merge_client_object_field(
                            schema,
                            parent_map,
                            parent_object_entity,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            object_selection,
                            parent_object_entity_name,
                            newly_encountered_client_object_selectable_id,
                        );
                    }
                    DefinitionLocation::Server((
                        field_parent_object_entity_name,
                        field_object_selectable_name,
                    )) => {
                        merge_server_object_field(
                            schema,
                            parent_map,
                            parent_object_entity,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            object_selection,
                            parent_object_entity_name,
                            object_selection_parent_object.expect(
                                "Expected entity to exist. \
                                This is indicative of a bug in Isograph.",
                            ),
                            field_parent_object_entity_name,
                            field_object_selectable_name,
                        );
                    }
                }

                merge_traversal_state.traversal_path.pop();
            }
        }
    }

    select_typename_and_id_fields_in_merged_selection(schema, parent_map, parent_object_entity);
}

#[allow(clippy::too_many_arguments)]
fn merge_server_object_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    object_selection: &ValidatedObjectSelection,
    parent_object_entity_name: ServerObjectEntityName,
    object_selection_parent_object: &ServerObjectEntity<TNetworkProtocol>,
    field_parent_object_entity_name: ServerObjectEntityName,
    field_server_object_selectable_name: ServerObjectSelectableName,
) {
    if let ObjectSelectionDirectiveSet::Updatable(_) =
        object_selection.object_selection_directive_set
    {
        merge_traversal_state.has_updatable = true;
    }

    let server_object_selectable = schema
        .server_object_selectable(
            field_parent_object_entity_name,
            field_server_object_selectable_name,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    match &server_object_selectable.object_selectable_variant {
        ServerObjectSelectableVariant::InlineFragment => {
            let type_to_refine_to = object_selection_parent_object.name.item;

            let normalization_key = NormalizationKey::InlineFragment(type_to_refine_to);
            merge_traversal_state
                .traversal_path
                .push(normalization_key.clone());

            let inline_fragment = parent_map.entry(normalization_key).or_insert_with(|| {
                MergedServerSelection::InlineFragment(MergedInlineFragmentSelection {
                    type_to_refine_to,
                    selection_map: BTreeMap::new(),
                })
            });

            match inline_fragment {
                MergedServerSelection::ScalarField(_) => {
                    panic!(
                        "Expected inline fragment, but encountered scalar. \
                        This is indicative of a bug in Isograph."
                    )
                }
                MergedServerSelection::LinkedField(_) => {
                    panic!(
                        "Expected inline fragment, but encountered linked field. \
                        This is indicative of a bug in Isograph."
                    )
                }
                MergedServerSelection::InlineFragment(existing_inline_fragment) => {
                    let object_selection_parent_object = schema
                        .server_entity_data
                        .server_object_entity(parent_object_entity_name)
                        .expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        );

                    let reader_selection_set =
                        inline_fragment_reader_selection_set(schema, server_object_selectable);
                    merge_validated_selections_into_selection_map(
                        schema,
                        &mut existing_inline_fragment.selection_map,
                        object_selection_parent_object,
                        &reader_selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
                        variable_context,
                    );
                    merge_validated_selections_into_selection_map(
                        schema,
                        &mut existing_inline_fragment.selection_map,
                        object_selection_parent_object,
                        &object_selection.selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
                        variable_context,
                    );

                    create_merged_selection_map_for_field_and_insert_into_global_map(
                        schema,
                        parent_object_entity,
                        &object_selection.selection_set,
                        encountered_client_type_map,
                        DefinitionLocation::Server((
                            field_parent_object_entity_name,
                            field_server_object_selectable_name,
                        )),
                        &server_object_selectable.initial_variable_context(),
                    );
                }
            }
        }
        ServerObjectSelectableVariant::LinkedField => {
            let normalization_key = create_transformed_name_and_arguments(
                object_selection.name.item.into(),
                &object_selection.arguments,
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
            let linked_field = parent_map.entry(normalization_key).or_insert_with(|| {
                MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                    concrete_type: schema
                        .server_entity_data
                        .server_object_entity(
                            *schema
                                .object_selectable(object_selection.associated_data)
                                .expect(
                                    "Expected selectable to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .target_object_entity_name()
                                .inner(),
                        )
                        .expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        )
                        .concrete_type,
                    name: object_selection.name.item,
                    selection_map: BTreeMap::new(),
                    arguments: transform_arguments_with_child_context(
                        object_selection
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
                    merge_validated_selections_into_selection_map(
                        schema,
                        &mut existing_linked_field.selection_map,
                        object_selection_parent_object,
                        &object_selection.selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
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
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn merge_client_object_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    object_selection: &ValidatedObjectSelection,
    parent_object_entity_name: ServerObjectEntityName,
    newly_encountered_client_object_selectable_id: ClientObjectSelectableName,
) {
    let newly_encountered_client_object_selectable = schema
        .client_pointer(
            parent_object_entity_name,
            newly_encountered_client_object_selectable_id,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    merge_non_loadable_client_type(
        parent_object_entity,
        schema,
        parent_map,
        merge_traversal_state,
        SelectionType::Object((
            parent_object_entity_name,
            newly_encountered_client_object_selectable_id,
        )),
        SelectionType::Object(newly_encountered_client_object_selectable),
        encountered_client_type_map,
        variable_context,
        &object_selection.arguments,
    );

    merge_traversal_state
        .accessible_client_fields
        .insert(SelectionType::Object((
            parent_object_entity_name,
            newly_encountered_client_object_selectable_id,
        )));
}

#[allow(clippy::too_many_arguments)]
fn merge_client_scalar_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    scalar_field_selection: &ValidatedScalarSelection,
    parent_object_entity_name: &ServerObjectEntityName,
    newly_encountered_scalar_client_selectable_id: &ClientScalarSelectableName,
) {
    let newly_encountered_scalar_client_selectable = schema
        .client_field(
            *parent_object_entity_name,
            *newly_encountered_scalar_client_selectable_id,
        )
        .expect(
            "Expected selectable to exist. \
            This is indicative of a bug in Isograph.",
        );

    // If the field is selected loadably or is imperative, we must note the refetch path,
    // because this results in an artifact being generated.
    match categorize_field_loadability(
        newly_encountered_scalar_client_selectable,
        &scalar_field_selection.scalar_selection_directive_set,
    ) {
        Some(Loadability::LoadablySelectedField(_loadable_variant)) => {
            create_merged_selection_map_for_field_and_insert_into_global_map(
                schema,
                parent_object_entity,
                newly_encountered_scalar_client_selectable.selection_set_for_parent_query(),
                encountered_client_type_map,
                DefinitionLocation::Client(SelectionType::Scalar((
                    *parent_object_entity_name,
                    *newly_encountered_scalar_client_selectable_id,
                ))),
                &initial_variable_context(&SelectionType::Scalar(
                    newly_encountered_scalar_client_selectable,
                )),
            );

            let state = encountered_client_type_map
                .get_mut(&DefinitionLocation::Client(SelectionType::Scalar((
                    *parent_object_entity_name,
                    *newly_encountered_scalar_client_selectable_id,
                ))))
                .expect(
                    "Expected field to exist when \
                                                it is encountered loadably",
                );
            state.was_ever_selected_loadably = true;
        }
        Some(Loadability::ImperativelyLoadedField(variant)) => {
            insert_imperative_field_into_refetch_paths(
                schema,
                encountered_client_type_map,
                merge_traversal_state,
                *newly_encountered_scalar_client_selectable_id,
                newly_encountered_scalar_client_selectable,
                *parent_object_entity_name,
                parent_object_entity,
                variant,
            );
        }
        None => match newly_encountered_scalar_client_selectable.variant {
            ClientFieldVariant::Link => {}
            ClientFieldVariant::ImperativelyLoadedField(_) | ClientFieldVariant::UserWritten(_) => {
                merge_non_loadable_client_type(
                    parent_object_entity,
                    schema,
                    parent_map,
                    merge_traversal_state,
                    SelectionType::Scalar((
                        *parent_object_entity_name,
                        *newly_encountered_scalar_client_selectable_id,
                    )),
                    SelectionType::Scalar(newly_encountered_scalar_client_selectable),
                    encountered_client_type_map,
                    variable_context,
                    &scalar_field_selection.arguments,
                )
            }
        },
    }

    merge_traversal_state
        .accessible_client_fields
        .insert(SelectionType::Scalar((
            *parent_object_entity_name,
            *newly_encountered_scalar_client_selectable_id,
        )));
}

#[allow(clippy::too_many_arguments)]
fn insert_imperative_field_into_refetch_paths<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_scalar_client_selectable_name: ClientScalarSelectableName,
    newly_encountered_scalar_client_selectable: &ClientScalarSelectable<TNetworkProtocol>,
    parent_object_entity_name: ServerObjectEntityName,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    variant: &ImperativelyLoadedFieldVariant,
) {
    let path = PathToRefetchField {
        linked_fields: merge_traversal_state.traversal_path.clone(),
        field_name: newly_encountered_scalar_client_selectable.name.item,
    };

    let info = PathToRefetchFieldInfo {
        refetch_field_parent_object_entity_name: parent_object_entity_name,
        imperatively_loaded_field_variant: variant.clone(),
        extra_selections: BTreeMap::new(),
        client_field_name: newly_encountered_scalar_client_selectable_name,
    };

    merge_traversal_state.refetch_paths.insert(
        (
            path,
            ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        ),
        RootRefetchedPath {
            field_name: newly_encountered_scalar_client_selectable.name.item,
            path_to_refetch_field_info: info,
        },
    );

    // Generate a merged selection set, but using the refetch strategy
    create_merged_selection_map_for_field_and_insert_into_global_map(
        schema,
        parent_object_entity,
        newly_encountered_scalar_client_selectable
            .refetch_strategy
            .as_ref()
            .expect(
                "Expected refetch strategy. \
                This is indicative of a bug in Isograph.",
            )
            .refetch_selection_set(),
        encountered_client_type_map,
        DefinitionLocation::Client(SelectionType::Scalar((
            parent_object_entity_name,
            newly_encountered_scalar_client_selectable_name,
        ))),
        &initial_variable_context(&SelectionType::Scalar(
            newly_encountered_scalar_client_selectable,
        )),
    );
}

#[allow(clippy::too_many_arguments)]
fn merge_non_loadable_client_type<TNetworkProtocol: NetworkProtocol>(
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
    schema: &Schema<TNetworkProtocol>,
    parent_map: &mut MergedSelectionMap,
    parent_merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_client_type_id: ClientSelectableId,
    newly_encountered_client_type: ClientSelectable<TNetworkProtocol>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    parent_variable_context: &VariableContext,
    selection_arguments: &[WithLocation<SelectionFieldArgument>],
) {
    // Here, we are doing a bunch of work, just so that we can have the refetched paths,
    // which is really really silly.
    let FieldTraversalResult {
        traversal_state,
        merged_selection_map: child_merged_selection_map,
        ..
    } = create_merged_selection_map_for_field_and_insert_into_global_map(
        schema,
        parent_object_entity,
        newly_encountered_client_type.reader_selection_set(),
        encountered_client_type_map,
        DefinitionLocation::Client(newly_encountered_client_type_id),
        &initial_variable_context(&newly_encountered_client_type),
    );

    let transformed_child_variable_context = parent_variable_context.child_variable_context(
        selection_arguments,
        newly_encountered_client_type.variable_definitions(),
        &ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
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

fn merge_server_scalar_field(
    scalar_field_selection: &ValidatedScalarSelection,
    parent_map: &mut MergedSelectionMap,
    variable_context: &VariableContext,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
) {
    if let ScalarSelectionDirectiveSet::Updatable(_) =
        scalar_field_selection.scalar_selection_directive_set
    {
        merge_traversal_state.has_updatable = true;
    }

    // HACK. We probably should filter these out in a better way.
    let scalar_field_name = scalar_field_selection.name.item;
    let normalization_key = if scalar_field_name == "__typename" {
        NormalizationKey::Discriminator
    } else if scalar_field_name == "id" {
        NormalizationKey::Id
    } else {
        NormalizationKey::ServerField(create_transformed_name_and_arguments(
            scalar_field_name.into(),
            &scalar_field_selection.arguments,
            variable_context,
        ))
    };

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
                    name: scalar_field_name,
                    arguments: transform_arguments_with_child_context(
                        scalar_field_selection
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

fn select_typename_and_id_fields_in_merged_selection<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    merged_selection_map: &mut MergedSelectionMap,
    parent_object_entity: &ServerObjectEntity<TNetworkProtocol>,
) {
    if parent_object_entity.concrete_type.is_none() {
        maybe_add_typename_selection(merged_selection_map)
    };

    // If the type has an id field, we must select it.
    if let Some(id_field) = schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&parent_object_entity.name.item)
        .and_then(|ServerObjectEntityExtraInfo { id_field, .. }| *id_field)
    {
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
                        name: id_field.into(),
                        arguments: vec![],
                    },
                ));
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WrappedSelectionMapSelection {
    LinkedField {
        server_object_selectable_name: ServerObjectSelectableName,
        arguments: Vec<ArgumentKeyAndValue>,
        concrete_type: Option<ServerObjectEntityName>,
    },
    InlineFragment(ServerObjectEntityName),
}

pub fn selection_map_wrapped(
    mut inner_selection_map: MergedSelectionMap,
    // NOTE: these must be in reverse order, e.g. node { ... on Foo { etc } } would be
    // [foo_inline_fragment, node_field_selection]
    subfields_or_inline_fragments: Vec<WrappedSelectionMapSelection>,
) -> MergedSelectionMap {
    // TODO so far we only support regular linked fields, but the goal is to support
    // inline fragments, too.
    // TODO unify this with type_to_refine_to
    for subfield_or_inline_fragment in subfields_or_inline_fragments {
        let mut map = BTreeMap::new();
        match subfield_or_inline_fragment {
            WrappedSelectionMapSelection::LinkedField {
                server_object_selectable_name,
                arguments,
                concrete_type,
            } => {
                map.insert(
                    NormalizationKey::ServerField(NameAndArguments {
                        name: server_object_selectable_name.into(),
                        arguments: arguments.clone(),
                    }),
                    MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                        name: server_object_selectable_name,
                        selection_map: inner_selection_map,
                        arguments,
                        concrete_type,
                    }),
                );
            }
            WrappedSelectionMapSelection::InlineFragment(isograph_object_type_name) => {
                maybe_add_typename_selection(&mut inner_selection_map);
                map.insert(
                    NormalizationKey::InlineFragment(isograph_object_type_name),
                    MergedServerSelection::InlineFragment(MergedInlineFragmentSelection {
                        type_to_refine_to: isograph_object_type_name,
                        selection_map: inner_selection_map,
                    }),
                );
            }
        }
        inner_selection_map = map;
    }

    inner_selection_map
}

fn maybe_add_typename_selection(selections: &mut MergedSelectionMap) {
    // If a discriminator exists, this is a no-op
    selections.insert(
        NormalizationKey::Discriminator,
        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
            name: (*TYPENAME_FIELD_NAME).into(),
            arguments: vec![],
        }),
    );
}

fn get_aliased_mutation_field_name(
    name: SelectableName,
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

pub fn id_arguments(
    id_type_name: ServerScalarEntityName,
) -> Vec<VariableDefinition<ServerEntityName>> {
    vec![VariableDefinition {
        name: WithLocation::new("id".intern().into(), Location::generated()),
        type_: GraphQLTypeAnnotation::NonNull(Box::new(GraphQLNonNullTypeAnnotation::Named(
            GraphQLNamedTypeAnnotation(WithSpan::new(
                SelectionType::Scalar(id_type_name),
                Span::todo_generated(),
            )),
        ))),
        default_value: None,
    }]
}

pub fn inline_fragment_reader_selection_set<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    server_object_selectable: &ServerObjectSelectable<TNetworkProtocol>,
) -> Vec<WithSpan<ValidatedSelection>> {
    let selectables_map = &schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(server_object_selectable.target_object_entity.inner())
        .expect(
            "Expected subtype to exist \
            in server_object_entity_available_selectables",
        )
        .selectables;
    let typename_selection = WithSpan::new(
        SelectionTypeContainingSelections::Scalar(ScalarSelection {
            arguments: vec![],
            scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
            associated_data: DefinitionLocation::Server(
                *selectables_map
                    .get(&"__typename".intern().into())
                    .expect("Expected __typename to exist")
                    .as_server()
                    .as_ref()
                    .expect("Expected __typename to be server field")
                    .as_scalar()
                    .as_ref()
                    .expect("Expected __typename to be scalar"),
            ),
            name: WithLocation::new("__typename".intern().into(), Location::generated()),
            reader_alias: None,
        }),
        Span::todo_generated(),
    );

    let link_selection = WithSpan::new(
        SelectionTypeContainingSelections::Scalar(ScalarSelection {
            arguments: vec![],
            associated_data: DefinitionLocation::Client(
                *selectables_map
                    .get(&(*LINK_FIELD_NAME).into())
                    .expect("Expected link to exist")
                    .as_client()
                    .as_ref()
                    .expect("Expected link to be client field")
                    .as_scalar()
                    .as_ref()
                    .expect("Expected link to be scalar field"),
            ),
            scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
            name: WithLocation::new((*LINK_FIELD_NAME).into(), Location::generated()),
            reader_alias: None,
        }),
        Span::todo_generated(),
    );

    vec![typename_selection, link_selection]
}
