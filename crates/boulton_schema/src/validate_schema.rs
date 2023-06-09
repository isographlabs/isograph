use std::collections::HashMap;

use boulton_lang_types::{
    LinkedFieldSelection, ScalarFieldSelection, Selection, SelectionSetAndUnwraps,
    VariableDefinition,
};
use common_lang_types::{
    DefinedField, FieldDefinitionName, HasName, InputTypeId, OutputTypeId, ScalarFieldName, TypeId,
    TypeWithFieldsId, TypeWithFieldsName, TypeWithoutFieldsId, UnvalidatedTypeName, VariableName,
    WithSpan,
};
use graphql_lang_types::TypeAnnotation;
use thiserror::Error;

use crate::{
    Schema, SchemaData, SchemaField, SchemaResolverDefinitionInfo, SchemaTypeWithFields,
    UnvalidatedSchema, UnvalidatedSchemaField,
};

pub type ValidatedSchemaField =
    SchemaField<DefinedField<TypeAnnotation<OutputTypeId>, ValidatedSchemaResolverDefinitionInfo>>;

type ValidatedDefinedField = DefinedField<TypeWithoutFieldsId, ()>;

pub type ValidatedSelectionSetAndUnwraps =
    SelectionSetAndUnwraps<ValidatedDefinedField, TypeWithFieldsId>;

pub type ValidatedSelection = Selection<ValidatedDefinedField, TypeWithFieldsId>;

pub type ValidatedSchemaResolverDefinitionInfo =
    SchemaResolverDefinitionInfo<ValidatedDefinedField, TypeWithFieldsId, InputTypeId>;

pub type ValidatedVariableDefinition = VariableDefinition<InputTypeId>;

pub type ValidatedSchema =
    Schema<OutputTypeId, ValidatedDefinedField, TypeWithFieldsId, InputTypeId>;

impl ValidatedSchema {
    pub fn validate_and_construct(
        unvalidated_schema: UnvalidatedSchema,
    ) -> ValidateSchemaResult<Self> {
        let Schema {
            fields,
            schema_data,
            id_type,
            string_type,
            query_type_id: query_type,
        } = unvalidated_schema;

        let updated_fields = validate_and_transform_fields(fields, &schema_data)?;

        Ok(Self {
            fields: updated_fields,
            schema_data,
            id_type,
            string_type,
            query_type_id: query_type,
        })
    }
}

fn validate_and_transform_fields(
    fields: Vec<UnvalidatedSchemaField>,
    schema_data: &SchemaData,
) -> ValidateSchemaResult<Vec<ValidatedSchemaField>> {
    fields
        .into_iter()
        .map(|field| validate_and_transform_field(field, schema_data))
        .collect()
}

fn validate_and_transform_field(
    field: UnvalidatedSchemaField,
    schema_data: &SchemaData,
) -> ValidateSchemaResult<ValidatedSchemaField> {
    let (empty_field, field_type) = field.split();
    let field_type = match field_type {
        DefinedField::ServerField(server_field_type) => {
            let output_type_name = validate_server_field_type_exists_and_is_output_type(
                schema_data,
                &server_field_type,
                &empty_field,
            )?;
            DefinedField::ServerField(output_type_name)
        }
        DefinedField::ResolverField(resolver_field_type) => DefinedField::ResolverField(
            validate_resolver_fragment(schema_data, resolver_field_type, &empty_field)?,
        ),
    };
    Ok(SchemaField {
        description: empty_field.description,
        name: empty_field.name,
        id: empty_field.id,
        field_type,
        parent_type_id: empty_field.parent_type_id,
    })
}

fn validate_server_field_type_exists_and_is_output_type(
    schema_data: &SchemaData,
    server_field_type: &TypeAnnotation<UnvalidatedTypeName>,
    field: &SchemaField<()>,
) -> ValidateSchemaResult<TypeAnnotation<OutputTypeId>> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(server_field_type.inner()) {
        // Why do we need to clone here? Can we avoid this?
        Some(type_id) => server_field_type.clone().and_then(|_| {
            type_id.as_output_type_id().ok_or_else(|| {
                let parent_type = schema_data.lookup_type_with_fields(field.parent_type_id);
                ValidateSchemaError::FieldTypenameIsInputObject {
                    parent_type_name: parent_type.name(),
                    field_name: field.name,
                    field_type: *server_field_type.inner(),
                }
            })
        }),
        None => Err(ValidateSchemaError::FieldTypenameDoesNotExist {
            parent_type_name: schema_data
                .lookup_type_with_fields(field.parent_type_id)
                .name(),
            field_name: field.name,
            field_type: *server_field_type.inner(),
        }),
    }
}

