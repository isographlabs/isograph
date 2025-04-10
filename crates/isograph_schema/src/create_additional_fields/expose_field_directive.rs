use common_lang_types::{
    DirectiveArgumentName, DirectiveName, IsographObjectTypeName, Location, ObjectTypeAndFieldName,
    SelectableName, ServerObjectSelectableName, Span, StringLiteralValue, ValueKeyName,
    WithLocation, WithSpan,
};
use graphql_lang_types::{
    from_graph_ql_directive, DeserializationError, GraphQLConstantValue, GraphQLDirective,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientScalarSelectableId, DefinitionLocation, EmptyDirectiveSet,
    NonConstantValue, ScalarSelection, ScalarSelectionDirectiveSet, SelectionType,
    SelectionTypeContainingSelections, ServerEntityId, ServerObjectEntityId,
    ServerObjectSelectableId, VariableDefinition,
};

use serde::Deserialize;

use crate::{
    generate_refetch_field_strategy, ClientFieldVariant, ClientScalarSelectable,
    ImperativelyLoadedFieldVariant, NetworkProtocol, PrimaryFieldInfo, RefetchStrategy, Schema,
    UnprocessedClientFieldItem, WrappedSelectionMapSelection,
};
use lazy_static::lazy_static;

use super::{
    argument_map::ArgumentMap,
    create_additional_fields_error::{
        CreateAdditionalFieldsError, FieldMapItem, ProcessTypeDefinitionResult,
        ProcessedFieldMapItem,
    },
};

lazy_static! {
    static ref EXPOSE_FIELD_DIRECTIVE: DirectiveName = "exposeField".intern().into();
    static ref PATH_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "path".intern().into();
    static ref FIELD_MAP_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "field_map".intern().into();
    static ref FIELD_DIRECTIVE_ARGUMENT: DirectiveArgumentName = "field".intern().into();
    static ref FROM_VALUE_KEY_NAME: ValueKeyName = "from".intern().into();
    static ref TO_VALUE_KEY_NAME: ValueKeyName = "to".intern().into();
}
#[derive(Deserialize, Eq, PartialEq, Debug)]
#[serde(deny_unknown_fields, rename_all = "camelCase")]
pub struct ExposeFieldDirective {
    #[serde(default)]
    #[serde(rename = "as")]
    expose_as: Option<SelectableName>,
    path: StringLiteralValue,
    #[serde(default)]
    field_map: Vec<FieldMapItem>,
    field: StringLiteralValue,
}

impl ExposeFieldDirective {
    pub fn new(
        expose_as: Option<SelectableName>,
        path: StringLiteralValue,
        field_map: Vec<FieldMapItem>,
        field: StringLiteralValue,
    ) -> Self {
        Self {
            expose_as,
            path,
            field_map,
            field,
        }
    }
}

impl<TNetworkProtocol: NetworkProtocol> Schema<TNetworkProtocol> {
    /// Add magical mutation fields.
    ///
    /// Using the MagicMutationFieldInfo (derived from @exposeField directives),
    /// add a magical field to TargetType whose name is the mutation_name, which:
    /// - executes the mutation
    /// - has the mutation's arguments (except those from field_map)
    /// - then acts as a __refetch field on that TargetType, i.e. refetches all the fields
    ///   selected in the merged selection set.
    ///
    /// There is lots of cloning going on here! Not ideal.
    pub fn add_exposed_fields_to_parent_object_types(
        &mut self,
        parent_object_entity_id: ServerObjectEntityId,
    ) -> ProcessTypeDefinitionResult<Vec<UnprocessedClientFieldItem>> {
        // TODO don't clone if possible
        let parent_object = self
            .server_entity_data
            .server_object_entity(parent_object_entity_id);
        let parent_object_name = parent_object.name;

        // TODO this is a bit ridiculous
        let expose_field_directives = self
            .server_entity_data
            .server_object_entity_available_selectables
            .get(&parent_object_entity_id)
            .expect(
                "Expected parent_object_entity_id to exist \
                in server_object_entity_available_selectables",
            )
            .2
            .iter()
            .map(|d| self.parse_expose_field_directive(d))
            .collect::<Result<Vec<_>, _>>()?;
        let expose_field_directives = expose_field_directives
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        let mut unprocessed_client_field_items = vec![];
        for expose_field_directive in expose_field_directives.iter() {
            unprocessed_client_field_items.push(self.create_new_exposed_field(
                expose_field_directive,
                parent_object_name,
                parent_object_entity_id,
            )?);
        }

        Ok(unprocessed_client_field_items)
    }

