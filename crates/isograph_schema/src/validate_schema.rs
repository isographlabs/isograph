use std::collections::HashMap;

use common_lang_types::{
    IsographObjectTypeName, ScalarFieldName, SelectableFieldName, UnvalidatedTypeName,
    VariableName, WithSpan,
};
use graphql_lang_types::{NamedTypeAnnotation, TypeAnnotation};
use isograph_lang_types::{
    DefinedTypeId, InputTypeId, LinkedFieldSelection, ObjectId, OutputTypeId, ResolverFieldId,
    ScalarFieldSelection, ScalarId, Selection, ServerFieldId, VariableDefinition,
};
use thiserror::Error;

use crate::{
    refetched_paths::refetched_paths_with_path, DefinedField, NameAndArguments, PathToRefetchField,
    Schema, SchemaData, SchemaIdField, SchemaObject, SchemaResolver, SchemaServerField,
    UnvalidatedObjectFieldInfo, UnvalidatedSchema, UnvalidatedSchemaData, UnvalidatedSchemaField,
    UnvalidatedSchemaResolver, UnvalidatedSchemaServerField,
};

pub type ValidatedSchemaField = SchemaServerField<TypeAnnotation<OutputTypeId>>;

pub type ValidatedSelection = Selection<ValidatedScalarDefinedField, ObjectId>;

pub type ValidatedVariableDefinition = VariableDefinition<InputTypeId>;
pub type ValidatedSchemaResolver =
    SchemaResolver<ValidatedScalarDefinedField, ObjectId, InputTypeId>;

/// The validated defined field that shows up in the encountered field generic.
pub type ValidatedEncounteredDefinedField = DefinedField<ServerFieldId, ResolverFieldId>;
/// The validated defined field that shows up in the TScalarField generic.
pub type ValidatedScalarDefinedField = DefinedField<ServerFieldId, ResolverFieldId>;

pub type ValidatedSchemaObject = SchemaObject<ValidatedEncounteredDefinedField>;

pub type ValidatedSchemaIdField = SchemaIdField<NamedTypeAnnotation<ScalarId>>;

pub type ValidatedSchema = Schema<
    // Fields contain a field_type: TypeAnnotation<TFieldAssociatedType>
    OutputTypeId,
    // The associated data type of scalars in resolvers' selection sets and unwraps
    ValidatedScalarDefinedField,
    // The associated data type of linked fields in resolvers' selection sets and unwraps
    ObjectId,
    // The associated data type of resolvers' variable definitions
    InputTypeId,
    // On objects, what does the HashMap of encountered types contain
    ValidatedEncounteredDefinedField,
>;

impl ValidatedSchema {
    pub fn validate_and_construct(
        unvalidated_schema: UnvalidatedSchema,
    ) -> ValidateSchemaResult<Self> {
        let Schema {
            fields,
            resolvers,
            schema_data,
            id_type_id: id_type,
            string_type_id: string_type,
            query_type_id,
            float_type_id,
            boolean_type_id,
            int_type_id,
        } = unvalidated_schema;

        let updated_fields = validate_and_transform_fields(fields, &schema_data)?;
        let updated_resolvers =
            validate_and_transform_resolvers(resolvers, &schema_data, &updated_fields)?;

        let SchemaData {
            objects,
            scalars,
            defined_types,
        } = schema_data;

        let objects = objects
            .into_iter()
            .map(|object| transform_object_field_ids(&updated_fields, &updated_resolvers, object))
            .collect();

        Ok(Self {
            fields: updated_fields,
            resolvers: updated_resolvers,
            schema_data: SchemaData {
                objects,
                scalars,
                defined_types,
            },
            id_type_id: id_type,
            string_type_id: string_type,
            query_type_id,
            float_type_id,
            boolean_type_id,
            int_type_id,
        })
    }
}