fn validate_resolver_fragment(
    schema_data: &SchemaData,
    resolver_field_type: SchemaResolverDefinitionInfo<(), (), UnvalidatedTypeName>,
    field: &SchemaField<()>,
) -> ValidateSchemaResult<ValidatedSchemaResolverDefinitionInfo> {
    let variable_definitions =
        validate_variable_definitions(schema_data, resolver_field_type.variable_definitions)?;

    match resolver_field_type.selection_set_and_unwraps {
        Some(selection_set_and_unwraps) => {
            let parent_type = schema_data.lookup_type_with_fields(field.parent_type_id);
            let SelectionSetAndUnwraps {
                selection_set,
                unwraps,
            } = selection_set_and_unwraps;
            let selection_set = validate_resolver_definition_selections_exist_and_types_match(
                schema_data,
                selection_set,
                parent_type,
            )
            .map_err(|err| {
                validate_selections_error_to_validate_schema_error(err, parent_type, field)
            })?;
            Ok(SchemaResolverDefinitionInfo {
                resolver_definition_path: resolver_field_type.resolver_definition_path,
                selection_set_and_unwraps: Some(SelectionSetAndUnwraps {
                    selection_set,
                    unwraps,
                }),
                field_id: resolver_field_type.field_id,
                variant: resolver_field_type.variant,
                is_fetchable: resolver_field_type.is_fetchable,
                variable_definitions,
                type_and_field: resolver_field_type.type_and_field,
                has_associated_js_function: resolver_field_type.has_associated_js_function,
            })
        }
        None => Ok(SchemaResolverDefinitionInfo {
            resolver_definition_path: resolver_field_type.resolver_definition_path,
            selection_set_and_unwraps: None,
            field_id: resolver_field_type.field_id,
            variant: resolver_field_type.variant,
            is_fetchable: resolver_field_type.is_fetchable,
            variable_definitions,
            type_and_field: resolver_field_type.type_and_field,
            has_associated_js_function: resolver_field_type.has_associated_js_function,
        }),
    }
}

fn validate_variable_definitions(
    schema_data: &SchemaData,
    variable_definitions: Vec<WithSpan<VariableDefinition<UnvalidatedTypeName>>>,
) -> ValidateSchemaResult<Vec<WithSpan<ValidatedVariableDefinition>>> {
    variable_definitions
        .into_iter()
        .map(|x| {
            x.and_then(|vd| {
                // TODO this should be doable in the error branch
                let type_string = vd.type_.to_string();
                let inner_type = *vd.type_.inner();
                Ok(VariableDefinition {
                    name: vd.name,
                    type_: vd.type_.and_then(|type_name| {
                        match schema_data.defined_types.get(&type_name) {
                            Some(type_id) => type_id.as_input_type_id().ok_or_else(|| {
                                ValidateSchemaError::VariableDefinitionInnerTypeIsOutputType {
                                    variable_name: vd.name.item,
                                    type_: type_string,
                                }
                            }),
                            None => Err(
                                ValidateSchemaError::VariableDefinitionInnerTypeDoesNotExist {
                                    variable_name: vd.name.item,
                                    type_: type_string,
                                    inner_type,
                                },
                            ),
                        }
                    })?,
                })
            })
        })
        .collect()
}

fn validate_selections_error_to_validate_schema_error(
    err: ValidateSelectionsError,
    parent_type: SchemaTypeWithFields,
    field: &SchemaField<()>,
) -> ValidateSchemaError {
    match err {
        ValidateSelectionsError::FieldDoesNotExist(field_parent_type_name, field_name) => {
            ValidateSchemaError::ResolverSelectionFieldDoesNotExist {
                resolver_parent_type_name: parent_type.name(),
                resolver_field_name: field.name,
                field_parent_type_name,
                field_name,
            }
        }
        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
            field_parent_type_name: parent_type_name,
            field_name,
            target_type,
            target_type_name,
        } => ValidateSchemaError::ResolverSelectionFieldIsNotScalar {
            resolver_parent_type_name: parent_type.name(),
            resolver_field_name: field.name,
            field_parent_type_name: parent_type_name,
            field_name,
            field_type: target_type,
            target_type_name,
        },
        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
            field_parent_type_name,
            field_name,
            target_type,
            target_type_name,
        } => ValidateSchemaError::ResolverSelectionFieldIsScalar {
            resolver_parent_type_name: parent_type.name(),
            resolver_field_name: field.name,
            field_parent_type_name,
            field_name,
            field_type: target_type,
            target_type_name,
        },
        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsResolver {
            field_parent_type_name,
            field_name,
        } => ValidateSchemaError::ResolverSelectionFieldIsResolver {
            resolver_parent_type_name: parent_type.name(),
            resolver_field_name: field.name,
            field_parent_type_name,
            field_name,
        },
    }
}