    fn create_new_exposed_field(
        &mut self,
        expose_field_directive: &ExposeFieldDirective,
        parent_object_name: IsographObjectTypeName,
        parent_object_entity_id: ServerObjectEntityId,
    ) -> Result<UnprocessedClientFieldItem, WithLocation<CreateAdditionalFieldsError>> {
        let ExposeFieldDirective {
            expose_as,
            path,
            field_map,
            field,
        } = expose_field_directive;

        let mutation_subfield_id =
            self.parse_mutation_subfield_id(*field, parent_object_entity_id)?;

        // TODO do not use mutation naming here
        let mutation_field = self.server_object_selectable(mutation_subfield_id);
        let payload_object_type_annotation = &mutation_field.target_object_entity;
        let payload_object_entity_id = *payload_object_type_annotation.inner();

        // TODO it's a bit annoying that we call .object twice!
        let mutation_field_payload_type_name = self
            .server_entity_data
            .server_object_entity(payload_object_entity_id)
            .name;

        let client_field_scalar_selection_name =
            expose_as.unwrap_or(mutation_field.name.item.into());
        // TODO what is going on here. Should mutation_field have a checked way of converting to LinkedField?
        let top_level_schema_field_name = mutation_field.name.item.unchecked_conversion();
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = mutation_field.description;

        let processed_field_map_items = skip_arguments_contained_in_field_map(
            self,
            mutation_field_arguments.clone(),
            mutation_field_payload_type_name,
            parent_object_name,
            client_field_scalar_selection_name,
            // TODO don't clone
            field_map.clone(),
        )?;

        let payload_object = self
            .server_entity_data
            .server_object_entity(payload_object_entity_id);

        // TODO split path on .
        let primary_field_name: ServerObjectSelectableName = path.unchecked_conversion();

        let primary_field = self
            .server_entity_data
            .server_object_entity_available_selectables
            .get(&payload_object_entity_id)
            .expect(
                "Expected payload_object_entity_id to exist \
                in server_object_entity_available_selectables",
            )
            .0
            .get(&primary_field_name.into());

        let (maybe_abstract_parent_object_entity_id, maybe_abstract_parent_type_name) =
            match primary_field {
                Some(DefinitionLocation::Server(SelectionType::Object(object_selectable_id))) => {
                    let server_object_selectable =
                        self.server_object_selectable(*object_selectable_id);

                    // TODO validate that the payload object has no plural fields in between

                    let client_field_parent_object_entity_id =
                        server_object_selectable.target_object_entity.inner();
                    let client_field_parent_object = self
                        .server_entity_data
                        .server_object_entity(*client_field_parent_object_entity_id);

                    Ok((
                        *client_field_parent_object_entity_id,
                        // This is the parent type name (Pet)
                        client_field_parent_object.name,
                    ))
                }
                _ => Err(WithLocation::new(
                    CreateAdditionalFieldsError::InvalidMutationField,
                    Location::generated(),
                )),
            }?;

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
                    associated_data: ScalarSelectionDirectiveSet::None(EmptyDirectiveSet {}),
                    // TODO what about arguments? How would we handle them?
                    arguments: vec![],
                };

