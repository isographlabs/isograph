use std::collections::{hash_map::Entry, HashMap};

use common_lang_types::{
    IsographObjectTypeName, ScalarTypeName, SelectableFieldName, Span, UnvalidatedTypeName,
    WithSpan,
};
use graphql_lang_types::{
    InputValueDefinition, NamedTypeAnnotation, NonNullTypeAnnotation, OutputFieldDefinition,
    ScalarTypeDefinition, TypeAnnotation, TypeSystemDefinition, TypeSystemDocument,
};
use intern::{string_key::Intern, Lookup};
use isograph_lang_types::{
    DefinedTypeId, ObjectId, ResolverFieldId, ScalarFieldSelection, Selection, ServerFieldId,
    ServerFieldSelection, ServerIdFieldId,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    DefinedField, IsographObjectTypeDefinition, ResolverActionKind, ResolverArtifactKind,
    ResolverTypeAndField, ResolverVariant, Schema, SchemaObject, SchemaResolver, SchemaScalar,
    SchemaServerField, UnvalidatedObjectFieldInfo, UnvalidatedSchema, UnvalidatedSchemaField,
    UnvalidatedSchemaResolver, ValidRefinement, ID_GRAPHQL_TYPE, STRING_JAVASCRIPT_TYPE,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
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

        let mut mutation_type_id = None;
        for type_system_definition in type_system_document.0 {
            match type_system_definition {
                TypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                    let mutation_id = self.process_object_type_definition(
                        object_type_definition.into(),
                        &mut valid_type_refinement_map,
                    )?;
                    if let Some(mutation_id) = mutation_id {
                        mutation_type_id = Some(mutation_id);
                    }
                }
                TypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                    self.process_scalar_definition(scalar_type_definition)?;
                    // N.B. we assume that Mutation will be an object, not a scalar
                }
                TypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                    self.process_object_type_definition(
                        interface_type_definition.into(),
                        &mut valid_type_refinement_map,
                    )?;
                    // N.B. we assume that Mutation will be an object, not an interface
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

        if let Some(mutation_id) = mutation_type_id {
            self.add_mutation_fields(mutation_id)?;
        }

        Ok(())
    }

    fn process_object_type_definition(
        &mut self,
        type_definition: IsographObjectTypeDefinition,
        valid_type_refinement_map: &mut HashMap<IsographObjectTypeName, Vec<ObjectId>>,
    ) -> ProcessTypeDefinitionResult<Option<ObjectId>> {
        let &mut Schema {
            fields: ref mut schema_fields,
            ref mut schema_data,
            resolvers: ref mut schema_resolvers,
            ..
        } = self;
        let next_object_id = schema_data.objects.len().into();
        let string_type_for_typename = schema_data.scalar(self.string_type_id).name;
        let ref mut type_names = schema_data.defined_types;
        let ref mut objects = schema_data.objects;
        let mut mutation_id = None;
        match type_names.entry(type_definition.name.item.into()) {
            Entry::Occupied(_) => {
                return Err(ProcessTypeDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "object",
                    type_name: type_definition.name.item.into(),
                });
            }
            Entry::Vacant(vacant) => {
                // TODO avoid this
                let type_def_2 = type_definition.clone();
                let FieldObjectIdsEtc {
                    unvalidated_schema_fields,
                    server_fields,
                    mut encountered_fields,
                    id_field,
                } = get_field_objects_ids_and_names(
                    type_def_2.fields,
                    schema_fields.len(),
                    next_object_id,
                    type_def_2.name.item.into(),
                    get_typename_type(string_type_for_typename.item),
                )?;

                let object_resolvers = get_resolvers_for_schema_object(
                    &id_field,
                    &mut encountered_fields,
                    schema_resolvers,
                    next_object_id,
                    &type_definition,
                );

                objects.push(SchemaObject {
                    description: type_definition.description.map(|d| d.item),
                    name: type_definition.name.item,
                    id: next_object_id,
                    server_fields,
                    resolvers: object_resolvers,
                    encountered_fields,
                    valid_refinements: vec![],
                    id_field,
                });

                // ----- HACK -----
                // This should mutate a default query object; only if no schema declaration is ultimately
                // encountered should we use the default query object.
                //
                // Also, this is a GraphQL concept, but it's leaking into Isograph land :/ (is it?)
                if type_definition.name.item == *QUERY_TYPE {
                    self.query_type_id = Some(next_object_id);
                }
                // --- END HACK ---

                // ----- HACK -----
                // It's unclear to me that this is the best way to add magic mutation fields.
                if type_definition.name.item == *MUTATION_TYPE {
                    mutation_id = Some(next_object_id)
                }
                // --- END HACK ---

                schema_fields.extend(unvalidated_schema_fields);
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

        Ok(mutation_id)
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
                    description: scalar_type_definition.description,
                    name: scalar_type_definition.name,
                    id: next_scalar_id,
                    javascript_name: *STRING_JAVASCRIPT_TYPE,
                });

                vacant.insert(DefinedTypeId::Scalar(next_scalar_id));
            }
        }
        Ok(())
    }

    /// Add magical mutation fields.
    ///
    /// > This is a bit hacky! It should be controlled more by directives. Instead, lots of behavior
    /// > is hard-coded now.
    ///
    /// For each field on a mutation object, if:
    /// - that field's type is a non-nullable object and **exactly one** of that object's fields:
    ///   - either
    ///     - has directive "@primary", or
    ///     - is the only field
    ///   - has no arguments, and
    ///   - is an object type (call it TargetType)
    /// - and that field's arguments contain an argument named "id" of type "ID!"
    ///
    /// then, add a magical field to TargetType whose name is __ + mutation_name, which:
    /// - executes the mutation
    /// - has the mutation's arguments (except an id)
    /// - then acts as a __refetch field on that TargetType, i.e. refetches all the fields
    ///   selected in the merged selection set.
    ///
    /// There is lots of cloning going on here! Not ideal.
    fn add_mutation_fields(&mut self, mutation_id: ObjectId) -> ProcessTypeDefinitionResult<()> {
        let mutation_object_fields = self.schema_data.object(mutation_id).server_fields.clone();

        for field_id in mutation_object_fields.iter() {
            let mutation_field = self.field(*field_id);
            let magic_mutation_field_name = format!("__{}", mutation_field.name).intern().into();

            if let Some((
                parent_object_id,
                mutation_response_primary_field_name,
                mutation_field_args,
            )) = self.get_valid_mutation_field_target_type(
                mutation_field.associated_data.clone(),
                &mutation_field.arguments,
            ) {
                // Woohoo! We found a valid object type onto which we can add a magic mutation field.
                // TODO continue from here
                let description = mutation_field.description.clone();
                let parent_object = self.schema_data.object_mut(parent_object_id);

                let next_resolver_id = self.resolvers.len().into();

                let id_field_selection = WithSpan::new(
                    Selection::ServerField(ServerFieldSelection::ScalarField(
                        ScalarFieldSelection {
                            name: WithSpan::new("id".intern().into(), Span::todo_generated()),
                            reader_alias: None,
                            normalization_alias: None,
                            associated_data: (),
                            unwraps: vec![],
                            arguments: vec![],
                        },
                    )),
                    Span::todo_generated(),
                );

                self.resolvers.push(SchemaResolver {
                    description,
                    name: magic_mutation_field_name,
                    id: next_resolver_id,
                    selection_set_and_unwraps: Some((vec![id_field_selection], vec![])),
                    variant: Some(WithSpan::new(
                        ResolverVariant::MutationField((
                            magic_mutation_field_name,
                            mutation_response_primary_field_name,
                            mutation_field_args,
                        )),
                        Span::todo_generated(),
                    )),
                    variable_definitions: vec![],
                    type_and_field: ResolverTypeAndField {
                        type_name: parent_object.name,
                        field_name: magic_mutation_field_name,
                    },
                    parent_object_id,
                    artifact_kind: ResolverArtifactKind::NonFetchable,
                    action_kind: ResolverActionKind::MutationField,
                });

                if parent_object
                    .encountered_fields
                    .insert(
                        magic_mutation_field_name,
                        DefinedField::ResolverField(next_resolver_id),
                    )
                    .is_some()
                {
                    return Err(ProcessTypeDefinitionError::DuplicateField {
                        field_name: magic_mutation_field_name,
                        parent_type: parent_object.name,
                    });
                }

                parent_object.resolvers.push(next_resolver_id);
            }
        }

        Ok(())
    }

    fn get_valid_mutation_field_target_type(
        &self,
        // From the top level field, e.g. create_user
        type_: TypeAnnotation<UnvalidatedTypeName>,
        arguments: &[WithSpan<InputValueDefinition>],
    ) -> Option<(
        ObjectId,
        SelectableFieldName,
        Vec<WithSpan<InputValueDefinition>>,
    )> {
        // Is the mutation_field's type a non-nullable type?
        let mutation_response_inner_non_nullable_named_type = type_.inner_non_null_named_type()?;

        // Is the mutation_field's type an object type?
        if let DefinedTypeId::Object(mutation_response_object_id) = self
            .schema_data
            .defined_types
            .get(&mutation_response_inner_non_nullable_named_type.0.item)
            .expect("object type should exist. This indicates a bug in Isograph")
        {
            // Does the mutation field have an argument named "id"
            // (TODO validate it has type ID!)
            let arguments_without_id_arg = arguments_without_id_arg(arguments)?;

            // This is the mutation response object, in other words something like CreateUserResponse
            let mutation_response_object = self.schema_data.object(*mutation_response_object_id);

            // Does the mutation response object have exactly one field?
            // TODO handle the @primary directive
            let mutation_response_only_field = self.field(get_only_non_typename_field(
                self,
                &mutation_response_object.server_fields,
            )?);

            // Does the mutation response object's only field have no arguments?
            if mutation_response_only_field.arguments.is_empty() {
                let primary_field_type = mutation_response_only_field
                    .associated_data
                    .inner_non_null_named_type()?;
                if let DefinedTypeId::Object(primary_field_object_id) = self
                    .schema_data
                    .defined_types
                    .get(&primary_field_type.0.item)
                    .expect("object type should exist. This indicates a bug in Isograph.")
                {
                    return Some((
                        *primary_field_object_id,
                        mutation_response_only_field.name,
                        arguments_without_id_arg,
                    ));
                }
            }
        }

        None
    }
}

