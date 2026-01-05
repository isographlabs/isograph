use std::collections::{BTreeMap, BTreeSet, HashSet, btree_map::Entry};

use common_lang_types::{
    EmbeddedLocation, EntityName, SelectableName, VariableName, WithEmbeddedLocation,
    WithLocationPostfix,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, DefinitionLocation, DefinitionLocationPostfix, EmptyDirectiveSet,
    NonConstantValue, ObjectSelection, ObjectSelectionDirectiveSet, ScalarSelection,
    ScalarSelectionDirectiveSet, SelectionFieldArgument, SelectionSet, SelectionType,
    SelectionTypePostfix, TypeAnnotationDeclaration, VariableDeclaration, VariableNameWrapper,
};
use lazy_static::lazy_static;
use prelude::Postfix;

use crate::{
    ClientFieldVariant, ClientObjectSelectable, ClientScalarSelectable, ClientSelectable,
    ClientSelectableId, CompilationProfile, ID_ENTITY_NAME, ID_FIELD_NAME,
    ImperativelyLoadedFieldVariant, IsographDatabase, NameAndArguments, PathToRefetchField,
    ServerEntity, ServerObjectSelectableVariant, VariableContext, client_object_selectable_named,
    client_scalar_selectable_named, client_scalar_selectable_selection_set_for_parent_query,
    create_transformed_name_and_arguments, fetchable_types,
    field_loadability::{Loadability, categorize_field_loadability},
    initial_variable_context, refetch_strategy_for_client_scalar_selectable_named,
    selectable_named, selectable_reader_selection_set, server_entity_named, server_id_selectable,
    server_selectable_named, transform_arguments_with_child_context,
    transform_name_and_arguments_with_child_variable_context,
};

pub type MergedSelectionMap = BTreeMap<NormalizationKey, MergedServerSelection>;

// Maybe this should be FNVHashMap? We don't really need stable iteration order
pub type FieldToCompletedMergeTraversalStateMap = BTreeMap<
    DefinitionLocation<(EntityName, SelectableName), ClientSelectableId>,
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
    pub static ref REFETCH_FIELD_NAME: SelectableName = "__refetch".intern().into();
    pub static ref NODE_FIELD_NAME: SelectableName = "node".intern().into();
    pub static ref TYPENAME_FIELD_NAME: SelectableName = "__typename".intern().into();
    pub static ref LINK_FIELD_NAME: SelectableName = "__link".intern().into();
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct RootRefetchedPath {
    pub field_name: SelectableName,
    pub path_to_refetch_field_info: PathToRefetchFieldInfo,
}

// TODO add id and typename variants, impl Ord, and get rid of the NormalizationKey enum
#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum MergedServerSelection {
    ScalarField(MergedScalarFieldSelection),
    LinkedField(MergedLinkedFieldSelection),
    ClientObjectSelectable(MergedLinkedFieldSelection),
    // TODO does this belong? This is very GraphQL specific.
    InlineFragment(MergedInlineFragmentSelection),
}

impl MergedServerSelection {
    pub fn reachable_variables(&self) -> BTreeSet<VariableNameWrapper> {
        match self {
            MergedServerSelection::ScalarField(field) => get_variables(&field.arguments).collect(),
            MergedServerSelection::ClientObjectSelectable(field)
            | MergedServerSelection::LinkedField(field) => get_variables(&field.arguments)
                .chain(
                    field
                        .selection_map
                        .values()
                        .flat_map(|x| x.reachable_variables()),
                )
                .collect(),
            MergedServerSelection::InlineFragment(inline_fragment) => inline_fragment
                .selection_map
                .values()
                .flat_map(|selection| selection.reachable_variables())
                .collect(),
        }
    }
}

fn get_variables(
    arguments: &[ArgumentKeyAndValue],
) -> impl Iterator<Item = VariableNameWrapper> + '_ {
    arguments.iter().flat_map(|arg| match arg.value {
        isograph_lang_types::NonConstantValue::Variable(v) => Some(v),
        // TODO handle variables in objects and lists
        _ => None,
    })
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug)]
pub struct MergedScalarFieldSelection {
    pub parent_object_entity_name: EntityName,
    pub name: SelectableName,
    pub arguments: Vec<ArgumentKeyAndValue>,
}