                WithSpan::new(
                    SelectionTypeContainingSelections::Scalar(scalar_field_selection),
                    Span::todo_generated(),
                )
            })
            .collect::<Vec<_>>();

        let mutation_field_client_field_id = self.client_scalar_selectables.len().into();
        let top_level_arguments = mutation_field_arguments
            .iter()
            .map(|input_value_def| ArgumentKeyAndValue {
                key: input_value_def.item.name.item.unchecked_conversion(),
                value: NonConstantValue::Variable(input_value_def.item.name.item),
            })
            .collect();

        let top_level_schema_field_concrete_type = payload_object.concrete_type;
        let primary_field_concrete_type = self
            .server_entity_data
            .server_object_entity(maybe_abstract_parent_object_entity_id)
            .concrete_type;

        let mutation_client_field = ClientScalarSelectable {
            description,
            name: client_field_scalar_selection_name.unchecked_conversion(),
            reader_selection_set: vec![],

            variant: ClientFieldVariant::ImperativelyLoadedField(ImperativelyLoadedFieldVariant {
                client_field_scalar_selection_name: client_field_scalar_selection_name
                    .unchecked_conversion(),
                top_level_schema_field_name,
                top_level_schema_field_arguments: mutation_field_arguments
                    .into_iter()
                    .map(|x| x.item)
                    .collect::<Vec<_>>(),
                top_level_schema_field_concrete_type,
                primary_field_info: Some(PrimaryFieldInfo {
                    primary_field_name,
                    primary_field_return_type_object_entity_id:
                        maybe_abstract_parent_object_entity_id,
                    primary_field_field_map: field_map.to_vec(),
                    primary_field_concrete_type,
                }),

                root_object_entity_id: parent_object_entity_id,
            }),
            variable_definitions: vec![],
            type_and_field: ObjectTypeAndFieldName {
                // TODO make this zero cost?
                type_name: maybe_abstract_parent_type_name.unchecked_conversion(), // e.g. Pet
                field_name: client_field_scalar_selection_name, // set_pet_best_friend
            },
            parent_object_entity_id: maybe_abstract_parent_object_entity_id,
            refetch_strategy: None,
            output_format: std::marker::PhantomData,
        };
        self.client_scalar_selectables.push(mutation_client_field);

        self.insert_client_field_on_object(
            client_field_scalar_selection_name,
            maybe_abstract_parent_object_entity_id,
            mutation_field_client_field_id,
            mutation_field_payload_type_name,
        )?;
        Ok(UnprocessedClientFieldItem {
            client_field_id: mutation_field_client_field_id,
            reader_selection_set: vec![],
            refetch_strategy: Some(RefetchStrategy::UseRefetchField(
                generate_refetch_field_strategy(
                    fields.to_vec(),
                    // NOTE: this will probably panic if we're not exposing fields which are
                    // originally on Mutation
                    parent_object_entity_id,
                    vec![
                        WrappedSelectionMapSelection::LinkedField {
                            server_object_selectable_name: primary_field_name,
                            arguments: vec![],
                            concrete_type: primary_field_concrete_type,
                        },
                        // We might need an inline fragment here. What we have is blatantly incorrect
                        // - at this point, we don't know whether
                        // we require refinement, since the same field is copied from the abstract
                        // type to the concrete type. So, when we do that, we need to account
                        // for this.
                        WrappedSelectionMapSelection::LinkedField {
                            server_object_selectable_name: top_level_schema_field_name,
                            arguments: top_level_arguments,
                            concrete_type: top_level_schema_field_concrete_type,
                        },
                    ],
                ),
            )),
        })
    }

    // TODO this should be defined elsewhere, probably
    pub fn insert_client_field_on_object(
        &mut self,
        mutation_field_name: SelectableName,
        client_field_parent_object_entity_id: ServerObjectEntityId,
        client_field_id: ClientScalarSelectableId,
        payload_object_name: IsographObjectTypeName,
    ) -> Result<(), WithLocation<CreateAdditionalFieldsError>> {
        if self
            .server_entity_data
            .server_object_entity_available_selectables
            .entry(client_field_parent_object_entity_id)
            .or_default()
            .0
            .insert(
                mutation_field_name,
                DefinitionLocation::Client(SelectionType::Scalar(client_field_id)),
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

    fn parse_expose_field_directive(
        &self,
        d: &GraphQLDirective<GraphQLConstantValue>,
    ) -> ProcessTypeDefinitionResult<Option<ExposeFieldDirective>> {
        if d.name.item == *EXPOSE_FIELD_DIRECTIVE {
            let expose_field_directive = from_graph_ql_directive(d).map_err(|err| match err {
                DeserializationError::Custom(err) => WithLocation::new(
                    CreateAdditionalFieldsError::FailedToDeserialize(err),
                    d.name.location.into(), // TODO: use location of the entire directive
                ),
            })?;
            Ok(Some(expose_field_directive))
        } else {
            Ok(None)
        }
    }

    /// Here, we are turning "pet" (the field_arg) to the ServerFieldId
    /// of that specific field
    fn parse_mutation_subfield_id(
        &self,
        field_arg: StringLiteralValue,
        mutation_object_entity_id: ServerObjectEntityId,
    ) -> ProcessTypeDefinitionResult<ServerObjectSelectableId> {
        let field_id = self
            .server_entity_data
            .server_object_entity_available_selectables
            .get(&mutation_object_entity_id)
            .expect(
                "Expected mutation_object_entity_id to exist \
                in server_object_entity_available_selectables",
            )
            .0
            .iter()
            .find_map(|(name, field_id)| {
                if let DefinitionLocation::Server(SelectionType::Object(server_field_id)) = field_id
                {
                    if *name == field_arg {
                        return Some(server_field_id);
                    }
                }
                None
            })
            .ok_or_else(|| {
                WithLocation::new(
                    CreateAdditionalFieldsError::InvalidField,
                    // TODO
                    Location::generated(),
                )
            })?;

        Ok(*field_id)
    }
}

fn skip_arguments_contained_in_field_map<TNetworkProtocol: NetworkProtocol>(
    // TODO move this to impl Schema
    schema: &mut Schema<TNetworkProtocol>,
    arguments: Vec<WithLocation<VariableDefinition<ServerEntityId>>>,
    primary_type_name: IsographObjectTypeName,
    mutation_object_name: IsographObjectTypeName,
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