fn arguments_without_id_arg(
    arguments: &[WithSpan<InputValueDefinition>],
) -> Option<Vec<WithSpan<InputValueDefinition>>> {
    let mut found_id = false;
    let new_arguments = arguments
        .iter()
        .filter_map(|arg| {
            // TODO also confirm stuff like that the type is ID!
            if arg.item.name.item == "id".intern().into() {
                found_id = true;
                None
            } else {
                Some(arg.clone())
            }
        })
        .collect();

    if found_id {
        Some(new_arguments)
    } else {
        None
    }
}

fn get_only_non_typename_field(
    schema: &UnvalidatedSchema,
    server_fields: &[ServerFieldId],
) -> Option<ServerFieldId> {
    let mut found_field = None;
    let typename_field_name = "__typename".intern().into();

    for field_id in server_fields {
        let field = schema.field(*field_id);
        // TODO this is a hacky check
        if field.name != typename_field_name {
            if found_field.is_some() {
                return None;
            } else {
                found_field = Some(*field_id);
            }
        }
    }
    found_field
}

/// Returns the resolvers for a schema object that we know up-front (before processing
/// iso literals.) This is either a refetch field (if the object is refetchable), or
/// nothing.
fn get_resolvers_for_schema_object(
    id_field_id: &Option<ServerIdFieldId>,
    encountered_fields: &mut HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
    schema_resolvers: &mut Vec<UnvalidatedSchemaResolver>,
    parent_object_id: ObjectId,
    type_definition: &IsographObjectTypeDefinition,
) -> Vec<ResolverFieldId> {
    if let Some(_id_field_id) = id_field_id {
        let next_resolver_id = schema_resolvers.len().into();
        let id_field_selection = WithSpan::new(
            Selection::ServerField(ServerFieldSelection::ScalarField(ScalarFieldSelection {
                name: WithSpan::new("id".intern().into(), Span::todo_generated()),
                reader_alias: None,
                normalization_alias: None,
                associated_data: (),
                unwraps: vec![],
                arguments: vec![],
            })),
            Span::todo_generated(),
        );
        schema_resolvers.push(SchemaResolver {
            description: Some("A refetch field for this object.".intern().into()),
            name: "__refetch".intern().into(),
            id: next_resolver_id,
            selection_set_and_unwraps: Some((vec![id_field_selection], vec![])),
            variant: Some(WithSpan::new(
                ResolverVariant::RefetchField,
                Span::todo_generated(),
            )),
            variable_definitions: vec![],
            type_and_field: ResolverTypeAndField {
                type_name: type_definition.name.item,
                field_name: "__refetch".intern().into(),
            },
            parent_object_id,
            // N.B. __refetch fields are non-fetchable, but they do execute queries which
            // have fetchable artifacts (i.e. normalization ASTs).
            artifact_kind: ResolverArtifactKind::NonFetchable,
            action_kind: ResolverActionKind::RefetchField,
        });
        encountered_fields.insert(
            "__refetch".intern().into(),
            DefinedField::ResolverField(next_resolver_id),
        );
        vec![next_resolver_id]
    } else {
        vec![]
    }
}

