use common_lang_types::{
    ClientScalarSelectableName, Location, ObjectSelectableName,
    ParentObjectEntityNameAndSelectableName, SelectableName, ServerObjectEntityName,
    ServerObjectSelectableName, Span, StringLiteralValue, WithLocation, WithSpan,
};
use intern::{Lookup, string_key::Intern};
use isograph_lang_types::{
    DefinitionLocation, EmptyDirectiveSet, ScalarSelection, ScalarSelectionDirectiveSet,
    SelectionType, SelectionTypeContainingSelections, VariableDefinition,
};

use serde::Deserialize;

use crate::{
    ClientFieldVariant, ClientScalarSelectable, ExposeAsFieldToInsert,
    ImperativelyLoadedFieldVariant, NetworkProtocol, RefetchStrategy, Schema, ServerEntityName,
    ServerObjectEntity, ServerObjectSelectableVariant, UnprocessedClientFieldItem,
    WrappedSelectionMapSelection, generate_refetch_field_strategy,
    imperative_field_subfields_or_inline_fragments,
};

use super::{
    argument_map::ArgumentMap,
    create_additional_fields_error::{
        CreateAdditionalFieldsError, FieldMapItem, ProcessTypeDefinitionResult,
        ProcessedFieldMapItem,
    },
};

// TODO move to graphql_network_protocol crate
#[derive(Deserialize, Eq, PartialEq, Debug, Hash, Clone)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExposeFieldDirective {
    // TODO make this a ScalarSelectableName
    #[serde(default)]
    #[serde(rename = "as")]
    pub expose_as: Option<SelectableName>,
    #[serde(default)]
    pub field_map: Vec<FieldMapItem>,
    pub field: StringLiteralValue,
}

impl ExposeFieldDirective {
    pub fn new(
        expose_as: Option<SelectableName>,
        field_map: Vec<FieldMapItem>,
        field: StringLiteralValue,
    ) -> Self {
        Self {
            expose_as,
            field_map,
            field,
        }
    }
}

