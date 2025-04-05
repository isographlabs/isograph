use std::collections::HashMap;

use common_lang_types::{
    IsoLiteralText, IsographObjectTypeName, Location, ServerScalarSelectableName, TextSource,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use isograph_lang_types::{
    ClientScalarSelectableId, DefinitionLocation, EntrypointDeclaration, SelectionType,
    ServerEntityId, ServerObjectEntityId,
};

use thiserror::Error;

use crate::{NetworkProtocol, Schema};

pub fn validate_entrypoints<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    entrypoint_declarations: Vec<(TextSource, WithSpan<EntrypointDeclaration>)>,
) -> Result<
    HashMap<ClientScalarSelectableId, IsoLiteralText>,
    Vec<WithLocation<ValidateEntrypointDeclarationError>>,
> {
    let mut errors = vec![];
    let mut entrypoints = HashMap::new();
    for (text_source, entrypoint_declaration) in entrypoint_declarations {
        match validate_entrypoint_type_and_field(schema, text_source, entrypoint_declaration) {
            Ok(client_field_id) => {
                entrypoints.insert(
                    client_field_id,
                    entrypoint_declaration.item.iso_literal_text,
                );
            }
            Err(e) => {
                errors.push(e);
            }
        }
    }

    if errors.is_empty() {
        Ok(entrypoints)
    } else {
        Err(errors)
    }
}

fn validate_entrypoint_type_and_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    text_source: TextSource,
    entrypoint_declaration: WithSpan<EntrypointDeclaration>,
) -> Result<ClientScalarSelectableId, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_object_entity_id = validate_parent_object_entity_id(
        schema,
        entrypoint_declaration.item.parent_type,
        text_source,
    )?;
    let client_field_id = validate_client_field(
        schema,
        entrypoint_declaration.item.client_field_name,
        text_source,
        parent_object_entity_id,
    )?;

    Ok(client_field_id)
}

fn validate_parent_object_entity_id<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_type: WithSpan<UnvalidatedTypeName>,
    text_source: TextSource,
) -> Result<ServerObjectEntityId, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_type_id = schema
        .server_entity_data
        .defined_entities
        .get(&parent_type.item)
        .ok_or(WithLocation::new(
            ValidateEntrypointDeclarationError::ParentTypeNotDefined {
                parent_type_name: parent_type.item,
            },
            Location::new(text_source, parent_type.span),
        ))?;

    match parent_type_id {
        ServerEntityId::Object(object_entity_id) => {
            if !schema.fetchable_types.contains_key(object_entity_id) {
                Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::NonFetchableParentType {
                        parent_type_name: parent_type.item,
                        fetchable_types: schema
                            .fetchable_types
                            .keys()
                            .map(|object_entity_id| {
                                schema
                                    .server_entity_data
                                    .server_object_entity(*object_entity_id)
                                    .name
                                    .to_string()
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    },
                    Location::new(text_source, parent_type.span),
                ))
            } else {
                Ok(*object_entity_id)
            }
        }
        ServerEntityId::Scalar(scalar_entity_id) => {
            let scalar_name = schema
                .server_entity_data
                .server_scalar_entity(*scalar_entity_id)
                .name;
            Err(WithLocation::new(
                ValidateEntrypointDeclarationError::InvalidParentType {
                    parent_type: "scalar",
                    parent_type_name: scalar_name.item.into(),
                },
                Location::new(text_source, parent_type.span),
            ))
        }
    }
}

fn validate_client_field<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    field_name: WithSpan<ServerScalarSelectableName>,
    text_source: TextSource,
    parent_object_entity_id: ServerObjectEntityId,
) -> Result<ClientScalarSelectableId, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_object = schema
        .server_entity_data
        .server_object_entity(parent_object_entity_id);

    match schema.server_entity_data.server_object_entity_available_selectables
        .get(&parent_object_entity_id)
        .expect("Expected parent_object_entity_id to exist in server_object_entity_available_selectables")
        .get(&field_name.item.into())
    {
        Some(defined_field) => match defined_field {
            DefinitionLocation::Client(SelectionType::Object(_))
            | DefinitionLocation::Server(_) => Err(WithLocation::new(
                ValidateEntrypointDeclarationError::FieldMustBeClientField {
                    parent_type_name: parent_object.name,
                    client_field_name: field_name.item,
                },
                Location::new(text_source, field_name.span),
            )),
            DefinitionLocation::Client(SelectionType::Scalar(client_field_id)) => {
                Ok(*client_field_id)
            }
        },
        None => Err(WithLocation::new(
            ValidateEntrypointDeclarationError::ClientFieldMustExist {
                parent_type_name: parent_object.name,
                client_field_name: field_name.item,
            },
            Location::new(text_source, field_name.span),
        )),
    }
}

#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum ValidateEntrypointDeclarationError {
    #[error("`{parent_type_name}` is not a type that has been defined.")]
    ParentTypeNotDefined {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid parent type. `{parent_type_name}` is a {parent_type}, but it should be an object or interface.")]
    InvalidParentType {
        parent_type: &'static str,
        parent_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The type `{parent_type_name}` is not fetchable. The following types are fetchable: {fetchable_types}.",
    )]
    NonFetchableParentType {
        parent_type_name: UnvalidatedTypeName,
        fetchable_types: String,
    },

    #[error("The client field `{parent_type_name}.{client_field_name}` is not defined.")]
    ClientFieldMustExist {
        parent_type_name: IsographObjectTypeName,
        client_field_name: ServerScalarSelectableName,
    },

    // N.B. We could conceivably support fetching server fields, though!
    #[error("The field `{parent_type_name}.{client_field_name}` is a server field. It must be a client defined field.")]
    FieldMustBeClientField {
        parent_type_name: IsographObjectTypeName,
        client_field_name: ServerScalarSelectableName,
    },
}
