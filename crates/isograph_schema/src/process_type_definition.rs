use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    DefinedField, IsographObjectTypeName, ScalarFieldName, ServerFieldDefinitionName,
    UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::{
    OutputFieldDefinition, ScalarTypeDefinition, TypeAnnotation, TypeSystemDefinition,
    TypeSystemDocument,
};
use intern::string_key::Intern;
use isograph_lang_types::{ServerFieldId, TypeId, TypeWithFieldsId};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    IsographObjectTypeDefinition, Schema, SchemaObject, SchemaScalar, SchemaServerField,
    UnvalidatedSchema, UnvalidatedSchemaField, STRING_JAVASCRIPT_TYPE,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
}

impl UnvalidatedSchema {
    pub fn process_type_system_document(
        &mut self,
        type_system_document: TypeSystemDocument,
    ) -> ProcessTypeDefinitionResult<()> {
        // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
        // with e.g. interfaces "simply" being objects that can be refined to other
        // concrete objects.
        //
        // Processing type system documents is done in two passes:
        // - First, create types for interfaces, objects, scalars, etc.
        // - Then, validate that all implemented interfaces exist, and add refinements
        //   to the found interface.
        //
        let mut valid_type_refinement_map = HashMap::new();

        for type_system_definition in type_system_document.0 {
            match type_system_definition {
                TypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                    self.process_object_type_definition(
                        object_type_definition.into(),
                        &mut valid_type_refinement_map,
                    )?;
                }
                TypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                    self.process_scalar_definition(scalar_type_definition)?;
                }
                TypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                    self.process_object_type_definition(
                        interface_type_definition.into(),
                        &mut valid_type_refinement_map,
                    )?;
                }
            }
        }

        for (supertype_id, subtypes) in valid_type_refinement_map {
            // supertype, if it exists, can be refined to each subtype
            let supertype_id = self
                .schema_data
                .defined_types
                .get(&supertype_id.into())
                .ok_or(
                    ProcessTypeDefinitionError::IsographObjectTypeNameNotDefined {
                        type_name: supertype_id,
                    },
                )?;

            // TODO modify supertype
        }

        Ok(())
    }

    fn process_object_type_definition(
        &mut self,
        type_definition: IsographObjectTypeDefinition,
        valid_type_refinement_map: &mut HashMap<
            IsographObjectTypeName,
            Vec<WithSpan<IsographObjectTypeName>>,
        >,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            fields: ref mut existing_fields,
            ref mut schema_data,
            ..
        } = self;
        let next_object_id = schema_data.objects.len().into();
        let ref mut type_names = schema_data.defined_types;
        let ref mut objects = schema_data.objects;
        match type_names.entry(type_definition.name.item.into()) {
            Entry::Occupied(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "object",
                    type_name: type_definition.name.item.into(),
                });
            }
            Entry::Vacant(vacant) => {
                let (new_fields, server_field_ids, encountered_field_names) =
                    get_field_objects_ids_and_names(
                        type_definition.fields,
                        existing_fields.len(),
                        TypeWithFieldsId::Object(next_object_id),
                        type_definition.name.item.into(),
                    )?;
                objects.push(SchemaObject {
                    description: type_definition.description.map(|d| d.item),
                    name: type_definition.name.item,
                    id: next_object_id,
                    fields: server_field_ids,
                    // Resolvers are not defined until we process iso literals. They're not contained in
                    // the schema definition.
                    resolvers: vec![],
                    encountered_field_names,
                    valid_refinements: vec![],
                });

                // ----- HACK -----
                // This should mutate a default query object; only if no schema declaration is ultimately
                // encountered should we use the default query object.
                //
                // Also, this is a GraphQL concept, but it's leaking into Isograph land :/
                if type_definition.name.item == *QUERY_TYPE {
                    self.query_type_id = Some(next_object_id);
                }
                // --- END HACK ---

                existing_fields.extend(new_fields);
                vacant.insert(TypeId::Object(next_object_id));
            }
        }

        for interface in type_definition.interfaces {
            // type_definition implements interface
            let definitions = valid_type_refinement_map
                .entry(interface.item.into())
                .or_default();
            definitions.push(type_definition.name);
        }

        Ok(())
    }

    fn process_scalar_definition(
        &mut self,
        scalar_type_definition: ScalarTypeDefinition,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            ref mut schema_data,
            ..
        } = self;
        let next_scalar_id = schema_data.scalars.len().into();
        let ref mut type_names = schema_data.defined_types;
        let ref mut scalars = schema_data.scalars;
        match type_names.entry(scalar_type_definition.name.item.into()) {
            Entry::Occupied(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "object",
                    type_name: scalar_type_definition.name.item.into(),
                });
            }
            Entry::Vacant(vacant) => {
                scalars.push(SchemaScalar {
                    description: scalar_type_definition.description.map(|d| d.item),
                    name: scalar_type_definition.name.item,
                    id: next_scalar_id,
                    javascript_name: *STRING_JAVASCRIPT_TYPE,
                });

                vacant.insert(TypeId::Scalar(next_scalar_id));
            }
        }
        Ok(())
    }
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names(
    new_fields: Vec<WithSpan<OutputFieldDefinition>>,
    next_field_id: usize,
    parent_type: TypeWithFieldsId,
    parent_type_name: IsographObjectTypeName,
) -> ProcessTypeDefinitionResult<(
    Vec<UnvalidatedSchemaField>,
    Vec<ServerFieldId>,
    HashMap<
        ServerFieldDefinitionName,
        DefinedField<TypeAnnotation<UnvalidatedTypeName>, ScalarFieldName>,
    >,
)> {
    let new_field_count = new_fields.len();
    let mut field_names_to_type_name = HashMap::with_capacity(new_field_count);
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut field_ids = Vec::with_capacity(new_field_count);
    for (current_field_index, field) in new_fields.into_iter().enumerate() {
        // TODO use entry
        match field_names_to_type_name.insert(
            field.item.name.item,
            DefinedField::ServerField(field.item.type_.clone()),
        ) {
            None => {
                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name.item,
                    id: (next_field_id + current_field_index).into(),
                    field_type: field.item.type_,
                    parent_type_id: parent_type,
                });
                field_ids.push((next_field_id + current_field_index).into());
            }
            Some(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateField {
                    field_name: field.item.name.item,
                    parent_type: parent_type_name,
                });
            }
        }
    }
    Ok((unvalidated_fields, field_ids, field_names_to_type_name))
}

type ProcessTypeDefinitionResult<T> = Result<T, ProcessTypeDefinitionError>;

/// Errors tha make semantic sense when referring to creating a GraphQL schema in-memory representation
#[derive(Error, Debug)]
pub enum ProcessTypeDefinitionError {
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: ServerFieldDefinitionName,
        parent_type: IsographObjectTypeName,
    },

    // When type Foo implements Bar and Bar is not defined:
    #[error("Type \"{type_name}\" is never defined.")]
    IsographObjectTypeNameNotDefined { type_name: IsographObjectTypeName },
}