type ValidateSelectionsResult<T> = Result<T, ValidateSelectionsError>;

enum ValidateSelectionsError {
    FieldDoesNotExist(TypeWithFieldsName, FieldDefinitionName),
    FieldSelectedAsScalarButTypeIsNotScalar {
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsScalar {
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsResolver {
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
    },
}

fn validate_resolver_definition_selections_exist_and_types_match(
    schema_data: &SchemaData,
    selection_set: Vec<WithSpan<Selection<(), ()>>>,
    parent_type: SchemaTypeWithFields,
) -> Result<Vec<WithSpan<ValidatedSelection>>, ValidateSelectionsError> {
    // Currently, we only check that each field exists and has an appropriate type, not that
    // there are no selection conflicts due to aliases or parameters.

    Ok(selection_set
        .into_iter()
        .map(|selection| {
            validate_resolver_definition_selection_exists_and_type_matches(
                selection,
                parent_type,
                schema_data,
            )
        })
        .collect::<Result<_, _>>()?)
}

fn validate_resolver_definition_selection_exists_and_type_matches(
    selection: WithSpan<Selection<(), ()>>,
    parent_type: SchemaTypeWithFields,
    schema_data: &SchemaData,
) -> Result<WithSpan<ValidatedSelection>, ValidateSelectionsError> {
    selection.and_then(|selection| {
        selection.and_then(&mut |field_selection| {
            field_selection.and_then(
                &mut |scalar_field_selection| {
                    validate_field_type_exists_and_is_scalar(
                        parent_type.encountered_field_names(),
                        schema_data,
                        parent_type,
                        scalar_field_selection,
                    )
                },
                &mut |linked_field_selection| {
                    validate_field_type_exists_and_is_linked(
                        parent_type.encountered_field_names(),
                        schema_data,
                        parent_type,
                        linked_field_selection,
                    )
                },
            )
        })
    })
}

/// Given that we selected a scalar field, the field should exist on the parent,
/// and type should be a resolver (which is a scalar) or a server scalar type.
fn validate_field_type_exists_and_is_scalar(
    parent_fields: &HashMap<
        FieldDefinitionName,
        DefinedField<TypeAnnotation<UnvalidatedTypeName>, ScalarFieldName>,
    >,
    schema_data: &SchemaData,
    parent_type: SchemaTypeWithFields,
    scalar_field_selection: ScalarFieldSelection<()>,
) -> ValidateSelectionsResult<ScalarFieldSelection<ValidatedDefinedField>> {
    let scalar_field_name = scalar_field_selection.name.item.into();
    match parent_fields.get(&scalar_field_name) {
        Some(defined_field_type) => match defined_field_type {
            DefinedField::ServerField(server_field_name) => {
                let field_type_id = *schema_data
                    .defined_types
                    .get(server_field_name.inner())
                    .expect("Expected field type to be defined, which I think was validated earlier, probably indicates a bug in Boulton");
                match field_type_id {
                    TypeId::Scalar(scalar_id) => Ok(ScalarFieldSelection {
                        name: scalar_field_selection.name,
                        field: DefinedField::ServerField(TypeWithoutFieldsId::Scalar(scalar_id)),
                        reader_alias: scalar_field_selection.reader_alias,
                        normalization_alias: scalar_field_selection.normalization_alias,
                        unwraps: scalar_field_selection.unwraps,
                        arguments: scalar_field_selection.arguments,
                    }),
                    TypeId::Object(_) => Err(
                        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
                            field_parent_type_name: parent_type.name(),
                            field_name: scalar_field_name,
                            target_type: "an object",
                            target_type_name: *server_field_name.inner(),
                        },
                    ),
                }
            }
            DefinedField::ResolverField(_) => Ok(ScalarFieldSelection {
                name: scalar_field_selection.name,
                reader_alias: scalar_field_selection.reader_alias,
                unwraps: scalar_field_selection.unwraps,
                field: DefinedField::ResolverField(()),
                arguments: scalar_field_selection.arguments,
                normalization_alias: scalar_field_selection.normalization_alias,
            }),
        },
        None => Err(ValidateSelectionsError::FieldDoesNotExist(
            parent_type.name(),
            scalar_field_name,
        )),
    }
}

/// Given that we selected a linked field, the field should exist on the parent,
/// and type should be a server interface, object or union.
fn validate_field_type_exists_and_is_linked(
    parent_fields: &HashMap<
        FieldDefinitionName,
        DefinedField<TypeAnnotation<UnvalidatedTypeName>, ScalarFieldName>,
    >,
    schema_data: &SchemaData,
    parent_type: SchemaTypeWithFields,
    linked_field_selection: LinkedFieldSelection<(), ()>,
) -> ValidateSelectionsResult<LinkedFieldSelection<ValidatedDefinedField, TypeWithFieldsId>> {
    let linked_field_name = linked_field_selection.name.item.into();
    match parent_fields.get(&linked_field_name) {
        Some(defined_field_type) => match defined_field_type {
            DefinedField::ServerField(server_field_name) => {
                let field_type_id = *schema_data
                    .defined_types
                    .get(server_field_name.inner())
                    .expect("Expected field type to be defined, which I think was validated earlier, probably indicates a bug in Boulton");
                match field_type_id {
                    TypeId::Scalar(_) => Err(
                        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
                            field_parent_type_name: parent_type.name(),
                            field_name: linked_field_name,
                            target_type: "a scalar",
                            target_type_name: *server_field_name.inner(),
                        },
                    ),
                    TypeId::Object(object_id) => {
                        let object = schema_data.objects.get(object_id.as_usize()).unwrap();
                        Ok(LinkedFieldSelection {
                            name: linked_field_selection.name,
                            reader_alias: linked_field_selection.reader_alias,
                            normalization_alias: linked_field_selection.normalization_alias,
                            selection_set_and_unwraps: linked_field_selection
                                .selection_set_and_unwraps
                                .and_then(&mut |selection| {
                                    validate_resolver_definition_selection_exists_and_type_matches(
                                        selection,
                                        SchemaTypeWithFields::Object(object),
                                        schema_data,
                                    )
                                })?,
                            field: TypeWithFieldsId::Object(object_id),
                            arguments: linked_field_selection.arguments,
                        })
                    }
                }
            }
            DefinedField::ResolverField(_) => Err(
                ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsResolver {
                    field_parent_type_name: parent_type.name(),
                    field_name: linked_field_name,
                },
            ),
        },
        None => Err(ValidateSelectionsError::FieldDoesNotExist(
            parent_type.name(),
            linked_field_name,
        )),
    }
}