fn get_typename_type(
    string_type_for_typename: ScalarTypeName,
) -> TypeAnnotation<UnvalidatedTypeName> {
    TypeAnnotation::NonNull(Box::new(NonNullTypeAnnotation::Named(NamedTypeAnnotation(
        WithSpan::new(
            string_type_for_typename.into(),
            // TODO we probably need a generated or built-in span type
            Span::todo_generated(),
        ),
    ))))
}

struct FieldObjectIdsEtc {
    unvalidated_schema_fields: Vec<UnvalidatedSchemaField>,
    server_fields: Vec<ServerFieldId>,
    encountered_fields: HashMap<SelectableFieldName, UnvalidatedObjectFieldInfo>,
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
                    set_and_validate_id_field(
                        &mut id_field,
                        current_field_id,
                        &field,
                        parent_type_name,
                    )?;
                }

                unvalidated_fields.push(SchemaServerField {
                    description: field.item.description.map(|d| d.item),
                    name: field.item.name.item,
                    id: current_field_id.into(),
                    associated_data: field.item.type_,
                    parent_type_id,
                    arguments: field.item.arguments,
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
    // TODO: the only way to determine that a field is a magic __typename field is
    // to check the name! That's a bit unfortunate. We should model these differently,
    // perhaps fields should contain an enum (IdField, TypenameField, ActualField)
    let typename_field_id = (next_field_id + field_ids.len()).into();
    let typename_name = "__typename".intern().into();
    field_ids.push(typename_field_id);
    unvalidated_fields.push(SchemaServerField {
        description: None,
        name: typename_name,
        id: typename_field_id,
        associated_data: typename_type.clone(),
        parent_type_id,
        arguments: vec![],
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

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerIdFieldId>,
    current_field_id: usize,
    field: &WithSpan<OutputFieldDefinition>,
    parent_type_name: IsographObjectTypeName,
) -> ProcessTypeDefinitionResult<()> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_id.into());

    match field.item.type_.inner_non_null_named_type() {
        Some(type_) => {
            if (*type_).0.item.lookup() != ID_GRAPHQL_TYPE.lookup() {
                Err(ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
                    parent_type: parent_type_name,
                })
            } else {
                Ok(())
            }
        }
        None => Err(ProcessTypeDefinitionError::IdFieldMustBeNonNullIdType {
            parent_type: parent_type_name,
        }),
    }
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
