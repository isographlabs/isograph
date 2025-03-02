use common_lang_types::{SelectableName, UnvalidatedTypeName, WithLocation};
use graphql_lang_types::GraphQLTypeAnnotation;
use isograph_lang_types::{
    SelectionType, ServerObjectId, ServerScalarId, TypeAnnotation, VariableDefinition,
};

use crate::{
    get_all_errors_or_all_ok, get_all_errors_or_all_ok_iter,
    schema_validation_state::SchemaValidationState, OutputFormat, SchemaServerField,
    SchemaServerFieldVariant, ServerFieldData, ServerFieldTypeAssociatedData,
    ServerFieldTypeAssociatedDataInlineFragment, UnvalidatedSchemaSchemaField,
    UnvalidatedSchemaState, UnvalidatedVariableDefinition, ValidateSchemaError,
    ValidateSchemaResult, ValidatedSchemaServerField, ValidatedVariableDefinition,
};

pub(crate) fn validate_and_transform_server_fields<TOutputFormat: OutputFormat>(
    fields: Vec<UnvalidatedSchemaSchemaField<TOutputFormat>>,
    schema_data: &ServerFieldData<TOutputFormat>,
) -> Result<Vec<ValidatedSchemaServerField<TOutputFormat>>, Vec<WithLocation<ValidateSchemaError>>>
{
    get_all_errors_or_all_ok_iter(
        fields
            .into_iter()
            .map(|field| validate_and_transform_server_field(field, schema_data)),
    )
}

fn validate_and_transform_server_field<TOutputFormat: OutputFormat>(
    field: UnvalidatedSchemaSchemaField<TOutputFormat>,
    schema_data: &ServerFieldData<TOutputFormat>,
) -> Result<
    ValidatedSchemaServerField<TOutputFormat>,
    impl Iterator<Item = WithLocation<ValidateSchemaError>>,
> {
    // TODO rewrite as field.map(...).transpose()
    let (empty_field, server_field) = field.split();

    let mut errors = vec![];

    let field_type =
        match validate_server_field_type_exists(schema_data, &server_field.type_name, &empty_field)
        {
            Ok(type_annotation) => Some(type_annotation),
            Err(e) => {
                errors.push(e);
                None
            }
        };

    let valid_arguments =
        match get_all_errors_or_all_ok(empty_field.arguments.into_iter().map(|argument| {
            validate_server_field_argument(
                argument,
                schema_data,
                empty_field.parent_type_id,
                empty_field.name.map(Into::into),
            )
        })) {
            Ok(arguments) => Some(arguments),
            Err(e) => {
                errors.extend(e);
                None
            }
        };

    if let Some(field_type) = field_type {
        let variant = match server_field.variant {
            SchemaServerFieldVariant::LinkedField => SchemaServerFieldVariant::LinkedField,
            SchemaServerFieldVariant::InlineFragment(associated_data) => match field_type {
                SelectionType::Scalar(_) => {
                    panic!("Expected inline fragment server field type to be an object")
                }
                SelectionType::Object(_) => SchemaServerFieldVariant::InlineFragment(
                    ServerFieldTypeAssociatedDataInlineFragment {
                        concrete_type: associated_data.concrete_type,
                        reader_selection_set: associated_data.reader_selection_set,
                        server_field_id: associated_data.server_field_id,
                    },
                ),
            },
        };

        if let Some(valid_arguments) = valid_arguments {
            return Ok(SchemaServerField {
                description: empty_field.description,
                name: empty_field.name,
                id: empty_field.id,
                associated_data: match field_type {
                    SelectionType::Scalar(scalar_id) => SelectionType::Scalar(scalar_id),
                    SelectionType::Object(object_id) => {
                        SelectionType::Object(ServerFieldTypeAssociatedData {
                            type_name: object_id,
                            variant,
                        })
                    }
                },
                parent_type_id: empty_field.parent_type_id,
                arguments: valid_arguments,
                is_discriminator: empty_field.is_discriminator,
                phantom_data: std::marker::PhantomData,
            });
        }
    }

    Err(errors.into_iter())
}

fn validate_server_field_type_exists<TOutputFormat: OutputFormat>(
    schema_data: &ServerFieldData<TOutputFormat>,
    server_field_type: &GraphQLTypeAnnotation<UnvalidatedTypeName>,
    field: &SchemaServerField<
        (),
        <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
        TOutputFormat,
    >,
) -> ValidateSchemaResult<
    SelectionType<TypeAnnotation<ServerScalarId>, TypeAnnotation<ServerObjectId>>,
> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(server_field_type.inner()) {
        // Why do we need to clone here? Can we avoid this?
        Some(type_id) => Ok(match type_id {
            SelectionType::Scalar(scalar_id) => {
                SelectionType::Scalar(TypeAnnotation::from_graphql_type_annotation(
                    server_field_type.clone().map(|_| *scalar_id),
                ))
            }
            SelectionType::Object(object_id) => {
                SelectionType::Object(TypeAnnotation::from_graphql_type_annotation(
                    server_field_type.clone().map(|_| *object_id),
                ))
            }
        }),
        None => Err(WithLocation::new(
            ValidateSchemaError::FieldTypenameDoesNotExist {
                parent_type_name: schema_data.object(field.parent_type_id).name,
                field_name: field.name.item.into(),
                field_type: *server_field_type.inner(),
            },
            field.name.location,
        )),
    }
}

fn validate_server_field_argument<TOutputFormat: OutputFormat>(
    argument: WithLocation<UnvalidatedVariableDefinition>,
    schema_data: &ServerFieldData<TOutputFormat>,
    parent_type_id: ServerObjectId,
    name: WithLocation<SelectableName>,
) -> ValidateSchemaResult<WithLocation<ValidatedVariableDefinition>> {
    // Isograph doesn't care about the default value, and that remains
    // unvalidated.

    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(argument.item.type_.inner()) {
        Some(selectable_server_field_id) => Ok(WithLocation::new(
            VariableDefinition {
                name: argument.item.name,
                type_: argument.item.type_.map(|_| *selectable_server_field_id),
                default_value: argument.item.default_value,
            },
            argument.location,
        )),
        None => Err(WithLocation::new(
            ValidateSchemaError::FieldArgumentTypeDoesNotExist {
                parent_type_name: schema_data.object(parent_type_id).name,
                field_name: name.item,
                argument_name: argument.item.name.item,
                argument_type: *argument.item.type_.inner(),
            },
            name.location,
        )),
    }
}