impl MergedScalarFieldSelection {
    pub fn normalization_alias(&self) -> Option<String> {
        // None if the alias is the same as the name (i.e. there are no args)
        if self.arguments.is_empty() {
            None
        } else {
            get_aliased_mutation_field_name(self.name, &self.arguments).wrap_some()
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MergedLinkedFieldSelection {
    pub parent_object_entity_name: EntityName,
    pub name: SelectableName,
    pub selection_map: MergedSelectionMap,
    pub arguments: Vec<ArgumentKeyAndValue>,
    /// None if the target is abstract
    pub concrete_target_entity_name: Option<EntityName>,
}

impl MergedLinkedFieldSelection {
    pub fn normalization_alias(&self) -> Option<String> {
        // None if the alias is the same as the name (i.e. there are no args)
        if self.arguments.is_empty() {
            None
        } else {
            get_aliased_mutation_field_name(self.name, &self.arguments).wrap_some()
        }
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub struct MergedInlineFragmentSelection {
    pub type_to_refine_to: EntityName,
    // TODO make this type more precise, this selection map should not contain inline fragments
    pub selection_map: MergedSelectionMap,
}

#[derive(Debug, Eq, PartialEq, Clone, PartialOrd, Ord, Hash)]
pub enum NormalizationKey {
    Discriminator, // AKA typename
    Id,
    // TODO this should not have NameAndArguments, but LinkedFieldNameAndArguments
    ServerField(NameAndArguments),
    ClientPointer(NameAndArguments),
    InlineFragment(EntityName),
}

impl NormalizationKey {
    fn transform_with_parent_variable_context(
        &self,
        parent_variable_context: &VariableContext,
    ) -> Self {
        // from_selection_field_argument_and_context(arg, variable_context)
        match self.reference() {
            NormalizationKey::Discriminator => NormalizationKey::Discriminator,
            NormalizationKey::Id => NormalizationKey::Id,
            NormalizationKey::ServerField(s) => NormalizationKey::ServerField(
                transform_name_and_arguments_with_child_variable_context(
                    s.clone(),
                    parent_variable_context,
                ),
            ),
            NormalizationKey::ClientPointer(s) => NormalizationKey::ClientPointer(
                transform_name_and_arguments_with_child_variable_context(
                    s.clone(),
                    parent_variable_context,
                ),
            ),
            NormalizationKey::InlineFragment(o) => NormalizationKey::InlineFragment(*o),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct PathToRefetchFieldInfo {
    // If the field (e.g. icheckin) returns an abstract type (ICheckin) that is different than
    // the concrete type we want (Checkin), then we refine to that concrete type.
    // TODO investigate whether this can be done when the ImperativelyLoadedFieldVariant is created
    pub wrap_refetch_field_with_inline_fragment: Option<EntityName>,
    pub imperatively_loaded_field_variant: ImperativelyLoadedFieldVariant,
    pub client_selectable_id: ClientSelectableId,
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
    pub accessible_client_scalar_selectables: HashSet<ClientSelectableId>,
    pub has_updatable: bool,
}

impl ScalarClientFieldTraversalState {
    fn new() -> Self {
        Self {
            refetch_paths: BTreeMap::new(),
            traversal_path: vec![],
            accessible_client_scalar_selectables: HashSet::new(),
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
                panic!(
                    "Expected a linked field, found scalar. This is indicative of a bug in Isograph."
                )
            }
            MergedServerSelection::LinkedField(linked_field) => {
                parent_selection_map = &linked_field.selection_map;
            }
            MergedServerSelection::ClientObjectSelectable(client_object_selectable) => {
                parent_selection_map = &client_object_selectable.selection_map;
            }
            MergedServerSelection::InlineFragment(inline_fragment) => {
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
                            parent_object_entity_name: scalar_field_selection
                                .parent_object_entity_name,
                            name: scalar_field_selection.name,
                            arguments: transform_arguments_with_child_context(
                                scalar_field_selection.arguments.into_iter(),
                                parent_variable_context,
                            ),
                        })
                    }
                    MergedServerSelection::LinkedField(linked_field_selection) => {
                        MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                            parent_object_entity_name: linked_field_selection
                                .parent_object_entity_name,
                            concrete_target_entity_name: linked_field_selection
                                .concrete_target_entity_name,
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
                    MergedServerSelection::ClientObjectSelectable(linked_field_selection) => {
                        MergedServerSelection::ClientObjectSelectable(MergedLinkedFieldSelection {
                            concrete_target_entity_name: linked_field_selection
                                .concrete_target_entity_name,
                            parent_object_entity_name: linked_field_selection
                                .parent_object_entity_name,
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
                    MergedServerSelection::ClientObjectSelectable(target_linked_field) => {
                        if let MergedServerSelection::ClientObjectSelectable(child_linked_field) =
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
    TCompilationProfile: CompilationProfile,
>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    root_field_id: DefinitionLocation<(EntityName, SelectableName), ClientSelectableId>,
    variable_context: &VariableContext,
    // TODO return Cow?
) -> FieldTraversalResult {
    // TODO move this check outside of this function

    match encountered_client_type_map.get_mut(&root_field_id) {
        Some(traversal_result) => traversal_result.clone(),
        None => {
            let field_traversal_result = create_field_traversal_result(
                db,
                parent_object_entity,
                selection_set,
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

fn create_field_traversal_result<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
) -> FieldTraversalResult {
    let mut traversal_state = ScalarClientFieldTraversalState::new();
    let mut merged_selection_map = BTreeMap::new();

    merge_selection_set_into_selection_map(
        db,
        &mut merged_selection_map,
        parent_object_entity,
        selection_set,
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

pub fn get_reachable_variables(
    selection_map: &MergedSelectionMap,
) -> impl Iterator<Item = VariableNameWrapper> {
    selection_map.values().flat_map(|x| x.reachable_variables())
}

pub fn imperative_field_subfields_or_inline_fragments(
    top_level_schema_field_name: SelectableName,
    top_level_schema_field_arguments: &[VariableDeclaration],
    top_level_schema_field_concrete_target_entity_name: Option<EntityName>,
    top_level_schema_field_parent_object_entity_name: EntityName,
) -> WrappedSelectionMapSelection {
    let top_level_schema_field_arguments = top_level_schema_field_arguments
        .iter()
        .map(|variable_definition| ArgumentKeyAndValue {
            key: variable_definition.name.item.unchecked_conversion(),
            value: NonConstantValue::Variable(variable_definition.name.item),
        })
        .collect();

    WrappedSelectionMapSelection::LinkedField {
        parent_object_entity_name: top_level_schema_field_parent_object_entity_name,
        server_object_selectable_name: top_level_schema_field_name,
        arguments: top_level_schema_field_arguments,
        concrete_target_entity_name: top_level_schema_field_concrete_target_entity_name,
    }
}

fn merge_selection_set_into_selection_map<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_map: &mut MergedSelectionMap,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    selection_set: &WithEmbeddedLocation<SelectionSet>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
) {
    for selection in selection_set.item.selections.iter() {
        let selectable = selectable_named(db, parent_object_entity.name, selection.item.name())
            .as_ref()
            .expect("Expected parsing to have succeeded. This is indicative of a bug in Isograph.")
            .expect("Expected selectable to exist. This is indicative of a bug in Isograph.");

        match selection.item.reference() {
            SelectionType::Scalar(scalar_field_selection) => {
                match selectable {
                    DefinitionLocation::Server(_) => {
                        merge_server_scalar_field(
                            scalar_field_selection,
                            parent_map,
                            variable_context,
                            merge_traversal_state,
                            parent_object_entity.name,
                        );
                    }
                    DefinitionLocation::Client(_) => {
                        merge_client_scalar_field(
                            db,
                            parent_map,
                            parent_object_entity,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            scalar_field_selection,
                            parent_object_entity.name,
                            scalar_field_selection.name.item,
                        );
                    }
                };
            }
            SelectionType::Object(object_selection) => {
                let object_selectable = selectable;

                match object_selectable {
                    DefinitionLocation::Server(server_object_entity) => {
                        let server_object_entity = server_object_entity.lookup(db);
                        let target_object_entity_name =
                            server_object_entity.target_entity_name.inner().0;

                        let object_selection_parent_object_entity =
                            &server_entity_named(db, target_object_entity_name)
                                .as_ref()
                                .expect(
                                    "Expected validation to have worked. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .as_ref()
                                .expect(
                                    "Expected entity to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .lookup(db);

                        merge_server_object_field(
                            db,
                            parent_map,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            object_selection,
                            target_object_entity_name,
                            object_selection_parent_object_entity,
                            parent_object_entity.name,
                            object_selection.name.item,
                        );
                    }
                    DefinitionLocation::Client(client_object_selectable) => {
                        merge_client_object_field(
                            db,
                            parent_map,
                            parent_object_entity,
                            merge_traversal_state,
                            encountered_client_type_map,
                            variable_context,
                            object_selection,
                            parent_object_entity.name,
                            object_selection.name.item,
                        );

                        insert_client_object_selectable_into_refetch_paths(
                            db,
                            parent_map,
                            encountered_client_type_map,
                            merge_traversal_state,
                            object_selection.name.item,
                            match client_object_selectable {
                                SelectionType::Scalar(_) => panic!(
                                    "Unexpected client scalar selectable. \
                                    This is indicative of a bug in Isograph."
                                ),
                                SelectionType::Object(o) => o.lookup(db),
                            },
                            object_selection,
                            variable_context,
                        );
                    }
                }

                merge_traversal_state.traversal_path.pop();
            }
        }
    }

    select_typename_and_id_fields_in_merged_selection(db, parent_map, parent_object_entity);
}

#[expect(clippy::too_many_arguments)]
fn merge_server_object_field<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    object_selection: &ObjectSelection,
    parent_object_entity_name: EntityName,
    object_selection_parent_object: &ServerEntity<TCompilationProfile>,
    field_parent_object_entity_name: EntityName,
    field_server_object_selectable_name: SelectableName,
) {
    if let ObjectSelectionDirectiveSet::Updatable(_) =
        object_selection.object_selection_directive_set
    {
        merge_traversal_state.has_updatable = true;
    }

    let server_object_selectable = server_selectable_named(
        db,
        field_parent_object_entity_name,
        field_server_object_selectable_name,
    )
    .as_ref()
    .expect(
        "Expected validation to have succeeded. \
        This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    )
    .lookup(db);

    let object_selectable_variant = match server_object_selectable.is_inline_fragment.reference() {
        SelectionType::Scalar(_s) => panic!(
            "Unexpected scalar selection variant. \
            This is indicative of a bug in Isograph."
        ),
        SelectionType::Object(o) => o.reference(),
    };

    match object_selectable_variant {
        ServerObjectSelectableVariant::InlineFragment => {
            let type_to_refine_to = object_selection_parent_object.name;

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
                MergedServerSelection::ScalarField(_)
                | MergedServerSelection::ClientObjectSelectable(_)
                | MergedServerSelection::LinkedField(_) => {
                    panic!(
                        "Expected inline fragment. \
                        This is indicative of a bug in Isograph."
                    )
                }
                MergedServerSelection::InlineFragment(existing_inline_fragment) => {
                    let object_selection_parent_object_entity =
                        &server_entity_named(db, parent_object_entity_name)
                            .as_ref()
                            .expect(
                                "Expected validation to have worked. \
                                This is indicative of a bug in Isograph.",
                            )
                            .as_ref()
                            .expect(
                                "Expected entity to exist. \
                                This is indicative of a bug in Isograph.",
                            )
                            .lookup(db);

                    let reader_selection_set = inline_fragment_reader_selection_set();
                    merge_selection_set_into_selection_map(
                        db,
                        &mut existing_inline_fragment.selection_map,
                        object_selection_parent_object_entity,
                        &reader_selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
                        variable_context,
                    );
                    merge_selection_set_into_selection_map(
                        db,
                        &mut existing_inline_fragment.selection_map,
                        object_selection_parent_object_entity,
                        &object_selection.selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
                        variable_context,
                    );

                    create_merged_selection_map_for_field_and_insert_into_global_map(
                        db,
                        object_selection_parent_object_entity,
                        &object_selection.selection_set,
                        encountered_client_type_map,
                        (
                            field_parent_object_entity_name,
                            field_server_object_selectable_name,
                        )
                            .server_defined(),
                        &server_object_selectable.initial_variable_context(),
                    );
                }
            }
        }
        ServerObjectSelectableVariant::LinkedField => {
            let normalization_key = create_transformed_name_and_arguments(
                object_selection.name.item,
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
                let parent_object_entity_name = server_object_selectable.parent_entity_name;

                let concrete_object_entity_name =
                    server_object_selectable.target_entity_name.inner().0;

                let server_object_selection_info =
                    server_entity_named(db, concrete_object_entity_name)
                        .as_ref()
                        .expect(
                            "Expected validation to have worked. \
                        This is indicative of a bug in Isograph.",
                        )
                        .as_ref()
                        .expect(
                            "Expected entity to exist. \
                        This is indicative of a bug in Isograph.",
                        )
                        .lookup(db)
                        .selection_info
                        .as_object()
                        .expect("Expected entity to be object");

                MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                    parent_object_entity_name,
                    name: object_selection.name.item,
                    selection_map: BTreeMap::new(),
                    arguments: transform_arguments_with_child_context(
                        object_selection
                            .arguments
                            .iter()
                            .map(|arg| arg.item.into_key_and_value()),
                        variable_context,
                    ),
                    concrete_target_entity_name: if server_object_selection_info.is_concrete.0 {
                        concrete_object_entity_name.wrap_some()
                    } else {
                        None
                    },
                })
            });
            match linked_field {
                MergedServerSelection::LinkedField(existing_linked_field) => {
                    merge_selection_set_into_selection_map(
                        db,
                        &mut existing_linked_field.selection_map,
                        object_selection_parent_object,
                        &object_selection.selection_set,
                        merge_traversal_state,
                        encountered_client_type_map,
                        variable_context,
                    );
                }
                MergedServerSelection::ClientObjectSelectable(_)
                | MergedServerSelection::ScalarField(_)
                | MergedServerSelection::InlineFragment(_) => {
                    panic!(
                        "Expected linked field. \
                                            This is indicative of a bug in Isograph."
                    )
                }
            }
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn merge_client_object_field<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    object_selection: &ObjectSelection,
    parent_object_entity_name: EntityName,
    newly_encountered_client_object_selectable_name: SelectableName,
) {
    let newly_encountered_client_object_selectable = client_object_selectable_named(
        db,
        parent_object_entity_name,
        newly_encountered_client_object_selectable_name,
    )
    .as_ref()
    .expect(
        "Expected selectable to be valid. \
        This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected selectable to exist. \
        This is indicative of a bug in Isograph.",
    )
    .lookup(db);

    merge_non_loadable_client_type(
        db,
        parent_object_entity,
        parent_map,
        merge_traversal_state,
        (
            parent_object_entity_name,
            newly_encountered_client_object_selectable_name,
        )
            .object_selected(),
        newly_encountered_client_object_selectable.object_selected(),
        encountered_client_type_map,
        variable_context,
        &object_selection.arguments,
    );

    merge_traversal_state
        .accessible_client_scalar_selectables
        .insert(
            (
                parent_object_entity_name,
                newly_encountered_client_object_selectable_name,
            )
                .object_selected(),
        );

    // this is theoretically wrong, we should be adding this to the client pointer
    // traversal state instead of it's parent, but it works out the same
    merge_traversal_state
        .accessible_client_scalar_selectables
        .insert(
            (
                newly_encountered_client_object_selectable
                    .target_entity_name
                    .inner()
                    .0,
                *LINK_FIELD_NAME,
            )
                .scalar_selected(),
        );
}

#[expect(clippy::too_many_arguments)]
fn merge_client_scalar_field<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_map: &mut BTreeMap<NormalizationKey, MergedServerSelection>,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    variable_context: &VariableContext,
    scalar_field_selection: &ScalarSelection,
    parent_object_entity_name: EntityName,
    newly_encountered_scalar_client_selectable_name: SelectableName,
) {
    let newly_encountered_scalar_client_selectable = client_scalar_selectable_named(
        db,
        parent_object_entity_name,
        newly_encountered_scalar_client_selectable_name,
    )
    .as_ref()
    .expect(
        "Expected client scalar selectable to be valid. \
        This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect("Expected client scalar selectable to exist.")
    .lookup(db);

    // If the field is selected loadably or is imperative, we must note the refetch path,
    // because this results in an artifact being generated.
    match categorize_field_loadability(
        newly_encountered_scalar_client_selectable,
        &scalar_field_selection.scalar_selection_directive_set,
    ) {
        Some(Loadability::LoadablySelectedField(_loadable_variant)) => {
            create_merged_selection_map_for_field_and_insert_into_global_map(
                db,
                parent_object_entity,
                client_scalar_selectable_selection_set_for_parent_query(
                    db,
                    newly_encountered_scalar_client_selectable.parent_entity_name,
                    newly_encountered_scalar_client_selectable.name,
                )
                .expect("Expected selection set to be valid.")
                .reference(),
                encountered_client_type_map,
                (
                    parent_object_entity_name,
                    newly_encountered_scalar_client_selectable_name,
                )
                    .scalar_selected()
                    .client_defined(),
                &initial_variable_context(
                    &newly_encountered_scalar_client_selectable.scalar_selected(),
                ),
            );

            let state = encountered_client_type_map
                .get_mut(
                    &(
                        parent_object_entity_name,
                        newly_encountered_scalar_client_selectable_name,
                    )
                        .scalar_selected()
                        .client_defined(),
                )
                .expect(
                    "Expected field to exist when \
                                                it is encountered loadably",
                );
            state.was_ever_selected_loadably = true;
        }
        Some(Loadability::ImperativelyLoadedField(variant)) => {
            insert_imperative_field_into_refetch_paths(
                db,
                encountered_client_type_map,
                merge_traversal_state,
                newly_encountered_scalar_client_selectable_name,
                newly_encountered_scalar_client_selectable,
                parent_object_entity_name,
                parent_object_entity,
                variant,
            );
        }
        None => match newly_encountered_scalar_client_selectable.variant {
            ClientFieldVariant::Link => {}
            ClientFieldVariant::ImperativelyLoadedField(_) | ClientFieldVariant::UserWritten(_) => {
                merge_non_loadable_client_type(
                    db,
                    parent_object_entity,
                    parent_map,
                    merge_traversal_state,
                    (
                        parent_object_entity_name,
                        newly_encountered_scalar_client_selectable_name,
                    )
                        .scalar_selected(),
                    newly_encountered_scalar_client_selectable.scalar_selected(),
                    encountered_client_type_map,
                    variable_context,
                    &scalar_field_selection.arguments,
                )
            }
        },
    }

    merge_traversal_state
        .accessible_client_scalar_selectables
        .insert(
            (
                parent_object_entity_name,
                newly_encountered_scalar_client_selectable_name,
            )
                .scalar_selected(),
        );
}

#[expect(clippy::too_many_arguments)]
fn insert_imperative_field_into_refetch_paths<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_scalar_client_selectable_name: SelectableName,
    newly_encountered_client_scalar_selectable: &ClientScalarSelectable<TCompilationProfile>,
    parent_object_entity_name: EntityName,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    variant: &ImperativelyLoadedFieldVariant,
) {
    let path = PathToRefetchField {
        linked_fields: merge_traversal_state.traversal_path.clone(),
        field_name: newly_encountered_client_scalar_selectable
            .name
            .scalar_selected(),
    };

    let info = PathToRefetchFieldInfo {
        wrap_refetch_field_with_inline_fragment: if parent_object_entity_name
            != newly_encountered_client_scalar_selectable.parent_entity_name
        {
            parent_object_entity_name.wrap_some()
        } else {
            None
        },
        imperatively_loaded_field_variant: variant.clone(),
        client_selectable_id: (
            newly_encountered_client_scalar_selectable.parent_entity_name,
            newly_encountered_scalar_client_selectable_name,
        )
            .scalar_selected(),
    };

    merge_traversal_state.refetch_paths.insert(
        (
            path,
            ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        ),
        RootRefetchedPath {
            field_name: newly_encountered_client_scalar_selectable.name,
            path_to_refetch_field_info: info,
        },
    );

    let empty_selection_set =
        SelectionSet { selections: vec![] }.with_location(EmbeddedLocation::todo_generated());

    let refetch_strategy = refetch_strategy_for_client_scalar_selectable_named(
        db,
        newly_encountered_client_scalar_selectable.parent_entity_name,
        newly_encountered_client_scalar_selectable.name,
    )
    .as_ref()
    .expect(
        "Expected refetch strategy to be valid. \
        This is indicative of a bug in Isograph.",
    )
    .as_ref()
    .expect(
        "Expected refetch strategy. \
        This is indicative of a bug in Isograph.",
    );

    // Generate a merged selection set, but using the refetch strategy
    create_merged_selection_map_for_field_and_insert_into_global_map(
        db,
        parent_object_entity,
        refetch_strategy
            .refetch_selection_set()
            .unwrap_or(&empty_selection_set),
        encountered_client_type_map,
        (
            parent_object_entity_name,
            newly_encountered_scalar_client_selectable_name,
        )
            .scalar_selected()
            .client_defined(),
        &initial_variable_context(&newly_encountered_client_scalar_selectable.scalar_selected()),
    );
}

#[expect(clippy::too_many_arguments)]
fn insert_client_object_selectable_into_refetch_paths<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_map: &mut MergedSelectionMap,
    encountered_client_field_map: &mut FieldToCompletedMergeTraversalStateMap,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_client_object_selectable_name: SelectableName,
    newly_encountered_client_object_selectable: &ClientObjectSelectable<TCompilationProfile>,
    object_selection: &ObjectSelection,
    variable_context: &VariableContext,
) {
    let target_server_object_entity_name = newly_encountered_client_object_selectable
        .target_entity_name
        .inner()
        .0;
    let target_server_object_entity = &server_entity_named(db, target_server_object_entity_name)
        .as_ref()
        .expect(
            "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
        )
        .as_ref()
        .expect(
            "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let parent_object_entity_name = newly_encountered_client_object_selectable.parent_entity_name;

    let fetchable_types_map = fetchable_types(db)
        .as_ref()
        .expect(
            "Expected parsing to have succeeded. \
        This is indicative of a bug in Isograph.",
        )
        .lookup(db);

    let query_id = fetchable_types_map
        .iter()
        .find(|(_, root_operation_name)| root_operation_name.0 == "query")
        .expect("Expected query to be found")
        .0;

    let name_and_arguments = create_transformed_name_and_arguments(
        object_selection.name.item,
        &object_selection.arguments,
        variable_context,
    );

    let path = PathToRefetchField {
        linked_fields: merge_traversal_state.traversal_path.clone(),
        field_name: name_and_arguments.clone().object_selected(),
    };

    let mut subfields_or_inline_fragments = vec![];
    if target_server_object_entity
        .selection_info
        .as_object()
        .expect("Expected target object entity to be an object")
        .is_concrete
        .0
    {
        subfields_or_inline_fragments.push(WrappedSelectionMapSelection::InlineFragment(
            target_server_object_entity.name,
        ));
    }
    subfields_or_inline_fragments.push(WrappedSelectionMapSelection::LinkedField {
        parent_object_entity_name: *query_id,
        server_object_selectable_name: *NODE_FIELD_NAME,
        arguments: vec![ArgumentKeyAndValue {
            key: ID_FIELD_NAME.unchecked_conversion(),
            value: NonConstantValue::Variable(
                ID_FIELD_NAME.unchecked_conversion::<VariableName>().into(),
            ),
        }],
        concrete_target_entity_name: None,
    });

    let info = PathToRefetchFieldInfo {
        wrap_refetch_field_with_inline_fragment: if target_server_object_entity
            .selection_info
            .as_object()
            .expect("Expected target server object entity to be an object")
            .is_concrete
            .0
        {
            None
        } else {
            (newly_encountered_client_object_selectable
                .target_entity_name
                .inner()
                .0)
                .wrap_some()
        },

        imperatively_loaded_field_variant: ImperativelyLoadedFieldVariant {
            client_selection_name: newly_encountered_client_object_selectable.name,
            top_level_schema_field_arguments: id_arguments(),
            // top_level_schema_field_name: *NODE_FIELD_NAME,
            // top_level_schema_field_concrete_type: None,
            // primary_field_info: None,
            field_map: vec![],
            subfields_or_inline_fragments,
            root_object_entity_name: {
                *fetchable_types(db)
                    .as_ref()
                    .expect(
                        "Expected parsing to have succeeded. \
                        This is indicative of a bug in Isograph.",
                    )
                    .lookup(db)
                    .iter()
                    .find(|(_, root_operation_name)| root_operation_name.0 == "query")
                    .expect("Expected query to be found")
                    .0
            },
        },
        client_selectable_id: (
            parent_object_entity_name,
            newly_encountered_client_object_selectable_name,
        )
            .object_selected(),
    };

    merge_traversal_state.refetch_paths.insert(
        (
            path,
            ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        ),
        RootRefetchedPath {
            field_name: newly_encountered_client_object_selectable.name,
            path_to_refetch_field_info: info,
        },
    );

    let normalization_key = NormalizationKey::ClientPointer(name_and_arguments);

    merge_traversal_state
        .traversal_path
        .push(normalization_key.clone());

    let client_object_selectable = parent_map.entry(normalization_key).or_insert_with(|| {
        MergedServerSelection::ClientObjectSelectable(MergedLinkedFieldSelection {
            parent_object_entity_name,
            concrete_target_entity_name: if target_server_object_entity
                .selection_info
                .as_object()
                .expect("Expected target server object entity to be an object")
                .is_concrete
                .0
            {
                target_server_object_entity.name.wrap_some()
            } else {
                None
            },
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

    match client_object_selectable {
        MergedServerSelection::ClientObjectSelectable(existing_client_object_selectable) => {
            merge_selection_set_into_selection_map(
                db,
                &mut existing_client_object_selectable.selection_map,
                target_server_object_entity,
                &object_selection.selection_set,
                merge_traversal_state,
                encountered_client_field_map,
                variable_context,
            );
        }
        MergedServerSelection::LinkedField(_)
        | MergedServerSelection::ScalarField(_)
        | MergedServerSelection::InlineFragment(_) => {
            panic!(
                "Expected client pointer. \
                This is indicative of a bug in Isograph."
            )
        }
    }
}

#[expect(clippy::too_many_arguments)]
fn merge_non_loadable_client_type<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
    parent_map: &mut MergedSelectionMap,
    parent_merge_traversal_state: &mut ScalarClientFieldTraversalState,
    newly_encountered_client_type_id: ClientSelectableId,
    newly_encountered_client_type: ClientSelectable<TCompilationProfile>,
    encountered_client_type_map: &mut FieldToCompletedMergeTraversalStateMap,
    parent_variable_context: &VariableContext,
    selection_arguments: &[WithEmbeddedLocation<SelectionFieldArgument>],
) {
    let selections = selectable_reader_selection_set(
        db,
        match newly_encountered_client_type {
            SelectionType::Scalar(s) => s.parent_entity_name,
            SelectionType::Object(o) => o.parent_entity_name,
        },
        match newly_encountered_client_type {
            SelectionType::Scalar(s) => s.name,
            SelectionType::Object(o) => o.name,
        },
    )
    .expect("Expected selections to be valid.")
    .lookup(db)
    .reference();

    // Here, we are doing a bunch of work, just so that we can have the refetched paths,
    // which is really really silly.
    let FieldTraversalResult {
        traversal_state,
        merged_selection_map: child_merged_selection_map,
        ..
    } = create_merged_selection_map_for_field_and_insert_into_global_map(
        db,
        parent_object_entity,
        selections,
        encountered_client_type_map,
        newly_encountered_client_type_id.client_defined(),
        &initial_variable_context(&newly_encountered_client_type),
    );

    let transformed_child_variable_context = parent_variable_context.child_variable_context(
        selection_arguments,
        match newly_encountered_client_type {
            SelectionType::Scalar(s) => &s.variable_definitions,
            SelectionType::Object(o) => &o.variable_definitions,
        },
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
    scalar_field_selection: &ScalarSelection,
    parent_map: &mut MergedSelectionMap,
    variable_context: &VariableContext,
    merge_traversal_state: &mut ScalarClientFieldTraversalState,
    parent_object_entity_name: EntityName,
) {
    if let ScalarSelectionDirectiveSet::Updatable(_) =
        scalar_field_selection.scalar_selection_directive_set
    {
        merge_traversal_state.has_updatable = true;
    }

    // HACK. We probably should filter these out in a better way.
    let scalar_field_name = scalar_field_selection.name.item;
    let normalization_key = if scalar_field_name == *TYPENAME_FIELD_NAME {
        NormalizationKey::Discriminator
    } else if scalar_field_name == *ID_FIELD_NAME {
        NormalizationKey::Id
    } else {
        NormalizationKey::ServerField(create_transformed_name_and_arguments(
            scalar_field_name,
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
                MergedServerSelection::ClientObjectSelectable(_) => {
                    panic!("Unexpected client pointer, probably a bug in Isograph");
                }
                MergedServerSelection::InlineFragment(_) => {
                    panic!("Unexpected inline fragment, probably a bug in Isograph");
                }
            };
        }
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(MergedServerSelection::ScalarField(
                MergedScalarFieldSelection {
                    parent_object_entity_name,
                    name: scalar_field_name.unchecked_conversion(),
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

fn select_typename_and_id_fields_in_merged_selection<TCompilationProfile: CompilationProfile>(
    db: &IsographDatabase<TCompilationProfile>,
    merged_selection_map: &mut MergedSelectionMap,
    parent_object_entity: &ServerEntity<TCompilationProfile>,
) {
    if !parent_object_entity
        .selection_info
        .as_object()
        .expect("Expected parent object entity to be an object")
        .is_concrete
        .0
    {
        maybe_add_typename_selection(merged_selection_map, parent_object_entity.name)
    };

    let id_field = server_id_selectable(db, parent_object_entity.name)
        .as_ref()
        .expect("Expected this to be valid.");

    // If the type has an id field, we must select it.
    if let Some(id_field) = id_field {
        match merged_selection_map.entry(NormalizationKey::Id) {
            Entry::Occupied(occupied) => {
                match occupied.get() {
                    MergedServerSelection::ScalarField(_) => {
                        // TODO check that the existing server field matches the one we
                        // would create.
                    }
                    MergedServerSelection::ClientObjectSelectable(_) => {
                        panic!("Unexpected client pointer for id, probably a bug in Isograph");
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
                        parent_object_entity_name: parent_object_entity.name,
                        name: id_field.lookup(db).name,
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
        parent_object_entity_name: EntityName,
        server_object_selectable_name: SelectableName,
        arguments: Vec<ArgumentKeyAndValue>,
        concrete_target_entity_name: Option<EntityName>,
    },
    InlineFragment(EntityName),
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
                parent_object_entity_name,
                server_object_selectable_name,
                arguments,
                concrete_target_entity_name,
            } => {
                map.insert(
                    NormalizationKey::ServerField(NameAndArguments {
                        name: server_object_selectable_name,
                        arguments: arguments.clone(),
                    }),
                    MergedServerSelection::LinkedField(MergedLinkedFieldSelection {
                        parent_object_entity_name,
                        name: server_object_selectable_name,
                        selection_map: inner_selection_map,
                        arguments,
                        concrete_target_entity_name,
                    }),
                );
            }
            WrappedSelectionMapSelection::InlineFragment(isograph_object_type_name) => {
                maybe_add_typename_selection(&mut inner_selection_map, isograph_object_type_name);
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

fn maybe_add_typename_selection(
    selections: &mut MergedSelectionMap,
    parent_object_entity_name: EntityName,
) {
    // If a discriminator exists, this is a no-op
    selections.insert(
        NormalizationKey::Discriminator,
        MergedServerSelection::ScalarField(MergedScalarFieldSelection {
            parent_object_entity_name,
            name: *TYPENAME_FIELD_NAME,
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

pub fn id_arguments() -> Vec<VariableDeclaration> {
    vec![VariableDeclaration {
        name: ID_FIELD_NAME
            .unchecked_conversion::<VariableName>()
            .to::<VariableNameWrapper>()
            .with_location(EmbeddedLocation::todo_generated()),
        type_: TypeAnnotationDeclaration::Scalar((*ID_ENTITY_NAME).into())
            .with_location(EmbeddedLocation::todo_generated()),
        default_value: None,
    }]
}

pub fn inline_fragment_reader_selection_set() -> WithEmbeddedLocation<SelectionSet> {
    let typename_selection = SelectionType::Scalar(ScalarSelection {
        arguments: vec![],
        scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        name: (*TYPENAME_FIELD_NAME).with_location(EmbeddedLocation::todo_generated()),
        reader_alias: None,
    })
    .with_location(EmbeddedLocation::todo_generated());

    let link_selection = SelectionType::Scalar(ScalarSelection {
        arguments: vec![],
        scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
        name: (*LINK_FIELD_NAME).with_location(EmbeddedLocation::todo_generated()),
        reader_alias: None,
    })
    .with_location(EmbeddedLocation::todo_generated());

    SelectionSet {
        selections: vec![typename_selection, link_selection],
    }
    .with_location(EmbeddedLocation::todo_generated())
}
