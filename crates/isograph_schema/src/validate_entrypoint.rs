use common_lang_types::{
    IsographObjectTypeName, Location, ScalarFieldName, TextSource, UnvalidatedTypeName,
    WithLocation, WithSpan,
};
use isograph_lang_types::{ClientFieldId, EntrypointTypeAndField, ObjectId, SelectableFieldId};
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
        let resolver_field_id = self.validate_resolver_field(
            entrypoint_type_and_field.item.client_field_name,
            text_source,
            parent_object_id,
        )?;

        Ok(resolver_field_id)
    }

    fn validate_parent_object_id(
        &self,
        parent_type: WithSpan<UnvalidatedTypeName>,
        text_source: TextSource,
    ) -> Result<ObjectId, WithLocation<ValidateEntrypointDeclarationError>> {
        let parent_type_id = self
            .schema_data
            .defined_types
            .get(&parent_type.item.into())
            .ok_or(WithLocation::new(
                ValidateEntrypointDeclarationError::ParentTypeNotDefined {
                    parent_type_name: parent_type.item,
                },
                Location::new(text_source, parent_type.span),
            ))?;

        match parent_type_id {
            SelectableFieldId::Object(object_id) => {
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
            SelectableFieldId::Scalar(scalar_id) => {
                let scalar_name = self.schema_data.scalars[scalar_id.as_usize()].name;
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

    fn validate_resolver_field(
        &self,
        field_name: WithSpan<ScalarFieldName>,
        text_source: TextSource,
        parent_object_id: ObjectId,
    ) -> Result<ClientFieldId, WithLocation<ValidateEntrypointDeclarationError>> {
        let parent_object = self.schema_data.object(parent_object_id);

        match parent_object
            .encountered_fields
            .get(&field_name.item.into())
        {
            Some(defined_field) => match defined_field {
                FieldDefinitionLocation::Server(_) => Err(WithLocation::new(
                    ValidateEntrypointDeclarationError::FieldMustBeResolverField {
                        parent_type_name: parent_object.name,
                        resolver_field_name: field_name.item,
                    },
                    Location::new(text_source, field_name.span),
                )),
                FieldDefinitionLocation::Client(resolver_field_id) => Ok(*resolver_field_id),
            },
            None => Err(WithLocation::new(
                ValidateEntrypointDeclarationError::ResolverFieldMustExist {
                    parent_type_name: parent_object.name,
                    resolver_field_name: field_name.item,
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

    #[error("The resolver `{parent_type_name}.{resolver_field_name}` is not defined.")]
    ResolverFieldMustExist {
        parent_type_name: IsographObjectTypeName,
        resolver_field_name: ScalarFieldName,
    },

    // N.B. We could conceivably support fetching server fields, though!
    #[error("The field `{parent_type_name}.{resolver_field_name}` is a server field. It must be a client defined field.")]
    FieldMustBeResolverField {
        parent_type_name: IsographObjectTypeName,
        resolver_field_name: ScalarFieldName,
    },
}
