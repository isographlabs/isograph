use common_lang_types::{
    DirectiveArgumentName, DirectiveName, IsographObjectTypeName, Location, SelectableFieldName,
    Span, StringLiteralValue, ValueKeyName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    from_graph_ql_directive, ConstantValue, DeserializationError, GraphQLDirective,
    GraphQLInputValueDefinition,
};
use intern::{string_key::Intern, Lookup};
use isograph_config::ConfigOptions;
use isograph_lang_types::{
    ClientFieldId, ObjectId, ScalarFieldSelection, SelectableFieldId, Selection, ServerFieldId,
    ServerFieldSelection,
};
use serde::Deserialize;

use crate::{
    ArgumentMap, ClientField, ClientFieldActionKind, ClientFieldVariant, FieldDefinitionLocation,
    FieldMapItem, MutationFieldClientFieldVariant, MutationFieldResolverActionKindInfo,
    ObjectTypeAndFieldNames, ProcessTypeDefinitionError, ProcessTypeDefinitionResult,
    ProcessedFieldMapItem, UnvalidatedSchema,
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
    pub fn create_mutation_fields_from_expose_field_directives(
        &mut self,
        mutation_id: ObjectId,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<()> {
        // TODO don't clone if possible
        let mutation_object = self.schema_data.object(mutation_id);
        let mutation_object_name = mutation_object.name;

        // TODO this is a bit ridiculous
        let expose_field_directives = mutation_object
            .directives
            .iter()
            .map(|d| self.parse_expose_field_directive(d))
            .collect::<Result<Vec<_>, _>>()?;
        let expose_field_directives = expose_field_directives
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        for expose_field_directive in expose_field_directives.iter() {
            self.create_new_mutation_field(
                expose_field_directive,
                mutation_object_name,
                mutation_id,
                options,
            )?;
        }

        Ok(())
    }

    fn create_new_mutation_field(
        &mut self,
        expose_field_directive: &ExposeFieldDirective,
        mutation_object_name: IsographObjectTypeName,
        mutation_id: ObjectId,
        options: ConfigOptions,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let ExposeFieldDirective {
            expose_as,
            path,
            field_map,
            field,
        } = expose_field_directive;

        let field_id = self.parse_field(*field, mutation_id)?;

        let mutation_field = self.field(field_id);
        let mutation_field_payload_type_name = *mutation_field.associated_data.inner();
        let mutation_field_name = expose_as.unwrap_or(mutation_field.name.item);
        let server_schema_mutation_field_name = mutation_field.name.item;
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = mutation_field.description.clone();
        let payload_id = self
            .schema_data
            .defined_types
            .get(&mutation_field_payload_type_name)
            .map(|x| *x);

        if let Some(SelectableFieldId::Object(mutation_field_object_id)) = payload_id {
            let (mutation_field_args_without_id, processed_field_map_items) =
                skip_arguments_contained_in_field_map(
                    self,
                    mutation_field_arguments.clone(),
                    // TODO make this a no-op
                    mutation_field_payload_type_name.lookup().intern().into(),
                    mutation_object_name,
                    mutation_field_name,
                    // TODO don't clone
                    field_map.clone(),
                    options,
                )?;

            // payload object is the object type of the mutation field, e.g. SetBestFriendResponse
            let payload_object = self.schema_data.object(mutation_field_object_id);
            let payload_object_name = payload_object.name;

            // TODO make this zero cost
            // TODO split path on .
            let path_selectable_field_name = path.lookup().intern().into();

            let primary_field = payload_object
                .encountered_fields
                .get(&path_selectable_field_name);

            let (maybe_abstract_parent_object_id, maybe_abstract_parent_type_name) =
                match primary_field {
                    Some(FieldDefinitionLocation::Server(server_field)) => {
                        // This is the parent type name (Pet)
                        let inner = server_field.inner();

                        // TODO validate that the payload object has no plural fields in between

                        let primary_type = self.schema_data.defined_types.get(inner).clone();

                        if let Some(SelectableFieldId::Object(client_field_parent_object_id)) =
                            primary_type
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
                        normalization_alias: None,
                        associated_data: (),
                        unwraps: vec![],
                        // TODO what about arguments? How would we handle them?
                        arguments: vec![],
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
            let mutation_client_field = ClientField {
                description,
                // set_pet_best_friend
                name: mutation_field_name,
                id: mutation_field_client_field_id,
                selection_set_and_unwraps: Some((fields.to_vec(), vec![])),
                variant: ClientFieldVariant::MutationField(MutationFieldClientFieldVariant {
                    mutation_field_name,
                    server_schema_mutation_field_name,
                    mutation_primary_field_name: path_selectable_field_name,
                    mutation_field_arguments: mutation_field_arguments.to_vec(),
                    filtered_mutation_field_arguments: mutation_field_args_without_id.to_vec(),
                    mutation_primary_field_return_type_object_id: maybe_abstract_parent_object_id,
                }),
                variable_definitions: vec![],
                type_and_field: ObjectTypeAndFieldNames {
                    // TODO make this zero cost?
                    type_name: maybe_abstract_parent_type_name.lookup().intern().into(), // e.g. Pet
                    field_name: mutation_field_name, // set_pet_best_friend
                },
                parent_object_id: maybe_abstract_parent_object_id,
                action_kind: ClientFieldActionKind::MutationField(
                    MutationFieldResolverActionKindInfo {
                        // TODO don't clone
                        field_map: field_map.to_vec(),
                    },
                ),
            };
            self.client_fields.push(mutation_client_field);

            self.insert_client_field_on_object(
                mutation_field_name,
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
        client_field_parent_object_id: ObjectId,
        client_field_id: ClientFieldId,
        payload_object_name: IsographObjectTypeName,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let client_field_parent = self.schema_data.object_mut(client_field_parent_object_id);
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
        client_field_parent.client_field_ids.push(client_field_id);

        Ok(())
    }

    fn parse_expose_field_directive(
        &self,
        d: &GraphQLDirective<ConstantValue>,
    ) -> ProcessTypeDefinitionResult<Option<ExposeFieldDirective>> {
        if d.name.item == *EXPOSE_FIELD_DIRECTIVE {
            let mutation = from_graph_ql_directive(d).map_err(|err| match err {
                DeserializationError::Custom(err) => WithLocation::new(
                    ProcessTypeDefinitionError::FailedToDeserialize(err),
                    d.name.location.into(), // TODO: use location of the entire directive
                ),
            })?;
            Ok(Some(mutation))
        } else {
            Ok(None)
        }
    }

    fn parse_field(
        &self,
        field_arg: StringLiteralValue,
        mutation_id: ObjectId,
    ) -> ProcessTypeDefinitionResult<ServerFieldId> {
        let mutation = self.schema_data.object(mutation_id);

        // TODO make this a no-op
        let field_arg = field_arg.lookup().intern().into();

        // TODO avoid a linear scan?
        let field_id = mutation
            .server_fields
            .iter()
            .find_map(|field_id| {
                let server_field = self.field(*field_id);
                if server_field.name.item == field_arg {
                    Some(*field_id)
                } else {
                    None
                }
            })
            .ok_or_else(|| {
                WithLocation::new(
                    ProcessTypeDefinitionError::InvalidField,
                    // TODO
                    Location::generated(),
                )
            })?;

        Ok(field_id)
    }
}

fn skip_arguments_contained_in_field_map(
    // TODO move this to impl Schema
    schema: &mut UnvalidatedSchema,
    arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    primary_type_name: IsographObjectTypeName,
    mutation_object_name: IsographObjectTypeName,
    mutation_field_name: SelectableFieldName,
    field_map_items: Vec<FieldMapItem>,
    options: ConfigOptions,
) -> ProcessTypeDefinitionResult<(
    Vec<WithLocation<GraphQLInputValueDefinition>>,
    Vec<ProcessedFieldMapItem>,
)> {
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

    Ok((
        argument_map.into_arguments(schema, options),
        processed_field_map_items,
    ))
}