impl<TNetworkProtocol: NetworkProtocol + 'static> Schema<TNetworkProtocol> {
    pub fn create_new_exposed_field(
        &mut self,
        expose_field_to_insert: ExposeAsFieldToInsert,
        parent_object_entity_name: ServerObjectEntityName,
    ) -> Result<UnprocessedClientFieldItem, WithLocation<CreateAdditionalFieldsError>> {
        let ExposeFieldDirective {
            expose_as,
            field_map,
            field,
        } = expose_field_to_insert.expose_field_directive;

        // HACK: we're essentially splitting the field arg by . and keeping the same
        // implementation as before. But really, there isn't much a distinction
        // between field and path, and we should clean this up.
        //
        // But, this is an expedient way to combine field and path.
        let mut path = field.lookup().split('.');
        let field = path.next().expect(
            "Expected iter to have at least one element. \
            This is indicative of a bug in Isograph.",
        );
        let primary_field_name_selection_parts =
            path.map(|x| x.intern().into()).collect::<Vec<_>>();

        let (parent_object_entity_name, mutation_subfield_name) =
            self.parse_mutation_subfield_id(field, parent_object_entity_name)?;

        // TODO do not use mutation naming here
        let mutation_field = self
            .server_object_selectable(parent_object_entity_name, mutation_subfield_name)
            .expect(
                "Expected selectable to exist. \
                    This is indicative of a bug in Isograph.",
            );
        let payload_object_type_annotation = &mutation_field.target_object_entity;
        let payload_object_entity_name = *payload_object_type_annotation.inner();

        // TODO it's a bit annoying that we call .object twice!
        let mutation_field_payload_type_name = self
            .server_entity_data
            .server_object_entity(payload_object_entity_name)
            .expect(
                "Expected object entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .name;

        let client_field_scalar_selection_name =
            expose_as.unwrap_or(mutation_field.name.item.into());
        // TODO what is going on here. Should mutation_field have a checked way of converting to LinkedField?
        let top_level_schema_field_name = mutation_field.name.item.unchecked_conversion();
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = expose_field_to_insert
            .description
            .or(mutation_field.description);

        let processed_field_map_items = skip_arguments_contained_in_field_map(
            self,
            mutation_field_arguments.clone(),
            mutation_field_payload_type_name.item,
            expose_field_to_insert.parent_object_name,
            client_field_scalar_selection_name,
            // TODO don't clone
            field_map.clone(),
        )?;

        let payload_object_entity = self
            .server_entity_data
            .server_object_entity(payload_object_entity_name);

        let maybe_abstract_target_object_entity = traverse_object_selections(
            self,
            payload_object_entity_name,
            primary_field_name_selection_parts.iter().copied(),
        )
        .map_err(|e| WithLocation::new(e, Location::generated()))?;

        let maybe_abstract_parent_object_entity_name = maybe_abstract_target_object_entity.name;
        let maybe_abstract_parent_object_entity = maybe_abstract_target_object_entity;

        let fields = processed_field_map_items
            .iter()
            .map(|field_map_item| {
                let scalar_field_selection = ScalarSelection {
                    name: WithLocation::new(
                        // TODO make this no-op
                        // TODO split on . here; we should be able to have from: "best_friend.id" or whatnot.
                        field_map_item.0.from.unchecked_conversion(),
                        Location::generated(),
                    ),
                    reader_alias: None,
                    associated_data: (),
                    scalar_selection_directive_set: ScalarSelectionDirectiveSet::None(
                        EmptyDirectiveSet {},
                    ),
                    // TODO what about arguments? How would we handle them?
                    arguments: vec![],
                };

                WithSpan::new(
                    SelectionTypeContainingSelections::Scalar(scalar_field_selection),
                    Span::todo_generated(),
                )
            })
            .collect::<Vec<_>>();

        let mutation_field_client_field_name =
            client_field_scalar_selection_name.unchecked_conversion();

        let top_level_schema_field_concrete_type = payload_object_entity
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .concrete_type;
        let primary_field_concrete_type = maybe_abstract_parent_object_entity.concrete_type;

        let top_level_schema_field_arguments = mutation_field_arguments
            .into_iter()
            .map(|x| x.item)
            .collect::<Vec<_>>();

        let mut parts_reversed = self
            .get_object_selections_path(
                payload_object_entity_name,
                primary_field_name_selection_parts.iter().copied(),
            )
            .map_err(|e| WithLocation::new(e, Location::generated()))?;
        parts_reversed.reverse();

        let mut subfields_or_inline_fragments = parts_reversed
            .iter()
            .map(|server_object_selectable| {
                // The server object selectable may represent a linked field or an inline fragment
                match server_object_selectable.object_selectable_variant {
                    ServerObjectSelectableVariant::LinkedField => {
                        WrappedSelectionMapSelection::LinkedField {
                            server_object_selectable_name: server_object_selectable.name.item,
                            arguments: vec![],
                            concrete_type: primary_field_concrete_type,
                        }
                    }
                    ServerObjectSelectableVariant::InlineFragment => {
                        WrappedSelectionMapSelection::InlineFragment(
                            self.server_entity_data
                                .server_object_entity(
                                    *server_object_selectable.target_object_entity.inner(),
                                )
                                .expect(
                                    "Expected entity to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .name
                                .item,
                        )
                    }
                }
            })
            .collect::<Vec<_>>();

        subfields_or_inline_fragments.push(imperative_field_subfields_or_inline_fragments(
            top_level_schema_field_name,
            &top_level_schema_field_arguments,
            top_level_schema_field_concrete_type,
        ));

        let mutation_client_scalar_selectable = ClientScalarSelectable {
            description,
            name: WithLocation::new(
                client_field_scalar_selection_name.unchecked_conversion(),
                Location::generated(),
            ),
            reader_selection_set: vec![],

            variant: ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
                top_level_schema_field_arguments,
                client_selection_name: client_field_scalar_selection_name.unchecked_conversion(),

                root_object_entity_name: parent_object_entity_name,
                subfields_or_inline_fragments: subfields_or_inline_fragments.clone(),
                field_map,
            }),
            variable_definitions: vec![],
            type_and_field: ParentObjectEntityNameAndSelectableName {
                type_name: maybe_abstract_parent_object_entity_name
                    .item
                    .unchecked_conversion(), // e.g. Pet
                field_name: client_field_scalar_selection_name, // set_pet_best_friend
            },
            parent_object_entity_name: maybe_abstract_parent_object_entity_name.item,
            refetch_strategy: None,
            network_protocol: std::marker::PhantomData,
        };
        self.client_scalar_selectables.insert(
            (
                maybe_abstract_parent_object_entity_name.item,
                mutation_client_scalar_selectable.name.item,
            ),
            mutation_client_scalar_selectable,
        );

        self.insert_client_field_on_object(
            client_field_scalar_selection_name,
            maybe_abstract_parent_object_entity_name.item,
            mutation_field_client_field_name,
            mutation_field_payload_type_name.item,
        )?;
        Ok(UnprocessedClientFieldItem {
            client_field_name: mutation_field_client_field_name,
            parent_object_entity_name: maybe_abstract_parent_object_entity_name.item,
            reader_selection_set: vec![],
            refetch_strategy: Some(RefetchStrategy::UseRefetchField(
                generate_refetch_field_strategy(
                    fields.to_vec(),
                    // NOTE: this will probably panic if we're not exposing fields which are
                    // originally on Mutation
                    parent_object_entity_name,
                    subfields_or_inline_fragments,
                ),
            )),
        })
    }

    // TODO this should be defined elsewhere, probably
    pub fn insert_client_field_on_object(
        &mut self,
        mutation_field_name: SelectableName,
        client_field_parent_object_entity_name: ServerObjectEntityName,
        client_field_name: ClientScalarSelectableName,
        payload_object_name: ServerObjectEntityName,
    ) -> Result<(), WithLocation<CreateAdditionalFieldsError>> {
        if self
            .server_entity_data
            .server_object_entity_extra_info
            .entry(client_field_parent_object_entity_name)
            .or_default()
            .selectables
            .insert(
                mutation_field_name,
                DefinitionLocation::Client(SelectionType::Scalar((
                    client_field_parent_object_entity_name,
                    client_field_name,
                ))),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                // TODO use a more generic error message when making this
                CreateAdditionalFieldsError::CompilerCreatedFieldExistsOnType {
                    field_name: mutation_field_name,
                    parent_type: payload_object_name,
                },
                // TODO this is blatantly incorrect
                Location::generated(),
            ));
        }

        Ok(())
    }

    /// Here, we are turning "pet" (the field_arg) to the ServerFieldId
    /// of that specific field
    fn parse_mutation_subfield_id(
        &self,
        field_arg: &str,
        mutation_object_entity_name: ServerObjectEntityName,
    ) -> ProcessTypeDefinitionResult<(ServerObjectEntityName, ServerObjectSelectableName)> {
        let parent_entity_name_and_mutation_subfield_name = self
            .server_entity_data
            .server_object_entity_extra_info
            .get(&mutation_object_entity_name)
            .expect(
                "Expected mutation_object_entity_name to exist \
                in server_object_entity_available_selectables",
            )
            .selectables
            .iter()
            .find_map(|(name, field_id)| {
                if let DefinitionLocation::Server(SelectionType::Object(server_field_id)) = field_id
                    && name.lookup() == field_arg
                {
                    return Some(server_field_id);
                }
                None
            })
            .ok_or_else(|| {
                WithLocation::new(
                    CreateAdditionalFieldsError::InvalidField {
                        field_arg: field_arg.to_string(),
                    },
                    // TODO
                    Location::generated(),
                )
            })?;

        Ok(*parent_entity_name_and_mutation_subfield_name)
    }
}

