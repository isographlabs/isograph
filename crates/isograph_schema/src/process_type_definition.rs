use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    DefinedField, IsographObjectTypeName, ScalarFieldName, SelectableFieldName, Span,
    UnvalidatedTypeName, WithSpan,
};
use graphql_lang_types::{
    NamedTypeAnnotation, NonNullTypeAnnotation, OutputFieldDefinition, ScalarTypeDefinition,
    TypeAnnotation, TypeSystemDefinition, TypeSystemDocument,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{DefinedTypeId, ObjectId, ServerFieldId, ServerIdFieldId};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    IsographObjectTypeDefinition, Schema, SchemaObject, SchemaScalar, SchemaServerField,
    UnvalidatedSchema, UnvalidatedSchemaField, ValidRefinement, ID_GRAPHQL_TYPE,
    STRING_JAVASCRIPT_TYPE,
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

        for (supertype_name, subtypes) in valid_type_refinement_map {
            // supertype, if it exists, can be refined to each subtype
            let supertype_id = self
                .schema_data
                .defined_types
                .get(&supertype_name.into())
                .ok_or(
                    ProcessTypeDefinitionError::IsographObjectTypeNameNotDefined {
                        type_name: supertype_name,
                    },
                )?;

            match supertype_id {
                DefinedTypeId::Scalar(_) => {
                    return Err(ProcessTypeDefinitionError::IsographObjectTypeNameIsScalar {
                        type_name: supertype_name,
                    })
                }
                DefinedTypeId::Object(object_id) => {
                    let supertype = self.schema_data.object_mut(*object_id);
                    // TODO validate that supertype was defined as an interface, perhaps by
                    // including references to the original definition (i.e. as a type parameter)
                    // and having the schema be able to validate this. (i.e. this should be
                    // a way to execute GraphQL-specific code in isograph-land without actually
                    // putting the code here.)

                    for subtype_id in subtypes {
                        supertype
                            .valid_refinements
                            .push(ValidRefinement { target: subtype_id });
                    }
                }
            }
        }

        Ok(())
    }

    fn process_object_type_definition(
        &mut self,
        type_definition: IsographObjectTypeDefinition,
        valid_type_refinement_map: &mut HashMap<IsographObjectTypeName, Vec<ObjectId>>,
    ) -> ProcessTypeDefinitionResult<()> {
        let &mut Schema {
            fields: ref mut existing_fields,
            ref mut schema_data,
            ..
        } = self;
        let next_object_id = schema_data.objects.len().into();
        let string_type_for_typename = schema_data.scalar(self.string_type_id).name;
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
                let FieldObjectIdsEtc {
                    unvalidated_schema_fields,
                    server_fields,
                    encountered_fields,
                    id_field,
                } = get_field_objects_ids_and_names(
                    type_definition.fields,
                    existing_fields.len(),
                    next_object_id,
                    type_definition.name.item.into(),
                    TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(
                        NamedTypeAnnotation(WithSpan::new(
                            string_type_for_typename.into(),
                            // TODO we probably need a generated or built-in span type
                            Span::new(0, 0),
                        )),
                    ))),
                )?;
                objects.push(SchemaObject {
                    description: type_definition.description.map(|d| d.item),
                    name: type_definition.name.item,
                    id: next_object_id,
                    server_fields,
                    // Resolvers are not defined until we process iso literals. They're not contained in
                    // the schema definition.
                    resolvers: vec![],
                    encountered_fields,
                    valid_refinements: vec![],
                    id_field,
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

                existing_fields.extend(unvalidated_schema_fields);
                vacant.insert(DefinedTypeId::Object(next_object_id));
            }
        }

        for interface in type_definition.interfaces {
            // type_definition implements interface
            let definitions = valid_type_refinement_map
                .entry(interface.item.into())
                .or_default();
            definitions.push(next_object_id);
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

                vacant.insert(DefinedTypeId::Scalar(next_scalar_id));
            }
        }
        Ok(())
    }
}

struct FieldObjectIdsEtc {
    unvalidated_schema_fields: Vec<UnvalidatedSchemaField>,
    server_fields: Vec<ServerFieldId>,
    encountered_fields: HashMap<
        SelectableFieldName,
        DefinedField<TypeAnnotation<UnvalidatedTypeName>, ScalarFieldName>,
    >,
    // TODO this should not be a ServerFieldId, but a special type
    id_field: Option<ServerIdFieldId>,
}

