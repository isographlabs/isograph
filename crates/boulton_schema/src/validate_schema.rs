use std::collections::HashMap;

use boulton_lang_types::{
    LinkedFieldSelection, ScalarFieldSelection, Selection, SelectionSetAndUnwraps,
};
use common_lang_types::{
    DefinedField, FieldDefinitionName, HasName, LinkedFieldName, OutputTypeId, ScalarFieldName,
    TypeId, TypeWithFieldsId, TypeWithFieldsName, TypeWithoutFieldsId, UnvalidatedTypeName,
    WithSpan,
};
use thiserror::Error;

use crate::{
    Schema, SchemaData, SchemaField, SchemaResolverDefinitionInfo, SchemaTypeWithFields,
    UnvalidatedSchemaField,
};

pub type ValidatedSchemaField = SchemaField<
    DefinedField<
        OutputTypeId,
        SchemaResolverDefinitionInfo<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>,
    >,
>;

pub type ValidatedSelectionSetAndUnwraps =
    SelectionSetAndUnwraps<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>;

pub type ValidatedSelection = Selection<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>;

pub type ValidatedSchemaResolverDefinitionInfo =
    SchemaResolverDefinitionInfo<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>;

pub type ValidatedSchema =
    Schema<OutputTypeId, DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>;

impl ValidatedSchema {
    pub fn validate_and_construct(
        unvalidated_schema: Schema<UnvalidatedTypeName, ScalarFieldName, LinkedFieldName>,
    ) -> ValidateSchemaResult<Self> {
        let Schema {
            fields,
            schema_data,
            id_type,
            string_type,
            query_type,
        } = unvalidated_schema;

        let updated_fields = validate_and_transform_fields(fields, &schema_data)?;

        Ok(Self {
            fields: updated_fields,
            schema_data,
            id_type,
            string_type,
            query_type,
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
    server_field_type: &UnvalidatedTypeName,
    field: &SchemaField<()>,
) -> ValidateSchemaResult<OutputTypeId> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(&server_field_type) {
        Some(type_id) => type_id.as_output_type_id().ok_or_else(|| {
            let parent_type = schema_data.lookup_type_with_fields(field.parent_type_id);
            ValidateSchemaError::FieldTypenameIsInputObject {
                parent_type_name: parent_type.name(),
                field_name: field.name,
                field_type: *server_field_type,
            }
        }),
        None => Err(ValidateSchemaError::FieldTypenameDoesNotExist {
            parent_type_name: schema_data
                .lookup_type_with_fields(field.parent_type_id)
                .name(),
            field_name: field.name,
            field_type: *server_field_type,
        }),
    }
}

fn validate_resolver_fragment(
    schema_data: &SchemaData,
    resolver_field_type: SchemaResolverDefinitionInfo<ScalarFieldName, LinkedFieldName>,
    field: &SchemaField<()>,
) -> ValidateSchemaResult<
    SchemaResolverDefinitionInfo<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>,
> {
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
            })
        }
        None => Ok(SchemaResolverDefinitionInfo {
            resolver_definition_path: resolver_field_type.resolver_definition_path,
            selection_set_and_unwraps: None,
            field_id: resolver_field_type.field_id,
        }),
    }
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
    selection_set: Vec<WithSpan<Selection<ScalarFieldName, LinkedFieldName>>>,
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
    selection: WithSpan<Selection<ScalarFieldName, LinkedFieldName>>,
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
        DefinedField<UnvalidatedTypeName, ScalarFieldName>,
    >,
    schema_data: &SchemaData,
    parent_type: SchemaTypeWithFields,
    scalar_field_selection: ScalarFieldSelection<ScalarFieldName>,
) -> ValidateSelectionsResult<ScalarFieldSelection<DefinedField<TypeWithoutFieldsId, ()>>> {
    let scalar_field_name = scalar_field_selection.field.item.into();
    match parent_fields.get(&scalar_field_name) {
        Some(defined_field_type) => match defined_field_type {
            DefinedField::ServerField(server_field_name) => {
                let field_type_id = *schema_data
                    .defined_types
                    .get(server_field_name)
                    .expect("Expected field type to be defined, which I think was validated earlier, probably indicates a bug in Boulton");
                match field_type_id {
                    TypeId::Scalar(scalar_id) => Ok(ScalarFieldSelection {
                        name: scalar_field_selection.name,
                        field: scalar_field_selection.field.map(|_| {
                            DefinedField::ServerField(TypeWithoutFieldsId::Scalar(scalar_id))
                        }),
                        alias: scalar_field_selection.alias,
                        unwraps: scalar_field_selection.unwraps,
                    }),
                    TypeId::Object(_) => Err(
                        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
                            field_parent_type_name: parent_type.name(),
                            field_name: scalar_field_name,
                            target_type: "an object",
                            target_type_name: *server_field_name,
                        },
                    ),
                }
            }
            DefinedField::ResolverField(_) => Ok(ScalarFieldSelection {
                name: scalar_field_selection.name,
                alias: scalar_field_selection.alias,
                unwraps: scalar_field_selection.unwraps,
                field: scalar_field_selection
                    .field
                    .map(|_| DefinedField::ResolverField(())),
            }),
        },
        None => {
            eprintln!(")1 {:#?}", parent_fields);
            Err(ValidateSelectionsError::FieldDoesNotExist(
                parent_type.name(),
                scalar_field_name,
            ))
        }
    }
}

/// Given that we selected a linked field, the field should exist on the parent,
/// and type should be a server interface, object or union.
fn validate_field_type_exists_and_is_linked(
    parent_fields: &HashMap<
        FieldDefinitionName,
        DefinedField<UnvalidatedTypeName, ScalarFieldName>,
    >,
    schema_data: &SchemaData,
    parent_type: SchemaTypeWithFields,
    linked_field_selection: LinkedFieldSelection<ScalarFieldName, LinkedFieldName>,
) -> ValidateSelectionsResult<
    LinkedFieldSelection<DefinedField<TypeWithoutFieldsId, ()>, TypeWithFieldsId>,
> {
    let linked_field_name = linked_field_selection.field.item.into();
    match parent_fields.get(&linked_field_name) {
        Some(defined_field_type) => match defined_field_type {
            DefinedField::ServerField(server_field_name) => {
                let field_type_id = *schema_data
                    .defined_types
                    .get(server_field_name)
                    .expect("Expected field type to be defined, which I think was validated earlier, probably indicates a bug in Boulton");
                match field_type_id {
                    TypeId::Scalar(_) => Err(
                        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
                            field_parent_type_name: parent_type.name(),
                            field_name: linked_field_name,
                            target_type: "a scalar",
                            target_type_name: *server_field_name,
                        },
                    ),
                    TypeId::Object(object_id) => {
                        let object = schema_data.objects.get(object_id.as_usize()).unwrap();
                        Ok(LinkedFieldSelection {
                            name: linked_field_selection.name,
                            alias: linked_field_selection.alias,
                            selection_set_and_unwraps: linked_field_selection
                                .selection_set_and_unwraps
                                .and_then(&mut |selection| {
                                    validate_resolver_definition_selection_exists_and_type_matches(
                                        selection,
                                        SchemaTypeWithFields::Object(object),
                                        schema_data,
                                    )
                                })?,
                            field: linked_field_selection
                                .field
                                .map(|_| TypeWithFieldsId::Object(object_id)),
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
}
