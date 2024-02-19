use common_lang_types::{
    DirectiveArgumentName, DirectiveName, EmbeddedLocation, IsographObjectTypeName, Location,
    SelectableFieldName, Span, StringLiteralValue, TextSource, ValueKeyName, WithEmbeddedLocation,
    WithLocation, WithSpan,
};
use graphql_lang_types::{ConstantValue, GraphQLDirective, GraphQLInputValueDefinition};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    DefinedTypeId, ObjectId, ResolverFieldId, ScalarFieldSelection, Selection, ServerFieldId,
    ServerFieldSelection,
};

use isograph_config::ConfigOptions;

use crate::{
    ArgumentMap, DefinedField, FieldMapItem, MutationFieldResolverActionKindInfo,
    MutationFieldResolverVariant, ProcessTypeDefinitionError, ProcessTypeDefinitionResult,
    ProcessedFieldMapItem, ResolverActionKind, ResolverTypeAndField, ResolverVariant,
    SchemaResolver, UnvalidatedSchema,
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

pub struct MagicMutationFieldInfo {
    path: StringLiteralValue,
    field_map_items: Vec<FieldMapItem>,
    text_source: TextSource,
    field_id: ServerFieldId,
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
    pub fn create_magic_mutation_fields(
        &mut self,
        mutation_id: ObjectId,
        options: ConfigOptions,
    ) -> ProcessTypeDefinitionResult<()> {
        // TODO don't clone if possible
        let mutation_object = self.schema_data.object(mutation_id);
        let mutation_object_name = mutation_object.name;

        // TODO this is a bit ridiculous
        let magic_mutation_infos = mutation_object
            .directives
            .iter()
            .map(|d| {
                self.extract_magic_mutation_field_info(d, d.name.location.text_source, mutation_id)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let magic_mutation_infos = magic_mutation_infos
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();

        for magic_mutation_info in magic_mutation_infos.iter() {
            self.create_new_magic_mutation_field(
                magic_mutation_info,
                mutation_object_name,
                options,
            )?;
        }

        Ok(())
    }

    fn create_new_magic_mutation_field(
        &mut self,
        magic_mutation_info: &MagicMutationFieldInfo,
        mutation_object_name: IsographObjectTypeName,
        options: ConfigOptions,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let MagicMutationFieldInfo {
            path,
            field_map_items,
            text_source,
            field_id,
        } = magic_mutation_info;
        let mutation_field = self.field(*field_id);
        let mutation_field_payload_type_name = *mutation_field.associated_data.inner();
        let mutation_field_name = mutation_field.name.item;
        let mutation_field_arguments = mutation_field.arguments.clone();
        let description = mutation_field.description.clone();
        let payload_id = self
            .schema_data
            .defined_types
            .get(&mutation_field_payload_type_name)
            .map(|x| *x);

        if let Some(DefinedTypeId::Object(mutation_field_object_id)) = payload_id {
            let (mutation_field_args_without_id, processed_field_map_items) =
                skip_arguments_contained_in_field_map(
                    self,
                    mutation_field_arguments.clone(),
                    // TODO make this a no-op
                    mutation_field_payload_type_name.lookup().intern().into(),
                    mutation_object_name,
                    mutation_field_name,
                    // TODO don't clone
                    field_map_items.clone(),
                    *text_source,
                    options,
                )?;

            // TODO this is dangerous! mutation_field.name is also formattable (with carats).
            // We should find a way to make WithLocation not impl Display, while also making
            // errors containing WithLocation<...> easy to work with.
            // TODO "expose as" optional field
            let magic_mutation_field_name = mutation_field_name;

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
                    Some(DefinedField::ServerField(server_field)) => {
                        // This is the parent type name (Pet)
                        let inner = server_field.inner();

                        // TODO validate that the payload object has no plural fields in between

                        let primary_type = self.schema_data.defined_types.get(inner).clone();

                        if let Some(DefinedTypeId::Object(resolver_parent_object_id)) = primary_type
                        {
                            Ok((*resolver_parent_object_id, *inner))
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

            let magic_mutation_field_resolver_id = self.resolvers.len().into();
            let magic_mutation_field_resolver = SchemaResolver {
                description,
                // set_pet_best_friend
                name: magic_mutation_field_name,
                id: magic_mutation_field_resolver_id,
                selection_set_and_unwraps: Some((fields.to_vec(), vec![])),
                variant: ResolverVariant::MutationField(MutationFieldResolverVariant {
                    mutation_field_name: magic_mutation_field_name,
                    mutation_primary_field_name: path_selectable_field_name,
                    mutation_field_arguments: mutation_field_arguments.to_vec(),
                    filtered_mutation_field_arguments: mutation_field_args_without_id.to_vec(),
                    mutation_primary_field_return_type_object_id: maybe_abstract_parent_object_id,
                }),
                variable_definitions: vec![],
                type_and_field: ResolverTypeAndField {
                    // TODO make this zero cost?
                    type_name: maybe_abstract_parent_type_name.lookup().intern().into(), // e.g. Pet
                    field_name: magic_mutation_field_name, // set_pet_best_friend
                },
                parent_object_id: maybe_abstract_parent_object_id,
                action_kind: ResolverActionKind::MutationField(
                    MutationFieldResolverActionKindInfo {
                        // TODO don't clone
                        field_map: field_map_items.to_vec(),
                    },
                ),
            };
            self.resolvers.push(magic_mutation_field_resolver);

            self.insert_resolver_field_on_object(
                magic_mutation_field_name,
                maybe_abstract_parent_object_id,
                magic_mutation_field_resolver_id,
                payload_object_name,
            )?;
        }
        Ok(())
    }

    // TODO this should be defined elsewhere, probably
    pub fn insert_resolver_field_on_object(
        &mut self,
        magic_mutation_field_name: SelectableFieldName,
        resolver_parent_object_id: ObjectId,
        resolver_id: ResolverFieldId,
        payload_object_name: IsographObjectTypeName,
    ) -> Result<(), WithLocation<ProcessTypeDefinitionError>> {
        let resolver_parent = self.schema_data.object_mut(resolver_parent_object_id);
        if resolver_parent
            .encountered_fields
            .insert(
                magic_mutation_field_name,
                DefinedField::ResolverField(resolver_id),
            )
            .is_some()
        {
            return Err(WithLocation::new(
                // TODO use a more generic error message when making this
                ProcessTypeDefinitionError::FieldExistsOnSubtype {
                    field_name: magic_mutation_field_name,
                    parent_type: payload_object_name,
                },
                // TODO this is blatantly incorrect
                Location::generated(),
            ));
        }
        resolver_parent.resolvers.push(resolver_id);

        Ok(())
    }

    fn extract_magic_mutation_field_info(
        &self,
        d: &GraphQLDirective<ConstantValue>,
        text_source: TextSource,
        mutation_id: ObjectId,
    ) -> ProcessTypeDefinitionResult<Option<MagicMutationFieldInfo>> {
        if d.name.item == *EXPOSE_FIELD_DIRECTIVE {
            Ok(Some(self.validate_magic_mutation_directive(
                d,
                text_source,
                mutation_id,
            )?))
        } else {
            Ok(None)
        }
    }

    fn validate_magic_mutation_directive(
        &self,
        d: &GraphQLDirective<ConstantValue>,
        text_source: TextSource,
        mutation_id: ObjectId,
    ) -> ProcessTypeDefinitionResult<MagicMutationFieldInfo> {
        if d.arguments.len() != 3 {
            return Err(WithEmbeddedLocation::new(
                ProcessTypeDefinitionError::InvalidPrimaryDirectiveArgumentCount,
                // This is wrong, the arguments should have a span, or the whole thing should have a span
                d.name.location,
            )
            .into_with_location());
        }

        let path = d
            .arguments
            .iter()
            .find(|d| d.name.item == *PATH_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingPathArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;
        let path_val = match path.value.item {
            ConstantValue::String(s) => Ok(s),
            _ => Err(WithEmbeddedLocation::new(
                ProcessTypeDefinitionError::PathValueShouldBeString,
                // This is wrong, the arguments should have a span, or the whole thing should have a span
                d.name.location,
            )
            .into_with_location()),
        }?;

        let field_map = d
            .arguments
            .iter()
            .find(|d| d.name.item == *FIELD_MAP_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingFieldMapArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;

        let field_map_items = parse_field_map_val(&field_map.value)?;

        let field = d
            .arguments
            .iter()
            .find(|d| d.name.item == *FIELD_DIRECTIVE_ARGUMENT)
            .ok_or_else(|| {
                WithEmbeddedLocation::new(
                    ProcessTypeDefinitionError::MissingFieldMapArg,
                    // This is wrong, the arguments should have a span, or the whole thing should have a span
                    d.name.location,
                )
                .into_with_location()
            })?;

        let field_id = self.parse_field(&field.value, mutation_id)?;

        Ok(MagicMutationFieldInfo {
            path: path_val,
            field_map_items,
            text_source,
            field_id,
        })
    }

    fn parse_field(
        &self,
        field_arg: &WithLocation<ConstantValue>,
        mutation_id: ObjectId,
    ) -> ProcessTypeDefinitionResult<ServerFieldId> {
        let mutation = self.schema_data.object(mutation_id);

        let field_arg_name = match field_arg.item {
            // TODO make this no op
            ConstantValue::String(s) => Ok(s.lookup().intern().into()),
            _ => Err(WithLocation::new(
                ProcessTypeDefinitionError::InvalidField,
                // TODO
                Location::generated(),
            )),
        }?;

        // TODO avoid a linear scan?
        let field_id = mutation
            .server_fields
            .iter()
            .find_map(|field_id| {
                let server_field = self.field(*field_id);
                if server_field.name.item == field_arg_name {
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

fn parse_field_map_val(
    value: &WithLocation<ConstantValue>,
) -> ProcessTypeDefinitionResult<Vec<FieldMapItem>> {
    let list = match &value.item {
        ConstantValue::List(l) => Ok(l),
        _ => Err(WithLocation::new(
            ProcessTypeDefinitionError::InvalidFieldMap,
            // This is wrong, the arguments should have a span, or the whole thing should have a span
            value.location,
        )),
    }?;

    list.iter()
        .map(|argument_value| {
            let object = match &argument_value.item {
                ConstantValue::Object(o) => Ok(o),
                _ => Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )),
            }?;

            if object.len() != 2 {
                return Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                ));
            }

            let from = object
                .iter()
                .find(|d| d.name.item == *FROM_VALUE_KEY_NAME)
                .ok_or_else(|| {
                    WithLocation::new(
                        ProcessTypeDefinitionError::InvalidFieldMap,
                        // This is wrong, the arguments should have a span, or the whole thing should have a span
                        argument_value.location,
                    )
                })?;

            let from_arg = match from.value.item {
                ConstantValue::String(s) => Ok(s),
                _ => Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )),
            }?;

            let to = object
                .iter()
                .find(|d| d.name.item == *TO_VALUE_KEY_NAME)
                .ok_or_else(|| {
                    WithLocation::new(
                        ProcessTypeDefinitionError::InvalidFieldMap,
                        // This is wrong, the arguments should have a span, or the whole thing should have a span
                        argument_value.location,
                    )
                })?;

            // This is weirdly low-level!
            let span = match to.value.location {
                Location::Embedded(EmbeddedLocation {
                    text_source: _,
                    span,
                }) => span,
                Location::Generated => {
                    panic!("TODO make this an error; location should not be generated here.")
                }
            };
            let to_arg = match to.value.item {
                ConstantValue::String(s) => Ok(s),
                _ => Err(WithLocation::new(
                    ProcessTypeDefinitionError::InvalidFieldMap,
                    argument_value.location,
                )),
            }?;
            let mut split = to_arg.lookup().split('.');
            let to_argument_name = split.next().expect(
                "Expected at least one item returned \
                by split. This is indicative of a bug in Isograph.",
            );
            let account_for_quote = 1;
            let account_for_period = 1;

            let mut offset: u32 =
                to_argument_name.len() as u32 + account_for_quote + account_for_period;
            let to_argument_name = WithSpan::new(
                to_argument_name.intern().into(),
                Span::new(
                    span.start + account_for_quote,
                    span.start + account_for_quote + to_argument_name.len() as u32,
                ),
            );

            Ok(FieldMapItem {
                from: from_arg,
                to_argument_name,
                to_field_names: split
                    .into_iter()
                    .map(|split_item| {
                        let len = split_item.len() as u32;
                        let old_offset = offset;
                        offset = old_offset + len + account_for_period;
                        WithSpan::new(
                            split_item.intern().into(),
                            Span::new(span.start + old_offset, span.start + old_offset + len),
                        )
                    })
                    .collect(),
            })
        })
        .collect::<Result<Vec<_>, _>>()
}

fn skip_arguments_contained_in_field_map(
    // TODO move this to impl Schema
    schema: &mut UnvalidatedSchema,
    arguments: Vec<WithLocation<GraphQLInputValueDefinition>>,
    primary_type_name: IsographObjectTypeName,
    mutation_object_name: IsographObjectTypeName,
    mutation_field_name: SelectableFieldName,
    field_map_items: Vec<FieldMapItem>,
    text_source: TextSource,
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
            text_source,
            schema,
        )?);
    }

    Ok((
        argument_map.into_arguments(schema, options),
        processed_field_map_items,
    ))
}