fn skip_arguments_contained_in_field_map<TNetworkProtocol: NetworkProtocol + 'static>(
    // TODO move this to impl Schema
    schema: &mut Schema<TNetworkProtocol>,
    arguments: Vec<WithLocation<VariableDefinition<ServerEntityName>>>,
    primary_type_name: ServerObjectEntityName,
    mutation_object_name: ServerObjectEntityName,
    mutation_field_name: SelectableName,
    field_map_items: Vec<FieldMapItem>,
) -> ProcessTypeDefinitionResult<Vec<ProcessedFieldMapItem>> {
    let mut processed_field_map_items = Vec::with_capacity(field_map_items.len());
    // TODO
    // We need to create entirely new arguments, which are the existing arguments minus
    // any paths that are in the field map.
    let mut argument_map = ArgumentMap::new(arguments);

    for field_map_item in field_map_items {
        processed_field_map_items.push(argument_map.remove_field_map_item(
            field_map_item,
            primary_type_name,
            mutation_object_name,
            mutation_field_name,
            schema,
        )?);
    }

    Ok(processed_field_map_items)
}

fn traverse_object_selections<TNetworkProtocol: NetworkProtocol + 'static>(
    schema: &Schema<TNetworkProtocol>,
    root_object_name: ServerObjectEntityName,
    selections: impl Iterator<Item = ObjectSelectableName>,
) -> Result<&ServerObjectEntity<TNetworkProtocol>, CreateAdditionalFieldsError> {
    let mut current_entity = schema
        .server_entity_data
        .server_object_entity(root_object_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );
    let mut current_selectables = &schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&root_object_name)
        .expect(
            "Expected root_object_entity_name to exist \
            in server_object_entity_available_selectables",
        )
        .selectables;

    for selection_name in selections {
        match current_selectables.get(&selection_name.into()) {
            Some(entity) => match entity.transpose() {
                SelectionType::Scalar(_) => {
                    // TODO show a better error message
                    return Err(CreateAdditionalFieldsError::InvalidField {
                        field_arg: selection_name.lookup().to_string(),
                    });
                }
                SelectionType::Object(object) => {
                    let target_object_entity_name = match object {
                        DefinitionLocation::Server((
                            parent_object_entity_name,
                            server_object_selectable_name,
                        )) => {
                            let selectable = schema.server_object_selectable(
                                *parent_object_entity_name,
                                *server_object_selectable_name,
                            );
                            selectable
                                .expect(
                                    "Expected selectable to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .target_object_entity
                                .inner()
                        }
                        DefinitionLocation::Client((
                            parent_object_entity_name,
                            client_object_selectable_name,
                        )) => {
                            let pointer = schema.client_object_selectable(
                                *parent_object_entity_name,
                                *client_object_selectable_name,
                            );
                            pointer
                                .expect(
                                    "Expected selectable to exist. \
                                    This is indicative of a bug in Isograph.",
                                )
                                .target_object_entity_name
                                .inner()
                        }
                    };

                    current_entity = schema
                        .server_entity_data
                        .server_object_entity(*target_object_entity_name)
                        .expect(
                            "Expected entity to exist. \
                            This is indicative of a bug in Isograph.",
                        );
                    current_selectables = &schema
                        .server_entity_data
                        .server_object_entity_extra_info
                        .get(target_object_entity_name)
                        .expect(
                            "Expected target_object_entity_name to exist \
                            in server_object_entity_available_selectables",
                        )
                        .selectables;
                }
            },
            None => {
                return Err(CreateAdditionalFieldsError::PrimaryDirectiveFieldNotFound {
                    primary_type_name: current_entity.name.item,
                    field_name: selection_name.unchecked_conversion(),
                });
            }
        };
    }

    Ok(current_entity)
}
