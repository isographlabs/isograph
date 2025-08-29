use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    ClientScalarSelectableName, IsoLiteralText, Location, ServerObjectEntityName,
    ServerScalarSelectableName, TextSource, UnvalidatedTypeName, WithLocation, WithSpan,
};
use isograph_lang_types::{
    DefinitionLocation, EntrypointDeclaration, EntrypointDirectiveSet, SelectionType,
    ServerObjectEntityNameWrapper,
};

use thiserror::Error;

use crate::{NetworkProtocol, Schema, ServerEntityName};

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct EntrypointDeclarationInfo {
    pub iso_literal_text: IsoLiteralText,
    pub directive_set: EntrypointDirectiveSet,
}

#[allow(clippy::type_complexity)]
pub fn validate_entrypoints<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    entrypoint_declarations: Vec<(TextSource, WithSpan<EntrypointDeclaration>)>,
) -> Result<
    HashMap<(ServerObjectEntityName, ClientScalarSelectableName), EntrypointDeclarationInfo>,
    Vec<WithLocation<ValidateEntrypointDeclarationError>>,
> {
    let mut errors = vec![];
    let mut entrypoints: HashMap<
        (ServerObjectEntityName, ClientScalarSelectableName),
        EntrypointDeclarationInfo,
    > = HashMap::new();
    for (text_source, entrypoint_declaration) in entrypoint_declarations {
        match validate_entrypoint_type_and_field(schema, text_source, &entrypoint_declaration) {
            Ok(client_field_id) => {
                let new_entrypoint = EntrypointDeclarationInfo {
                    iso_literal_text: entrypoint_declaration.item.iso_literal_text,
                    directive_set: entrypoint_declaration.item.entrypoint_directive_set,
                };
                match entrypoints.entry((
                    entrypoint_declaration
                        .item
                        .parent_type
                        .item
                        .0
                        .unchecked_conversion(),
                    client_field_id,
                )) {
                    Entry::Occupied(occupied_entry) => {
                        if occupied_entry.get().directive_set != new_entrypoint.directive_set {
                            errors.push(WithLocation::new(
                                ValidateEntrypointDeclarationError::LazyLoadInconsistentEntrypoint,
                                Location::new(
                                    text_source,
                                    entrypoint_declaration.item.entrypoint_keyword.span,
                                ),
                            ));
                        }
                    }
                    Entry::Vacant(vacant) => {
                        vacant.insert(new_entrypoint);
                    }
                }
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
    entrypoint_declaration: &WithSpan<EntrypointDeclaration>,
) -> Result<ClientScalarSelectableName, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_object_entity_name = validate_parent_object_entity_name(
        schema,
        entrypoint_declaration.item.parent_type,
        text_source,
    )?;
    let client_field_id = validate_client_field(
        schema,
        entrypoint_declaration.item.client_field_name,
        text_source,
        parent_object_entity_name,
    )?;

    Ok(client_field_id)
}

fn validate_parent_object_entity_name<TNetworkProtocol: NetworkProtocol>(
    schema: &Schema<TNetworkProtocol>,
    parent_type: WithSpan<ServerObjectEntityNameWrapper>,
    text_source: TextSource,
) -> Result<ServerObjectEntityName, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_type_id = schema
        .server_entity_data
        .defined_entities
        .get(&parent_type.item.0)
        .ok_or(WithLocation::new(
            ValidateEntrypointDeclarationError::ParentTypeNotDefined {
                parent_type_name: parent_type.item,
            },
            Location::new(text_source, parent_type.span),
        ))?;

    match parent_type_id {
        ServerEntityName::Object(object_entity_name) => {
            if !schema.fetchable_types.contains_key(object_entity_name) {
                Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::NonFetchableParentType {
                        parent_type_name: parent_type.item,
                        fetchable_types: schema
                            .fetchable_types
                            .keys()
                            .map(|object_entity_name| {
                                schema
                                    .server_entity_data
                                    .server_object_entity(*object_entity_name)
                                    .expect(
                                        "Expected entity to exist. \
                                            This is indicative of a bug in Isograph.",
                                    )
                                    .name
                                    .to_string()
                            })
                            .collect::<Vec<_>>()
                            .join(", "),
                    },
                    Location::new(text_source, parent_type.span),
                ))
            } else {
                Ok(*object_entity_name)
            }
        }
        ServerEntityName::Scalar(scalar_entity_name) => {
            let scalar_name = schema
                .server_entity_data
                .server_scalar_entity(*scalar_entity_name)
                .expect(
                    "Expected entity to exist. \
                    This is indicative of a bug in Isograph.",
                )
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
    parent_object_name: ServerObjectEntityName,
) -> Result<ClientScalarSelectableName, WithLocation<ValidateEntrypointDeclarationError>> {
    let parent_object = schema
        .server_entity_data
        .server_object_entity(parent_object_name)
        .expect(
            "Expected entity to exist. \
            This is indicative of a bug in Isograph.",
        );

    match schema
        .server_entity_data
        .server_object_entity_extra_info
        .get(&parent_object_name)
        .expect(
            "Expected parent_object_entity_name to exist \
            in server_object_entity_available_selectables",
        )
        .selectables
        .get(&field_name.item.into())
    {
        Some(defined_field) => match defined_field {
            DefinitionLocation::Client(SelectionType::Object(_))
            | DefinitionLocation::Server(_) => Err(WithLocation::new(
                ValidateEntrypointDeclarationError::FieldMustBeClientField {
                    parent_type_name: parent_object.name.item,
                    client_field_name: field_name.item,
                },
                Location::new(text_source, field_name.span),
            )),
            DefinitionLocation::Client(SelectionType::Scalar((
                _parent_object_entity_name,
                client_field_name,
            ))) => Ok(*client_field_name),
        },
        None => Err(WithLocation::new(
            ValidateEntrypointDeclarationError::ClientFieldMustExist {
                parent_type_name: parent_object.name.item,
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
        parent_type_name: ServerObjectEntityNameWrapper,
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
        parent_type_name: ServerObjectEntityNameWrapper,
        fetchable_types: String,
    },

    #[error("The client field `{parent_type_name}.{client_field_name}` is not defined.")]
    ClientFieldMustExist {
        parent_type_name: ServerObjectEntityName,
        client_field_name: ServerScalarSelectableName,
    },

    // N.B. We could conceivably support fetching server fields, though!
    #[error("The field `{parent_type_name}.{client_field_name}` is a server field. It must be a client defined field.")]
    FieldMustBeClientField {
        parent_type_name: ServerObjectEntityName,
        client_field_name: ServerScalarSelectableName,
    },

    #[error("Entrypoint declared lazy in one location and declared eager in another location. Entrypoint must be either lazy or non-lazy in all instances.")]
    LazyLoadInconsistentEntrypoint,
}