fn transform_object_field_ids(
    schema_fields: &[ValidatedSchemaField],
    schema_resolvers: &[ValidatedSchemaResolver],
    object: SchemaObject<UnvalidatedObjectFieldInfo>,
) -> SchemaObject<ValidatedEncounteredDefinedField> {
    let SchemaObject {
        name,
        server_fields,
        description,
        id,
        encountered_fields: unvalidated_encountered_fields,
        resolvers,
        valid_refinements,
        id_field,
    } = object;

    let validated_encountered_fields = unvalidated_encountered_fields
        .into_iter()
        .map(|(encountered_field_name, _)| {
            for server_field_id in server_fields.iter() {
                let field = &schema_fields[server_field_id.as_usize()];
                if field.name.item == encountered_field_name {
                    return (encountered_field_name, DefinedField::ServerField(field.id));
                }
            }
            for resolver in resolvers.iter() {
                let resolver = &schema_resolvers[resolver.as_usize()];
                if resolver.name == encountered_field_name {
                    return (
                        encountered_field_name,
                        DefinedField::ResolverField(resolver.id),
                    );
                }
            }
            panic!(
                "field {:?} not found, probably a isograph bug but we should confirm",
                encountered_field_name
            );
        })
        .collect();

    SchemaObject {
        description,
        name,
        id,
        server_fields,
        encountered_fields: validated_encountered_fields,
        resolvers,
        valid_refinements,
        id_field,
    }
}

fn validate_and_transform_fields(
    fields: Vec<UnvalidatedSchemaField>,
    schema_data: &UnvalidatedSchemaData,
) -> ValidateSchemaResult<Vec<ValidatedSchemaField>> {
    fields
        .into_iter()
        .map(|field| validate_and_transform_field(field, schema_data))
        .collect()
}

fn validate_and_transform_field(
    field: UnvalidatedSchemaField,
    schema_data: &UnvalidatedSchemaData,
) -> ValidateSchemaResult<ValidatedSchemaField> {
    // TODO rewrite as field.map(...).transpose()
    let (empty_field, server_field_type) = field.split();
    let field_type = validate_server_field_type_exists_and_is_output_type(
        schema_data,
        &server_field_type,
        &empty_field,
    )?;
    Ok(SchemaServerField {
        description: empty_field.description,
        name: empty_field.name,
        id: empty_field.id,
        associated_data: field_type,
        parent_type_id: empty_field.parent_type_id,
        arguments: empty_field.arguments,
    })
}

fn validate_server_field_type_exists_and_is_output_type(
    schema_data: &UnvalidatedSchemaData,
    server_field_type: &TypeAnnotation<UnvalidatedTypeName>,
    field: &SchemaServerField<()>,
) -> ValidateSchemaResult<TypeAnnotation<OutputTypeId>> {
    // look up the item in defined_types. If it's not there, error.
    match schema_data.defined_types.get(server_field_type.inner()) {
        // Why do we need to clone here? Can we avoid this?
        Some(type_id) => server_field_type.clone().and_then(|_| {
            type_id.as_output_type_id().ok_or_else(|| {
                let parent_type = schema_data.object(field.parent_type_id);
                ValidateSchemaError::FieldTypenameIsInputObject {
                    parent_type_name: parent_type.name,
                    field_name: field.name.item,
                    field_type: *server_field_type.inner(),
                }
            })
        }),
        None => Err(ValidateSchemaError::FieldTypenameDoesNotExist {
            parent_type_name: schema_data.object(field.parent_type_id).name,
            field_name: field.name.item,
            field_type: *server_field_type.inner(),
        }),
    }
}

