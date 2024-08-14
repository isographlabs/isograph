use common_lang_types::{
    DirectiveArgumentName, DirectiveName, IsographObjectTypeName, LinkedFieldName, Location,
    SelectableFieldName, Span, StringLiteralValue, ValueKeyName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    from_graph_ql_directive, DeserializationError, GraphQLConstantValue, GraphQLDirective,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    ArgumentKeyAndValue, ClientFieldId, IsographSelectionVariant, NonConstantValue,
    ScalarFieldSelection, SelectableServerFieldId, Selection, ServerFieldId, ServerFieldSelection,
    ServerObjectId,
};
use serde::Deserialize;

use crate::{
    generate_refetch_field_strategy, ArgumentMap, ClientField, ClientFieldVariant,
    FieldDefinitionLocation, FieldMapItem, ImperativelyLoadedFieldVariant, ObjectTypeAndFieldName,
    PrimaryFieldInfo, ProcessTypeDefinitionError, ProcessTypeDefinitionResult,
    ProcessedFieldMapItem, UnvalidatedSchema, UnvalidatedVariableDefinition,
};
use lazy_static::lazy_static;

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
    expose_as: Option<SelectableFieldName>,
    path: StringLiteralValue,
    #[serde(default)]
    field_map: Vec<FieldMapItem>,
    field: StringLiteralValue,
}

