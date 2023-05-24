use boulton_lang_types::ResolverDeclaration;
use common_lang_types::{
    DefinedField, FieldDefinitionName, ObjectId, TypeId, TypeWithFieldsId, TypeWithFieldsName,
    UnvalidatedTypeName, WithSpan,
};
use thiserror::Error;

use crate::{SchemaField, SchemaResolverDefinitionInfo, UnvalidatedSchema};

impl UnvalidatedSchema {
    /// We don't currently support creating new types in resolvers,
    /// so we can assume parent_type exists
    pub fn process_resolver_declaration(
        &mut self,
        resolver_declaration: WithSpan<ResolverDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let parent_type_id = self
            .schema_data
            .defined_types
            .get(&resolver_declaration.item.parent_type.item.into())
            .ok_or(ProcessResolverDeclarationError::MissingParent {
                parent_type_name: resolver_declaration.item.parent_type.item,
            })?;

        match parent_type_id {
            TypeId::Object(object_id) => {
                self.add_resolver_field_to_object(*object_id, resolver_declaration)?;
            }
            TypeId::Scalar(scalar_id) => {
                let scalar_name = self.schema_data.scalars[scalar_id.as_usize()].name;
                return Err(ProcessResolverDeclarationError::InvalidParentType {
                    parent_type: "scalar",
                    parent_type_name: scalar_name.into(),
                });
            }
        }

        Ok(())
    }

    fn add_resolver_field_to_object(
        &mut self,
        object: ObjectId,
        resolver_declaration: WithSpan<ResolverDeclaration>,
    ) -> ProcessResolverDeclarationResult<()> {
        let object = &mut self.schema_data.objects[object.as_usize()];
        let resolver_field_name = resolver_declaration.item.resolver_field_name.item;

        if object
            .encountered_field_names
            .insert(
                resolver_field_name.into(),
                DefinedField::ResolverField(resolver_field_name),
            )
            .is_some()
        {
            // Did not insert, so this object already has a field with the same name :(
            return Err(ProcessResolverDeclarationError::ParentAlreadyHasField {
                parent_type: "object",
                parent_type_name: object.name.into(),
                resolver_field_name: resolver_field_name.into(),
            });
        }

        let field_id = self.fields.len();
        object.fields.push(field_id.into());

        self.fields.push(SchemaField {
            description: resolver_declaration.item.description.map(|d| d.item),
            name: resolver_declaration.item.resolver_field_name.item.into(),
            id: field_id.into(),
            field_type: DefinedField::ResolverField(SchemaResolverDefinitionInfo {
                resolver_definition_path: resolver_declaration.item.resolver_definition_path,
                selection_set_and_unwraps: resolver_declaration.item.selection_set_and_unwraps,
            }),
            parent_type_id: TypeWithFieldsId::Object(object.id),
        });
        Ok(())
    }
}

type ProcessResolverDeclarationResult<T> = Result<T, ProcessResolverDeclarationError>;

#[derive(Error, Debug)]
pub enum ProcessResolverDeclarationError {
    #[error("Missing parent type. Type: `{parent_type_name}`")]
    MissingParent {
        parent_type_name: UnvalidatedTypeName,
    },

    #[error("Invalid parent type. `{parent_type_name}` is a {parent_type}, but it should be an object or interface.")]
    InvalidParentType {
        parent_type: &'static str,
        parent_type_name: UnvalidatedTypeName,
    },

    #[error(
        "The {parent_type} {parent_type_name} already has a field named `{resolver_field_name}`."
    )]
    ParentAlreadyHasField {
        parent_type: &'static str,
        parent_type_name: TypeWithFieldsName,
        resolver_field_name: FieldDefinitionName,
    },
}