/// Given a vector of fields from the schema AST all belonging to the same object/interface,
/// return a vector of unvalidated fields and a set of field names.
fn get_field_objects_ids_and_names(
    new_fields: Vec<WithSpan<OutputFieldDefinition>>,
    next_field_id: usize,
    parent_type_id: ObjectId,
    parent_type_name: IsographObjectTypeName,
    typename_type: TypeAnnotation<UnvalidatedTypeName>,
) -> ProcessTypeDefinitionResult<FieldObjectIdsEtc> {
    let new_field_count = new_fields.len();
    let mut encountered_fields = HashMap::with_capacity(new_field_count);
    let mut unvalidated_fields = Vec::with_capacity(new_field_count);
    let mut field_ids = Vec::with_capacity(new_field_count + 1); // +1 for the typename
    let mut id_field = None;
    let id_name = "id".intern().into();
    for (current_field_index, field) in new_fields.into_iter().enumerate() {
        // TODO use entry
        match encountered_fields.insert(
            field.item.name.item,
            DefinedField::ServerField(field.item.type_.clone()),
        ) {
            None => {
                let current_field_id = next_field_id + current_field_index;

                // TODO check for @strong directive instead!
                if field.item.name.item == id_name {
                    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
                    // have contained this field name already.
                    debug_assert!(id_field.is_none(), "id field should not be defined twice");
                    id_field = Some(current_field_id.into());

                    if field.item.type_.inner_non_null_named_type().is_none() {
                        return Err(ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                            parent_type: parent_type_name,
                        });
                    }
                    if ID_GRAPHQL_TYPE.lookup() != field.item.type_.inner().lookup() {
                        return Err(ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                            parent_type: parent_type_name,
                        });
                    }
                    // We should change the type here! It should not be ID! It should be a
                    // type specific to the concrete type, e.g. UserID.
                }

                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name.item,
                    id: current_field_id.into(),
                    field_type: field.item.type_,
                    parent_type_id,
                });
                field_ids.push(current_field_id.into());
            }
            Some(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateField {
                    field_name: field.item.name.item,
                    parent_type: parent_type_name,
                });
            }
        }
    }

    // ------- HACK -------
    // Magic __typename field
    // TODO: find a way to do this that is less tied to GraphQL
    let typename_field_id = (next_field_id + field_ids.len()).into();
    let typename_name = "__typename".intern().into();
    field_ids.push(typename_field_id);
    unvalidated_fields.push(SchemaServerField {
        description: None,
        name: typename_name,
        id: typename_field_id,
        field_type: typename_type.clone(),
        parent_type_id,
    });
    if encountered_fields
        .insert(typename_name, DefinedField::ServerField(typename_type))
        .is_some()
    {
        return Err(ProcessTypeDefinitionError::TypenameCannotBeDefined {
            parent_type: parent_type_name,
        });
    }
    // ----- END HACK -----

    Ok(FieldObjectIdsEtc {
        unvalidated_schema_fields: unvalidated_fields,
        server_fields: field_ids,
        encountered_fields,
        id_field,
    })
}

type ProcessTypeDefinitionResult<T> = Result<T, ProcessTypeDefinitionError>;

/// Errors that make semantic sense when referring to creating a GraphQL schema in-memory representation
#[derive(Error, Debug)]
pub enum ProcessTypeDefinitionError {
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableFieldName,
        parent_type: IsographObjectTypeName,
    },

    // When type Foo implements Bar and Bar is not defined:
    #[error("Type \"{type_name}\" is never defined.")]
    IsographObjectTypeNameNotDefined { type_name: IsographObjectTypeName },

    // When type Foo implements Bar and Bar is scalar
    #[error("Type \"{type_name}\" is a scalar, but it should be an object type.")]
    IsographObjectTypeNameIsScalar { type_name: IsographObjectTypeName },

    #[error(
        "You cannot manually defined the \"__typename\" field, which is defined in \"{parent_type}\"."
    )]
    TypenameCannotBeDefined { parent_type: IsographObjectTypeName },

    #[error("The id field on \"{parent_type}\" must be \"ID!\".")]
    IdFieldMustBeNonNullIdType { parent_type: IsographObjectTypeName },
}