fn validate_and_transform_resolvers(
    resolvers: Vec<UnvalidatedSchemaResolver>,
    schema_data: &UnvalidatedSchemaData,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSchemaResult<Vec<ValidatedSchemaResolver>> {
    resolvers
        .into_iter()
        .map(|resolver| validate_resolver_fragment(schema_data, resolver, server_fields))
        .collect()
}

fn validate_resolver_fragment(
    schema_data: &UnvalidatedSchemaData,
    unvalidated_resolver: UnvalidatedSchemaResolver,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSchemaResult<ValidatedSchemaResolver> {
    let variable_definitions =
        validate_variable_definitions(schema_data, unvalidated_resolver.variable_definitions)?;

    match unvalidated_resolver.selection_set_and_unwraps {
        Some((selection_set, unwraps)) => {
            let parent_object = schema_data.object(unvalidated_resolver.parent_object_id);
            let selection_set = validate_resolver_definition_selections_exist_and_types_match(
                schema_data,
                selection_set,
                parent_object,
                server_fields,
            )
            .map_err(|err| {
                validate_selections_error_to_validate_schema_error(
                    err,
                    parent_object,
                    unvalidated_resolver.name,
                )
            })?;
            Ok(SchemaResolver {
                description: unvalidated_resolver.description,
                name: unvalidated_resolver.name,
                id: unvalidated_resolver.id,
                selection_set_and_unwraps: Some((selection_set, unwraps)),
                variant: unvalidated_resolver.variant,
                artifact_kind: unvalidated_resolver.artifact_kind,
                variable_definitions,
                type_and_field: unvalidated_resolver.type_and_field,
                parent_object_id: unvalidated_resolver.parent_object_id,
                action_kind: unvalidated_resolver.action_kind,
            })
        }
        None => Ok(SchemaResolver {
            description: unvalidated_resolver.description,
            name: unvalidated_resolver.name,
            id: unvalidated_resolver.id,
            selection_set_and_unwraps: None,
            variant: unvalidated_resolver.variant,
            artifact_kind: unvalidated_resolver.artifact_kind,
            variable_definitions,
            type_and_field: unvalidated_resolver.type_and_field,
            parent_object_id: unvalidated_resolver.parent_object_id,
            action_kind: unvalidated_resolver.action_kind,
        }),
    }
}

fn validate_variable_definitions(
    schema_data: &UnvalidatedSchemaData,
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
    parent_object: &SchemaObject<UnvalidatedObjectFieldInfo>,
    resolver_field_name: SelectableFieldName,
) -> ValidateSchemaError {
    match err {
        ValidateSelectionsError::FieldDoesNotExist(field_parent_type_name, field_name) => {
            ValidateSchemaError::ResolverSelectionFieldDoesNotExist {
                resolver_parent_type_name: parent_object.name,
                resolver_field_name,
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
            resolver_parent_type_name: parent_object.name,
            resolver_field_name,
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
            resolver_parent_type_name: parent_object.name,
            resolver_field_name,
            field_parent_type_name,
            field_name,
            field_type: target_type,
            target_type_name,
        },
        ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsResolver {
            field_parent_type_name,
            field_name,
        } => ValidateSchemaError::ResolverSelectionFieldIsResolver {
            resolver_parent_type_name: parent_object.name,
            resolver_field_name,
            field_parent_type_name,
            field_name,
        },
    }
}

type ValidateSelectionsResult<T> = Result<T, ValidateSelectionsError>;

#[allow(unused)]
#[derive(Debug)]
enum ValidateSelectionsError {
    FieldDoesNotExist(IsographObjectTypeName, SelectableFieldName),
    FieldSelectedAsScalarButTypeIsNotScalar {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsScalar {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        target_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },
    FieldSelectedAsLinkedButTypeIsResolver {
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },
}

fn validate_resolver_definition_selections_exist_and_types_match(
    schema_data: &UnvalidatedSchemaData,
    selection_set: Vec<WithSpan<Selection<(), ()>>>,
    parent_object: &SchemaObject<UnvalidatedObjectFieldInfo>,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSelectionsResult<Vec<WithSpan<ValidatedSelection>>> {
    // Currently, we only check that each field exists and has an appropriate type, not that
    // there are no selection conflicts due to aliases or parameters.

    Ok(selection_set
        .into_iter()
        .map(|selection| {
            validate_resolver_definition_selection_exists_and_type_matches(
                selection,
                parent_object,
                schema_data,
                server_fields,
            )
        })
        .collect::<Result<_, _>>()?)
}

fn validate_resolver_definition_selection_exists_and_type_matches(
    selection: WithSpan<Selection<(), ()>>,
    parent_object: &SchemaObject<UnvalidatedObjectFieldInfo>,
    schema_data: &UnvalidatedSchemaData,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSelectionsResult<WithSpan<ValidatedSelection>> {
    selection.and_then(|selection| {
        selection.and_then(&mut |field_selection| {
            field_selection.and_then(
                &mut |scalar_field_selection| {
                    validate_field_type_exists_and_is_scalar(
                        &parent_object.encountered_fields,
                        schema_data,
                        parent_object,
                        scalar_field_selection,
                        server_fields,
                    )
                },
                &mut |linked_field_selection| {
                    validate_field_type_exists_and_is_linked(
                        &parent_object.encountered_fields,
                        schema_data,
                        parent_object,
                        linked_field_selection,
                        server_fields,
                    )
                },
            )
        })
    })
}

/// Given that we selected a scalar field, the field should exist on the parent,
/// and type should be a resolver (which is a scalar) or a server scalar type.
fn validate_field_type_exists_and_is_scalar(
    parent_encountered_fields: &HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    schema_data: &UnvalidatedSchemaData,
    parent_object: &SchemaObject<UnvalidatedObjectFieldInfo>,
    scalar_field_selection: ScalarFieldSelection<()>,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSelectionsResult<ScalarFieldSelection<ValidatedScalarDefinedField>> {
    let scalar_field_name = scalar_field_selection.name.item.into();
    match parent_encountered_fields.get(&scalar_field_name) {
        Some(defined_field_type) => match defined_field_type {
            DefinedField::ServerField(server_field_name) => {
                let field_type_id = *schema_data
                    .defined_types
                    .get(server_field_name.inner())
                    .expect(
                        "Expected field type to be defined, which I think \
                        was validated earlier, probably indicates a bug in Isograph",
                    );
                match field_type_id {
                    DefinedTypeId::Scalar(_scalar_id) => Ok(ScalarFieldSelection {
                        name: scalar_field_selection.name,
                        associated_data: DefinedField::ServerField(
                            find_server_field_id(
                                server_fields,
                                scalar_field_selection.name.item,
                                &parent_object.server_fields,
                            )
                            .expect("Expected to find scalar field, this probably indicates a bug in Isograph"),
                        ),
                        reader_alias: scalar_field_selection.reader_alias,
                        normalization_alias: scalar_field_selection.normalization_alias,
                        unwraps: scalar_field_selection.unwraps,
                        arguments: scalar_field_selection.arguments,
                    }),
                    DefinedTypeId::Object(_) => Err(
                        ValidateSelectionsError::FieldSelectedAsScalarButTypeIsNotScalar {
                            field_parent_type_name: parent_object.name,
                            field_name: scalar_field_name,
                            target_type: "an object",
                            target_type_name: *server_field_name.inner(),
                        },
                    ),
                }
            }
            DefinedField::ResolverField(resolver_field_id) => {
                // TODO confirm this works if resolver_name is an alias
                Ok(ScalarFieldSelection {
                    name: scalar_field_selection.name,
                    reader_alias: scalar_field_selection.reader_alias,
                    unwraps: scalar_field_selection.unwraps,
                    associated_data: DefinedField::ResolverField(*resolver_field_id),
                    arguments: scalar_field_selection.arguments,
                    normalization_alias: scalar_field_selection.normalization_alias,
                })
            }
        },
        None => Err(ValidateSelectionsError::FieldDoesNotExist(
            parent_object.name,
            scalar_field_name,
        )),
    }
}

/// Given that we selected a linked field, the field should exist on the parent,
/// and type should be a server interface, object or union.
fn validate_field_type_exists_and_is_linked(
    parent_fields: &HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    schema_data: &UnvalidatedSchemaData,
    parent_object: &SchemaObject<UnvalidatedObjectFieldInfo>,
    linked_field_selection: LinkedFieldSelection<(), ()>,
    server_fields: &Vec<UnvalidatedSchemaServerField>,
) -> ValidateSelectionsResult<LinkedFieldSelection<ValidatedScalarDefinedField, ObjectId>> {
    let linked_field_name = linked_field_selection.name.item.into();
    match parent_fields.get(&linked_field_name) {
        Some(defined_field_type) => {
            match defined_field_type {
                DefinedField::ServerField(server_field_name) => {
                    let field_type_id = *schema_data
                        .defined_types
                        .get(server_field_name.inner())
                        .expect(
                            "Expected field type to be defined, which I \
                            think was validated earlier, probably indicates a bug in Isograph",
                        );
                    match field_type_id {
                        DefinedTypeId::Scalar(_) => Err(
                            ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsScalar {
                                field_parent_type_name: parent_object.name,
                                field_name: linked_field_name,
                                target_type: "a scalar",
                                target_type_name: *server_field_name.inner(),
                            },
                        ),
                        DefinedTypeId::Object(object_id) => {
                            let object = schema_data.objects.get(object_id.as_usize()).unwrap();
                            Ok(LinkedFieldSelection {
                                name: linked_field_selection.name,
                                reader_alias: linked_field_selection.reader_alias,
                                normalization_alias: linked_field_selection.normalization_alias,
                                selection_set: linked_field_selection.selection_set.into_iter().map(
                                    |selection| {
                                        validate_resolver_definition_selection_exists_and_type_matches(
                                            selection,
                                            object,
                                            schema_data,
                                            server_fields
                                        )
                                    },
                                ).collect::<Result<Vec<_>, _>>()?,
                                unwraps: linked_field_selection.unwraps,
                                associated_data: object_id,
                                arguments: linked_field_selection.arguments,
                            })
                        }
                    }
                }
                DefinedField::ResolverField(_) => Err(
                    ValidateSelectionsError::FieldSelectedAsLinkedButTypeIsResolver {
                        field_parent_type_name: parent_object.name,
                        field_name: linked_field_name,
                    },
                ),
            }
        }
        None => Err(ValidateSelectionsError::FieldDoesNotExist(
            parent_object.name,
            linked_field_name,
        )),
    }
}

fn find_server_field_id(
    server_fields: &Vec<UnvalidatedSchemaServerField>,
    field_name: ScalarFieldName,
    parent_server_fields: &[ServerFieldId],
) -> Option<ServerFieldId> {
    parent_server_fields.iter().find_map(|server_field_id| {
        let server_field = &server_fields[server_field_id.as_usize()];
        if server_field.name.item == field_name.into() {
            Some(*server_field_id)
        } else {
            None
        }
    })
}

type ValidateSchemaResult<T> = Result<T, ValidateSchemaError>;

#[derive(Debug, Error)]
pub enum ValidateSchemaError {
    #[error(
        "The field `{parent_type_name}.{field_name}` has type `{field_type}`, which does not exist."
    )]
    FieldTypenameDoesNotExist {
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}`, but that field does not exist on `{field_parent_type_name}`"
    )]
    ResolverSelectionFieldDoesNotExist {
        resolver_parent_type_name: IsographObjectTypeName,
        resolver_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a scalar, but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ResolverSelectionFieldIsNotScalar {
        resolver_parent_type_name: IsographObjectTypeName,
        resolver_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a linked field, but that field's type is `{target_type_name}`, which is {field_type}."
    )]
    ResolverSelectionFieldIsScalar {
        resolver_parent_type_name: IsographObjectTypeName,
        resolver_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
        field_type: &'static str,
        target_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The resolver `{resolver_parent_type_name}.{resolver_field_name}` attempts to select `{field_parent_type_name}.{field_name}` as a linked field, but that field is a resolver, which can only be selected as a scalar."
    )]
    ResolverSelectionFieldIsResolver {
        resolver_parent_type_name: IsographObjectTypeName,
        resolver_field_name: SelectableFieldName,
        field_parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
    },

    #[error(
        "The field `{parent_type_name}.{field_name}` has type `{field_type}`, which is an InputObject. It should be an output type."
    )]
    FieldTypenameIsInputObject {
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableFieldName,
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

pub fn refetched_paths_for_resolver(
    schema_resolver: &ValidatedSchemaResolver,
    schema: &ValidatedSchema,
    path: &mut Vec<NameAndArguments>,
) -> Vec<PathToRefetchField> {
    let path_set = match &schema_resolver.selection_set_and_unwraps {
        Some((selection_set, _)) => refetched_paths_with_path(&selection_set, schema, path),
        None => panic!("unexpected non-existent selection set on resolver"),
    };
    let mut paths: Vec<_> = path_set.into_iter().collect();
    paths.sort();
    paths
}
