use common_lang_types::{SelectableName, WithLocation};
use isograph_lang_types::{ServerObjectId, VariableDefinition};

use crate::{
    get_all_errors_or_all_ok, get_all_errors_or_all_ok_iter, OutputFormat, SchemaServerField,
    ServerFieldData, UnvalidatedSchemaSchemaField, UnvalidatedVariableDefinition,
    ValidateSchemaError, ValidateSchemaResult, ValidatedSchemaServerField,
    ValidatedVariableDefinition,
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
    // TODO rewrite this function... it can be simplified

    let mut errors = vec![];

    let valid_arguments =
        match get_all_errors_or_all_ok(field.arguments.into_iter().map(|argument| {
            validate_server_field_argument(
                argument,
                schema_data,
                field.parent_type_id,
                field.name.map(Into::into),
            )
        })) {
            Ok(arguments) => Some(arguments),
            Err(e) => {
                errors.extend(e);
                None
            }
        };

    if let Some(valid_arguments) = valid_arguments {
        return Ok(SchemaServerField {
            description: field.description,
            name: field.name,
            id: field.id,
            parent_type_id: field.parent_type_id,
            arguments: valid_arguments,
            phantom_data: std::marker::PhantomData,
            linked_field_variant: field.linked_field_variant,
            target_server_entity: field.target_server_entity,
        });
    }

    Err(errors.into_iter())
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