impl ExposeFieldDirective {
    pub fn new(
        expose_as: Option<SelectableFieldName>,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RequiresRefinement {
    Yes(IsographObjectTypeName),
    No,
}

impl UnvalidatedSchema {
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
        parent_object_id: ServerObjectId,
    ) -> ProcessTypeDefinitionResult<()> {
        // TODO don't clone if possible
        let parent_object = self.server_field_data.object(parent_object_id);
        let parent_object_name = parent_object.name;

        // TODO this is a bit ridiculous
        let expose_field_directives = parent_object
            .directives
            .iter()
            .map(|d| self.parse_expose_field_directive(d))
            .collect::<Result<Vec<_>, _>>()?;
        let expose_field_directives = expose_field_directives
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        for expose_field_directive in expose_field_directives.iter() {
            self.create_new_exposed_field(
                expose_field_directive,
                parent_object_name,
                parent_object_id,
            )?;
        }

        Ok(())
    }

    fn create_new_exposed_field(
        &mut self,
        expose_field_directive: &ExposeFieldDirective,
        parent_object_name: IsographObjectTypeName,
        parent_object_id: ServerObjectId,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let ExposeFieldDirective {
            expose_as,
            path,
            field_map,
            field,
        } = expose_field_directive;

        let mutation_subfield_id = self.parse_mutation_subfield_id(*field, parent_object_id)?;

        // TODO do not use mutation naming here
        let mutation_field = self.server_field(mutation_subfield_id);
        let mutation_field_payload_type_name = *mutation_field.associated_data.inner();
        let client_field_scalar_selection_name = expose_as.unwrap_or(mutation_field.name.item);
        // TODO what is going on here. Should mutation_field have a checked way of converting to LinkedField?
        let top_level_schema_field_name = mutation_field.name.item.lookup().intern().into();
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = mutation_field.description;
        let payload_id = self
            .server_field_data
            .defined_types
            .get(&mutation_field_payload_type_name)
            .copied();

        if let Some(SelectableServerFieldId::Object(mutation_field_object_id)) = payload_id {
            let processed_field_map_items = skip_arguments_contained_in_field_map(
                self,
                mutation_field_arguments.clone(),
                // TODO make this a no-op
                mutation_field_payload_type_name.lookup().intern().into(),
                parent_object_name,
                client_field_scalar_selection_name,
                // TODO don't clone
                field_map.clone(),
            )?;

            // payload object is the object type of the mutation field, e.g. SetBestFriendResponse
            let payload_object = self.server_field_data.object(mutation_field_object_id);
            let payload_object_name = payload_object.name;

            // TODO make this zero cost
            // TODO split path on .
            let primary_field_name: LinkedFieldName = path.lookup().intern().into();

            let primary_field = payload_object
                .encountered_fields
                .get(&primary_field_name.into());

            let (maybe_abstract_parent_object_id, maybe_abstract_parent_type_name) =
                match primary_field {
                    Some(FieldDefinitionLocation::Server(server_field_id)) => {
                        let server_field = self.server_field(*server_field_id);

                        // This is the parent type name (Pet)
                        let inner = server_field.associated_data.inner();

                        // TODO validate that the payload object has no plural fields in between

                        let primary_type = self.server_field_data.defined_types.get(inner);

                        if let Some(SelectableServerFieldId::Object(
                            client_field_parent_object_id,
                        )) = primary_type
                        {
                            Ok((*client_field_parent_object_id, *inner))
                        } else {
                            Err(WithLocation::new(
                                ProcessTypeDefinitionError::InvalidMutationField,
                                Location::generated(),
                            ))
                        }
                    }
                    _ => Err(WithLocation::new(
                        ProcessTypeDefinitionError::InvalidMutationField,
                        Location::generated(),
                    )),
                }?;

            let fields = processed_field_map_items
                .iter()
                .map(|field_map_item| {
                    let scalar_field_selection = ScalarFieldSelection {
                        name: WithLocation::new(
                            // TODO make this no-op
                            // TODO split on . here; we should be able to have from: "best_friend.id" or whatnot.
                            field_map_item.0.from.lookup().intern().into(),
                            Location::generated(),
                        ),
                        reader_alias: None,
                        associated_data: IsographSelectionVariant::Regular,
                        unwraps: vec![],
                        // TODO what about arguments? How would we handle them?
                        arguments: vec![],
                        directives: vec![],
                    };

                    WithSpan::new(
                        Selection::ServerField(ServerFieldSelection::ScalarField(
                            scalar_field_selection,
                        )),
                        Span::todo_generated(),
                    )
                })
                .collect::<Vec<_>>();

            let mutation_field_client_field_id = self.client_fields.len().into();
            let top_level_arguments = mutation_field_arguments
                .iter()
                .map(|input_value_def| {
                    let arg_name = input_value_def.item.name.item.lookup().intern();
                    ArgumentKeyAndValue {
                        key: arg_name.into(),
                        value: NonConstantValue::Variable(arg_name.into()),
                    }
                })
                .collect();
            let mutation_client_field = ClientField {
                description,
                // set_pet_best_friend
                name: client_field_scalar_selection_name,
                id: mutation_field_client_field_id,
                reader_selection_set: None,

                unwraps: vec![],
                variant: ClientFieldVariant::ImperativelyLoadedField(
                    ImperativelyLoadedFieldVariant {
                        client_field_scalar_selection_name: client_field_scalar_selection_name
                            .lookup()
                            .intern()
                            .into(),
                        top_level_schema_field_name,
                        top_level_schema_field_arguments: mutation_field_arguments
                            .into_iter()
                            .map(|x| x.item)
                            .collect::<Vec<_>>(),

                        primary_field_info: Some(PrimaryFieldInfo {
                            primary_field_name,
                            primary_field_return_type_object_id: maybe_abstract_parent_object_id,
                            primary_field_field_map: field_map.to_vec(),
                        }),

                        root_object_id: parent_object_id,
                    },
                ),
                variable_definitions: vec![],
                type_and_field: ObjectTypeAndFieldName {
                    // TODO make this zero cost?
                    type_name: maybe_abstract_parent_type_name.lookup().intern().into(), // e.g. Pet
                    field_name: client_field_scalar_selection_name, // set_pet_best_friend
                },
                parent_object_id: maybe_abstract_parent_object_id,
                refetch_strategy: Some(crate::RefetchStrategy::UseRefetchField(
                    generate_refetch_field_strategy(
                        fields.to_vec(),
                        // NOTE: this will probably panic if we're not exposing fields which are
                        // originally on Mutation
                        parent_object_id,
                        format!("Mutation__{}", primary_field_name).intern().into(),
                        top_level_schema_field_name,
                        top_level_arguments,
                        // This is blatantly incorrect - at this point, we don't know whether
                        // we require refinement, since the same field is copied from the abstract
                        // type to the concrete type. So, when we do that, we need to account
                        // for this.
                        RequiresRefinement::No,
                        Some(primary_field_name),
                    ),
                )),
            };
            self.client_fields.push(mutation_client_field);

            self.insert_client_field_on_object(
                client_field_scalar_selection_name,
                maybe_abstract_parent_object_id,
                mutation_field_client_field_id,
                payload_object_name,
            )?;
        }
        Ok(())
    }

    // TODO this should be defined elsewhere, probably
    pub fn insert_client_field_on_object(
        &mut self,
        mutation_field_name: SelectableFieldName,
        client_field_parent_object_id: ServerObjectId,
        client_field_id: ClientFieldId,
        payload_object_name: IsographObjectTypeName,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let client_field_parent = self
            .server_field_data
            .object_mut(client_field_parent_object_id);
        if client_field_parent
            .encountered_fields
            .insert(
                mutation_field_name,
                FieldDefinitionLocation::Client(client_field_id),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                // TODO use a more generic error message when making this
                ProcessTypeDefinitionError::FieldExistsOnSubtype {
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
                    ProcessTypeDefinitionError::FailedToDeserialize(err),
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
        mutation_object_id: ServerObjectId,
    ) -> ProcessTypeDefinitionResult<ServerFieldId> {
        let mutation = self.server_field_data.object(mutation_object_id);

        // TODO make this a no-op
        let field_arg = field_arg.lookup().intern().into();

        let field_id = mutation
            .encountered_fields
            .iter()
            .find_map(|(name, field_id)| {
                if let FieldDefinitionLocation::Server(server_field_id) = field_id {
                    if *name == field_arg {
                        return Some(server_field_id);
                    }
                }
                None
            })
            .ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::InvalidField,
                    // TODO
                    Location::generated(),
                )
            })?;

        Ok(*field_id)
    }
}

fn skip_arguments_contained_in_field_map(
    // TODO move this to impl Schema
    schema: &mut UnvalidatedSchema,
    arguments: Vec<WithLocation<UnvalidatedVariableDefinition>>,
    primary_type_name: IsographObjectTypeName,
    mutation_object_name: IsographObjectTypeName,
    mutation_field_name: SelectableFieldName,
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
