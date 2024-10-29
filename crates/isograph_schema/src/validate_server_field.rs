use common_lang_types::{SelectableFieldName, UnvalidatedTypeName, WithLocation};
use graphql_lang_types::GraphQLTypeAnnotation;
use isograph_lang_types::{
    SelectableServerFieldId, ServerObjectId, TypeAnnotation, VariableDefinition,
};

use crate::{
    get_all_errors_or_all_ok, get_all_errors_or_all_ok_iter, SchemaServerField,
    SchemaValidationState, ServerFieldData, UnvalidatedSchemaSchemaField, UnvalidatedSchemaState,
    UnvalidatedVariableDefinition, ValidateSchemaError, ValidateSchemaResult,
    ValidatedSchemaServerField, ValidatedVariableDefinition,
};

pub(crate) fn validate_and_transform_server_fields(
    fields: Vec<UnvalidatedSchemaSchemaField>,
    schema_data: &ServerFieldData,
) -> Result<Vec<ValidatedSchemaServerField>, Vec<WithLocation<ValidateSchemaError>>> {
    get_all_errors_or_all_ok_iter(
        fields
            .into_iter()
            .map(|field| validate_and_transform_server_field(field, schema_data)),
    )
}

fn validate_and_transform_server_field(
    field: UnvalidatedSchemaSchemaField,
    schema_data: &ServerFieldData,
) -> Result<ValidatedSchemaServerField, impl Iterator<Item = WithLocation<ValidateSchemaError>>> {
    // TODO rewrite as field.map(...).transpose()
    let (empty_field, server_field_type) = field.split();

    let mut errors = vec![];

    let field_type =
        match validate_server_field_type_exists(schema_data, &server_field_type, &empty_field) {
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
                empty_field.name,
            )
        })) {
            Ok(arguments) => Some(arguments),
            Err(e) => {
                errors.extend(e);
                None
            }
        };

    if let Some(field_type) = field_type {
        if let Some(valid_arguments) = valid_arguments {
            return Ok(SchemaServerField {
                description: empty_field.description,
                name: empty_field.name,
                id: empty_field.id,
                associated_data: field_type,
                parent_type_id: empty_field.parent_type_id,
                arguments: valid_arguments,
                is_discriminator: empty_field.is_discriminator,
                variant: empty_field.variant,
            });
        }
    }

    Err(errors.into_iter())
}

fn validate_server_field_type_exists(
    schema_data: &ServerFieldData,
    server_field_type: &GraphQLTypeAnnotation<UnvalidatedTypeName>,
    field: &SchemaServerField<
        (),
        <UnvalidatedSchemaState as SchemaValidationState>::VariableDefinitionInnerType,
    >,
) -> ValidateSchemaResult<TypeAnnotation<SelectableServerFieldId>> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(server_field_type.inner()) {
        // Why do we need to clone here? Can we avoid this?
        Some(type_id) => Ok(TypeAnnotation::from_graphql_type_annotation(
            server_field_type.clone().map(|_| *type_id),
        )),
        None => Err(WithLocation::new(
            ValidateSchemaError::FieldTypenameDoesNotExist {
                parent_type_name: schema_data.object(field.parent_type_id).name,
                field_name: field.name.item,
                field_type: *server_field_type.inner(),
            },
            field.name.location,
        )),
    }
}

fn validate_server_field_argument(
    argument: WithLocation<UnvalidatedVariableDefinition>,
    schema_data: &ServerFieldData,
    parent_type_id: ServerObjectId,
    name: WithLocation<SelectableFieldName>,
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
