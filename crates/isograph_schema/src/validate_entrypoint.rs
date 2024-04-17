use common_lang_types::{
    IsographObjectTypeName, Location, ScalarFieldName, TextSource, UnvalidatedTypeName,
    WithLocation, WithSpan,
};
use isograph_lang_types::{
    ClientFieldId, EntrypointTypeAndField, SelectableServerFieldId, ServerObjectId,
};
use thiserror::Error;

use crate::{FieldDefinitionLocation, UnvalidatedSchema};

impl UnvalidatedSchema {
    pub fn validate_entrypoint_type_and_field(
        &self,
        text_source: TextSource,
        entrypoint_type_and_field: WithSpan<EntrypointTypeAndField>,
    ) -> Result<ClientFieldId, WithLocation<ValidateEntrypointDeclarationError>> {
        let parent_object_id = self
            .validate_parent_object_id(entrypoint_type_and_field.item.parent_type, text_source)?;
        let client_field_id = self.validate_client_field(
            entrypoint_type_and_field.item.client_field_name,
            text_source,
            parent_object_id,
        )?;

        Ok(client_field_id)
    }

    fn validate_parent_object_id(
        &self,
        parent_type: WithSpan<UnvalidatedTypeName>,
        text_source: TextSource,
    ) -> Result<ServerObjectId, WithLocation<ValidateEntrypointDeclarationError>> {
        let parent_type_id = self
            .server_field_data
            .defined_types
            .get(&parent_type.item.into())
            .ok_or(WithLocation::new(
                ValidateEntrypointDeclarationError::ParentTypeNotDefined {
                    parent_type_name: parent_type.item,
                },
                Location::new(text_source, parent_type.span),
            ))?;

        match parent_type_id {
            SelectableServerFieldId::Object(object_id) => {
                // For now, only the query object is fetchable, and thus
                // can be used as a parent type in an iso entrypoint declaration.
                //
                // This requirement should be loosened — anything that we
                // know how to fetch (e.g. viewer, an item implementing Node, etc.)
                // should be fetchable.
                let query_id = self.query_type_id.ok_or(WithLocation::new(
                    ValidateEntrypointDeclarationError::RootQueryTypeMustExist,
                    Location::generated(),
                ))?;

                if query_id != *object_id {
                    Err(WithLocation::new(
                        ValidateEntrypointDeclarationError::NonFetchableParentType {
                            parent_type_name: parent_type.item,
                        },
                        Location::new(text_source, parent_type.span),
                    ))
                } else {
                    Ok(*object_id)
                }
            }
            SelectableServerFieldId::Scalar(scalar_id) => {
                let scalar_name = self.server_field_data.server_scalars[scalar_id.as_usize()].name;
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

    fn validate_client_field(
        &self,
        field_name: WithSpan<ScalarFieldName>,
        text_source: TextSource,
        parent_object_id: ServerObjectId,
    ) -> Result<ClientFieldId, WithLocation<ValidateEntrypointDeclarationError>> {
        let parent_object = self.server_field_data.object(parent_object_id);

        match parent_object
            .encountered_fields
            .get(&field_name.item.into())
        {
            Some(defined_field) => match defined_field {
                FieldDefinitionLocation::Server(_) => Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::FieldMustBeClientField {
                        parent_type_name: parent_object.name,
                        client_field_name: field_name.item,
                    },
                    Location::new(text_source, field_name.span),
                )),
                FieldDefinitionLocation::Client(client_field_id) => Ok(*client_field_id),
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
}

#[derive(Error, Debug)]
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

    #[error("A root query type must exist.")]
    RootQueryTypeMustExist,

    #[error(
        "The type `{parent_type_name}` is not fetchable. (Currently, only Query is fetchable.)"
    )]
    NonFetchableParentType {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("The client field `{parent_type_name}.{client_field_name}` is not defined.")]
    ClientFieldMustExist {
        parent_type_name: IsographObjectTypeName,
        client_field_name: ScalarFieldName,
    },

    // N.B. We could conceivably support fetching server fields, though!
    #[error("The field `{parent_type_name}.{client_field_name}` is a server field. It must be a client defined field.")]
    FieldMustBeClientField {
        parent_type_name: IsographObjectTypeName,
        client_field_name: ScalarFieldName,
    },
}