type ValidateSchemaResult<T> = Result<T, ValidateSchemaError>;

#[derive(Debug, Error)]
pub enum ValidateSchemaError {
    #[error(
        "The field `{parent_type_name}.{field_name}` has type `{field_type}`, which does not exist."
    )]
    FieldTypenameDoesNotExist {
        parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        field_type: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}`, but that field does not exist on `{field_parent_type_name}`"
    )]
    ResolverSelectionFieldDoesNotExist {
        resolver_parent_type_name: TypeWithFieldsName,
        resolver_field_name: FieldDefinitionName,
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a scalar, but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ResolverSelectionFieldIsNotScalar {
        resolver_parent_type_name: TypeWithFieldsName,
        resolver_field_name: FieldDefinitionName,
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a scalar, but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ResolverSelectionFieldIsScalar {
        resolver_parent_type_name: TypeWithFieldsName,
        resolver_field_name: FieldDefinitionName,
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a linked field, but that field is a resolver, which can only be selected as a scalar."
    )]
    ResolverSelectionFieldIsResolver {
        resolver_parent_type_name: TypeWithFieldsName,
        resolver_field_name: FieldDefinitionName,
        field_parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
    },

    #[error(
        "The field `{parent_type_name}.{field_name}` has type `{field_type}`, which is an InputObject. It should be an output type."
    )]
    FieldTypenameIsInputObject {
        parent_type_name: TypeWithFieldsName,
        field_name: FieldDefinitionName,
        field_type: UnvalidatedTypeName,
    },

    #[error(
        "The variable `{variable_name}` has type `{type_}`, which is an output type. It should be an input type."
    )]
    VariableDefinitionInnerTypeIsOutputType {
        variable_name: VariableName,
        type_: String,
    },

    #[error(
        "The variable `{variable_name}` has type `{type_}`, but the inner type `{inner_type}` does not exist."
    )]
    VariableDefinitionInnerTypeDoesNotExist {
        variable_name: VariableName,
        type_: String,
        inner_type: UnvalidatedTypeName,
    },
}
