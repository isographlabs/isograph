use std::collections::{btree_map::Entry, hash_map::Entry as HashMapEntry, BTreeMap, HashMap};

use common_lang_types::{
    GraphQLObjectTypeName, IsographObjectTypeName, Location, SelectableName,
    ServerScalarSelectableName, Span, UnvalidatedTypeName, VariableName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInputValueDefinition, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition, NameValuePair,
    RootOperationKind,
};
use intern::string_key::Intern;
use isograph_config::CompilerConfigOptions;
use isograph_lang_types::{
    DefinitionLocation, SelectionType, ServerEntityId, ServerObjectId, ServerScalarSelectableId,
    ServerStrongIdFieldId, TypeAnnotation, VariableDefinition,
};
use isograph_schema::{
    EncounteredRootTypes, IsographObjectTypeDefinition, ProcessTypeSystemDocumentOutcome,
    ProcessedRootTypes, RootOperationName, RootTypes, Schema, SchemaObject, SchemaScalar,
    SchemaServerObjectSelectableVariant, ServerScalarSelectable, TypeRefinementMaps,
    ID_GRAPHQL_TYPE, STRING_JAVASCRIPT_TYPE, TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLSchemaObjectAssociatedData, GraphQLSchemaOriginalDefinitionType,
    UnvalidatedGraphqlSchema,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
    static ref ID_FIELD_NAME: ServerScalarSelectableName = "id".intern().into();
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: UnvalidatedTypeName = "String".intern().into();
}

pub fn process_graphql_type_system_document(
    schema: &mut UnvalidatedGraphqlSchema,
    type_system_document: GraphQLTypeSystemDocument,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<ProcessTypeSystemDocumentOutcome> {
    // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
    // with e.g. interfaces "simply" being objects that can be refined to other
    // concrete objects.
    //
    // Processing type system documents is done in two passes:
    // - First, create types for interfaces, objects, scalars, etc.
    // - Then, validate that all implemented interfaces exist, and add refinements
    //   to the found interface.
    let mut supertype_to_subtype_map = HashMap::new();
    let mut subtype_to_supertype_map = HashMap::new();

    let mut encountered_root_types = RootTypes {
        query: None,
        mutation: None,
        subscription: None,
    };
    let mut processed_root_types = None;

    let mut field_queue = HashMap::new();

    for with_location in type_system_document.0 {
        let WithLocation {
            location,
            item: type_system_definition,
        } = with_location;
        match type_system_definition {
            GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                let concrete_type = Some(object_type_definition.name.item.into());

                for interface_name in object_type_definition.interfaces.iter() {
                    insert_into_type_refinement_maps(
                        interface_name.item.into(),
                        object_type_definition.name.item.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                    );
                }

                let object_type_definition = object_type_definition.into();

                let outcome = process_object_type_definition(
                    schema,
                    object_type_definition,
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Object,
                    },
                    GraphQLObjectDefinitionType::Object,
                    &mut field_queue,
                )?;

                if let Some(encountered_root_kind) = outcome.encountered_root_kind {
                    encountered_root_types.set_root_type(encountered_root_kind, outcome.object_id);
                }
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                process_scalar_definition(schema, scalar_type_definition)?;
                // N.B. we assume that Mutation will be an object, not a scalar
            }
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                process_object_type_definition(
                    schema,
                    interface_type_definition.into(),
                    None,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Interface,
                    },
                    GraphQLObjectDefinitionType::Interface,
                    &mut field_queue,
                )?;
                // N.B. we assume that Mutation will be an object, not an interface
            }
            GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                input_object_type_definition,
            ) => {
                let concrete_type = Some(input_object_type_definition.name.item.into());
                process_object_type_definition(
                    schema,
                    input_object_type_definition.into(),
                    // Shouldn't really matter what we pass here
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::InputObject,
                    },
                    GraphQLObjectDefinitionType::InputObject,
                    &mut field_queue,
                )?;
            }
            GraphQLTypeSystemDefinition::DirectiveDefinition(_) => {
                // For now, Isograph ignores directive definitions,
                // but it might choose to allow-list them.
            }
            GraphQLTypeSystemDefinition::EnumDefinition(enum_definition) => {
                // TODO Do not do this
                process_scalar_definition(
                    schema,
                    GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.unchecked_conversion()),
                        directives: enum_definition.directives,
                    },
                )?;
            }
            GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                // TODO do something reasonable here, once we add support for type refinements.
                process_object_type_definition(
                    schema,
                    IsographObjectTypeDefinition {
                        description: union_definition.description,
                        name: union_definition.name.map(|x| x.into()),
                        interfaces: vec![],
                        directives: union_definition.directives,
                        fields: vec![],
                    },
                    None,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Union,
                    },
                    GraphQLObjectDefinitionType::Union,
                    &mut field_queue,
                )?;

                for union_member_type in union_definition.union_member_types {
                    insert_into_type_refinement_maps(
                        union_definition.name.item.into(),
                        union_member_type.item.into(),
                        &mut supertype_to_subtype_map,
                        &mut subtype_to_supertype_map,
                    )
                }
            }
            GraphQLTypeSystemDefinition::SchemaDefinition(schema_definition) => {
                if processed_root_types.is_some() {
                    return Err(WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::DuplicateSchemaDefinition,
                        location,
                    ));
                }
                processed_root_types = Some(RootTypes {
                    query: schema_definition.query,
                    mutation: schema_definition.mutation,
                    subscription: schema_definition.subscription,
                })
            }
        }
    }

    process_fields(schema, field_queue, options)?;

    let type_refinement_map =
        get_type_refinement_map(schema, supertype_to_subtype_map, subtype_to_supertype_map)?;

    let root_types = process_root_types(schema, processed_root_types, encountered_root_types)?;

    if let Some(query_type_id) = root_types.query {
        schema
            .fetchable_types
            .insert(query_type_id, RootOperationName("query".to_string()));
    }
    if let Some(mutation_type_id) = root_types.mutation {
        schema
            .fetchable_types
            .insert(mutation_type_id, RootOperationName("mutation".to_string()));
    }
    // TODO add support for subscriptions

    Ok(ProcessTypeSystemDocumentOutcome {
        root_types,
        type_refinement_maps: type_refinement_map,
    })
}

pub fn process_graphql_type_extension_document(
    schema: &mut UnvalidatedGraphqlSchema,
    extension_document: GraphQLTypeSystemExtensionDocument,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<ProcessTypeSystemDocumentOutcome> {
    let mut definitions = Vec::with_capacity(extension_document.0.len());
    let mut extensions = Vec::with_capacity(extension_document.0.len());

    for extension_or_definition in extension_document.0 {
        let WithLocation { location, item } = extension_or_definition;
        match item {
            GraphQLTypeSystemExtensionOrDefinition::Definition(definition) => {
                definitions.push(WithLocation::new(definition, location));
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                extensions.push(WithLocation::new(extension, location))
            }
        }
    }

    // N.B. we should probably restructure this...?
    // Like, we could discover the mutation type right now!
    let outcome = process_graphql_type_system_document(
        schema,
        GraphQLTypeSystemDocument(definitions),
        options,
    )?;

    for extension in extensions.into_iter() {
        // TODO collect errors into vec
        // TODO we can encounter new interface implementations; we should account for that
        process_graphql_type_system_extension(schema, extension)?;
    }

    Ok(outcome)
}

pub(crate) type ProcessTypeDefinitionResult<T> =
    Result<T, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>;

fn insert_into_type_refinement_maps(
    supertype_name: UnvalidatedTypeName,
    subtype_name: UnvalidatedTypeName, // aka the concrete type or union member
    supertype_to_subtype_map: &mut UnvalidatedTypeRefinementMap,
    subtype_to_supertype_map: &mut UnvalidatedTypeRefinementMap,
) {
    supertype_to_subtype_map
        .entry(supertype_name)
        .or_default()
        .push(subtype_name);
    subtype_to_supertype_map
        .entry(subtype_name)
        .or_default()
        .push(supertype_name);
}

#[derive(Error, Eq, PartialEq, Debug)]
pub(crate) enum ProcessGraphqlTypeSystemDefinitionError {
    // TODO include info about where the type was previously defined
    // TODO the type_definition_name refers to the second object being defined, which isn't
    // all that helpful
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
    },

    #[error(
        "The argument `{argument_name}` on field `{parent_type_name}.{field_name}` has inner type `{argument_type}`, which does not exist."
    )]
    FieldArgumentTypeDoesNotExist {
        argument_name: VariableName,
        parent_type_name: IsographObjectTypeName,
        field_name: SelectableName,
        argument_type: UnvalidatedTypeName,
    },

    // TODO include info about where the field was previously defined
    #[error("Duplicate field named \"{field_name}\" on type \"{parent_type}\"")]
    DuplicateField {
        field_name: SelectableName,
        parent_type: IsographObjectTypeName,
    },

    // TODO
    // This is held in a span pointing to one place the non-existent type was referenced.
    // We should perhaps include info about all the places it was referenced.
    //
    // When type Foo implements Bar and Bar is not defined:
    #[error("Type \"{type_name}\" is never defined.")]
    IsographObjectTypeNameNotDefined { type_name: UnvalidatedTypeName },

    #[error("Expected {type_name} to be an object, but it was a scalar.")]
    GenericObjectIsScalar { type_name: UnvalidatedTypeName },

    #[error(
        "The {strong_field_name} field on \"{parent_type}\" must have type \"ID!\".\n\
    This error can be suppressed using the \"on_invalid_id_type\" config parameter."
    )]
    IdFieldMustBeNonNullIdType {
        parent_type: IsographObjectTypeName,
        strong_field_name: &'static str,
    },

    #[error(
        "The type `{type_name}` is {is_type}, but it is being extended as {extended_as_type}."
    )]
    TypeExtensionMismatch {
        type_name: UnvalidatedTypeName,
        is_type: &'static str,
        extended_as_type: &'static str,
    },

    #[error("Duplicate schema definition")]
    DuplicateSchemaDefinition,

    #[error("Root types must be objects. This type is a scalar.")]
    RootTypeMustBeObject,

    #[error("This field has type {unvalidated_type_name}, which does not exist")]
    FieldTypenameDoesNotExist {
        unvalidated_type_name: UnvalidatedTypeName,
    },
}

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;
// When constructing the final map, we can replace object type names with ids.
pub type ValidatedTypeRefinementMap = HashMap<ServerObjectId, Vec<ServerObjectId>>;

fn process_object_type_definition(
    schema: &mut UnvalidatedGraphqlSchema,
    object_type_definition: IsographObjectTypeDefinition,
    concrete_type: Option<IsographObjectTypeName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
    type_definition_type: GraphQLObjectDefinitionType,
    field_queue: &mut HashMap<ServerObjectId, Vec<WithLocation<GraphQLFieldDefinition>>>,
) -> ProcessTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome> {
    let &mut Schema {
        ref mut server_field_data,
        ..
    } = schema;
    let next_object_id = server_field_data.server_objects.len().into();
    let defined_types = &mut server_field_data.defined_types;
    let server_objects = &mut server_field_data.server_objects;
    let encountered_root_kind = match defined_types.entry(object_type_definition.name.item.into()) {
        HashMapEntry::Occupied(_) => {
            return Err(WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::DuplicateTypeDefinition {
                    // BUG: this could be an interface, actually
                    type_definition_type: type_definition_type.as_str(),
                    type_name: object_type_definition.name.item.into(),
                },
                object_type_definition.name.location,
            ));
        }
        HashMapEntry::Vacant(vacant) => {
            server_objects.push(SchemaObject {
                description: object_type_definition.description.map(|d| d.item),
                name: object_type_definition.name.item,
                id: next_object_id,
                encountered_fields: BTreeMap::new(),
                id_field: None,
                directives: object_type_definition.directives,
                concrete_type,
                output_associated_data: associated_data,
            });

            vacant.insert(ServerEntityId::Object(next_object_id));

            let mut fields_to_insert = object_type_definition.fields;

            // We need to define a typename field for objects and interfaces, but not unions or input objects
            if type_definition_type.has_typename_field() {
                fields_to_insert.push(WithLocation::new(
                    GraphQLFieldDefinition {
                        description: None,
                        name: WithLocation::new(
                            (*TYPENAME_FIELD_NAME).into(),
                            Location::generated(),
                        ),
                        type_: GraphQLTypeAnnotation::NonNull(Box::new(
                            GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                                WithSpan::new(*STRING_TYPE_NAME, Span::todo_generated()),
                            )),
                        )),
                        arguments: vec![],
                        directives: vec![],
                    },
                    Location::generated(),
                ));
            }
            field_queue.insert(next_object_id, fields_to_insert);

            if object_type_definition.name.item == *QUERY_TYPE {
                Some(RootOperationKind::Query)
            } else if object_type_definition.name.item == *MUTATION_TYPE {
                Some(RootOperationKind::Mutation)
            } else {
                // TODO subscription
                None
            }
        }
    };

    Ok(ProcessObjectTypeDefinitionOutcome {
        object_id: next_object_id,
        encountered_root_kind,
    })
}

// TODO This is currently a completely useless function, serving only to surface
// some validation errors. It might be necessary once we handle __asNode etc.
// style fields.
fn get_type_refinement_map(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_supertype_to_subtype_map: UnvalidatedTypeRefinementMap,
    unvalidated_subtype_to_supertype_map: UnvalidatedTypeRefinementMap,
) -> ProcessTypeDefinitionResult<TypeRefinementMaps> {
    let supertype_to_subtype_map =
        validate_type_refinement_map(schema, unvalidated_supertype_to_subtype_map)?;
    let subtype_to_supertype_map =
        validate_type_refinement_map(schema, unvalidated_subtype_to_supertype_map)?;

    Ok(TypeRefinementMaps {
        subtype_to_supertype_map,
        supertype_to_subtype_map,
    })
}

// TODO this should accept an IsographScalarTypeDefinition
fn process_scalar_definition(
    schema: &mut UnvalidatedGraphqlSchema,
    scalar_type_definition: GraphQLScalarTypeDefinition,
) -> ProcessTypeDefinitionResult<()> {
    let &mut Schema {
        ref mut server_field_data,
        ..
    } = schema;
    let next_scalar_id = server_field_data.server_scalars.len().into();
    let type_names = &mut server_field_data.defined_types;
    let scalars = &mut server_field_data.server_scalars;
    match type_names.entry(scalar_type_definition.name.item.into()) {
        HashMapEntry::Occupied(_) => {
            return Err(WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::DuplicateTypeDefinition {
                    type_definition_type: "scalar",
                    type_name: scalar_type_definition.name.item.into(),
                },
                scalar_type_definition.name.location,
            ));
        }
        HashMapEntry::Vacant(vacant) => {
            scalars.push(SchemaScalar {
                description: scalar_type_definition.description,
                name: scalar_type_definition.name,
                id: next_scalar_id,
                javascript_name: *STRING_JAVASCRIPT_TYPE,
                output_format: std::marker::PhantomData,
            });

            vacant.insert(ServerEntityId::Scalar(next_scalar_id));
        }
    }
    Ok(())
}

fn process_root_types(
    schema: &UnvalidatedGraphqlSchema,
    processed_root_types: Option<ProcessedRootTypes>,
    encountered_root_types: EncounteredRootTypes,
) -> ProcessTypeDefinitionResult<EncounteredRootTypes> {
    match processed_root_types {
        Some(processed_root_types) => {
            let RootTypes {
                query: query_type_name,
                mutation: mutation_type_name,
                subscription: subscription_type_name,
            } = processed_root_types;

            let query_id = query_type_name
                .map(|query_type_name| look_up_root_type(schema, query_type_name))
                .transpose()?;
            let mutation_id = mutation_type_name
                .map(|mutation_type_name| look_up_root_type(schema, mutation_type_name))
                .transpose()?;
            let subscription_id = subscription_type_name
                .map(|subscription_type_name| look_up_root_type(schema, subscription_type_name))
                .transpose()?;

            Ok(RootTypes {
                query: query_id,
                mutation: mutation_id,
                subscription: subscription_id,
            })
        }
        None => Ok(encountered_root_types),
    }
}

fn look_up_root_type(
    schema: &UnvalidatedGraphqlSchema,
    type_name: WithLocation<GraphQLObjectTypeName>,
) -> ProcessTypeDefinitionResult<ServerObjectId> {
    match schema
        .server_field_data
        .defined_types
        .get(&type_name.item.into())
    {
        Some(ServerEntityId::Object(object_id)) => Ok(*object_id),
        Some(ServerEntityId::Scalar(_)) => Err(WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::RootTypeMustBeObject,
            type_name.location,
        )),
        None => Err(WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::IsographObjectTypeNameNotDefined {
                type_name: type_name.item.into(),
            },
            type_name.location,
        )),
    }
}

fn process_graphql_type_system_extension(
    schema: &mut UnvalidatedGraphqlSchema,
    extension: WithLocation<GraphQLTypeSystemExtension>,
) -> ProcessTypeDefinitionResult<()> {
    match extension.item {
        GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
            let name = object_extension.name.item;

            let id = schema
                .server_field_data
                .defined_types
                .get(&name.into())
                .expect(
                    "TODO why does this id not exist. This probably indicates a bug in Isograph.",
                );

            match *id {
                ServerEntityId::Object(object_id) => {
                    let schema_object = schema.server_field_data.object_mut(object_id);

                    if !object_extension.fields.is_empty() {
                        panic!("Adding fields in schema extensions is not allowed, yet.");
                    }
                    if !object_extension.interfaces.is_empty() {
                        panic!("Adding interfaces in schema extensions is not allowed, yet.");
                    }

                    schema_object.directives.extend(object_extension.directives);

                    Ok(())
                }
                ServerEntityId::Scalar(_) => Err(WithLocation::new(
                    ProcessGraphqlTypeSystemDefinitionError::TypeExtensionMismatch {
                        type_name: name.into(),
                        is_type: "a scalar",
                        extended_as_type: "an object",
                    },
                    object_extension.name.location,
                )),
            }
        }
    }
}

/// Now that we have processed all objects and scalars, we can process fields (i.e.
/// selectables), as we have the knowledge of whether the field points to a scalar
/// or object.
///
/// For each field:
/// - insert it into to the parent object's encountered_fields
/// - append it to schema.server_fields
/// - if it is an id field, modify the parent object
fn process_fields(
    schema: &mut UnvalidatedGraphqlSchema,
    field_queue: HashMap<ServerObjectId, Vec<WithLocation<GraphQLFieldDefinition>>>,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<()> {
    for (object_id, fields) in field_queue {
        let server_objects = &mut schema.server_field_data.server_objects;
        // Calling .objects takes a reference to all of server_field_data, but we need
        // to also access .defined_types
        let parent_object = &mut server_objects[object_id.as_usize()];

        for field in fields.into_iter() {
            let next_server_field_id = schema.server_scalar_selectables.len().into();
            match parent_object
                .encountered_fields
                .entry(field.item.name.item.into())
            {
                Entry::Vacant(vacant_entry) => {
                    vacant_entry.insert(DefinitionLocation::Server(next_server_field_id));

                    if field.item.name.item == *ID_FIELD_NAME {
                        set_and_validate_id_field(
                            &mut parent_object.id_field,
                            next_server_field_id,
                            &field,
                            parent_object.name,
                            options,
                        )?;
                    }

                    let inner_type = field.item.type_.inner();
                    let target_server_entity = match schema
                        .server_field_data
                        .defined_types
                        .get(inner_type)
                    {
                        Some(selection_type) => match selection_type {
                            SelectionType::Scalar(scalar_id) => SelectionType::Scalar(
                                TypeAnnotation::from_graphql_type_annotation(field.item.type_)
                                    .map(&mut |_| *scalar_id),
                            ),
                            SelectionType::Object(object_id) => SelectionType::Object((
                                SchemaServerObjectSelectableVariant::LinkedField,
                                TypeAnnotation::from_graphql_type_annotation(field.item.type_)
                                    .map(&mut |_| *object_id),
                            )),
                        },
                        None => return Err(WithLocation::new(
                            ProcessGraphqlTypeSystemDefinitionError::FieldTypenameDoesNotExist {
                                unvalidated_type_name: *inner_type,
                            },
                            field.item.name.location,
                        )),
                    };

                    schema
                        .server_scalar_selectables
                        .push(ServerScalarSelectable {
                            description: field.item.description.map(|d| d.item),
                            name: field.item.name,
                            id: next_server_field_id,
                            target_server_entity,
                            parent_type_id: parent_object.id,
                            arguments: field
                                .item
                                .arguments
                                .into_iter()
                                .map(|input_value_definition| {
                                    graphql_input_value_definition_to_variable_definition(
                                        &schema.server_field_data.defined_types,
                                        input_value_definition,
                                        parent_object.name,
                                        field.item.name.item.into(),
                                    )
                                })
                                .collect::<Result<Vec<_>, _>>()?,
                            phantom_data: std::marker::PhantomData,
                        });
                }
                Entry::Occupied(_) => {
                    return Err(WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::DuplicateField {
                            field_name: field.item.name.item.into(),
                            parent_type: parent_object.name,
                        },
                        field.item.name.location,
                    ));
                }
            }
        }
    }

    Ok(())
}

/// If we have encountered an id field, we can:
/// - validate that the id field is properly defined, i.e. has type ID!
/// - set the id field
fn set_and_validate_id_field(
    id_field: &mut Option<ServerStrongIdFieldId>,
    current_field_id: ServerScalarSelectableId,
    field: &WithLocation<GraphQLFieldDefinition>,
    parent_type_name: IsographObjectTypeName,
    options: &CompilerConfigOptions,
) -> ProcessTypeDefinitionResult<()> {
    // N.B. id_field is guaranteed to be None; otherwise field_names_to_type_name would
    // have contained this field name already.
    debug_assert!(id_field.is_none(), "id field should not be defined twice");

    // We should change the type here! It should not be ID! It should be a
    // type specific to the concrete type, e.g. UserID.
    *id_field = Some(current_field_id.unchecked_conversion());

    match field.item.type_.inner_non_null_named_type() {
        Some(type_) => {
            if type_.0.item != *ID_GRAPHQL_TYPE {
                options.on_invalid_id_type.on_failure(|| {
                    WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::IdFieldMustBeNonNullIdType {
                            strong_field_name: "id",
                            parent_type: parent_type_name,
                        },
                        // TODO this shows the wrong span?
                        field.location,
                    )
                })?;
            }
            Ok(())
        }
        None => {
            options.on_invalid_id_type.on_failure(|| {
                WithLocation::new(
                    ProcessGraphqlTypeSystemDefinitionError::IdFieldMustBeNonNullIdType {
                        strong_field_name: "id",
                        parent_type: parent_type_name,
                    },
                    // TODO this shows the wrong span?
                    field.location,
                )
            })?;
            Ok(())
        }
    }
}

fn validate_type_refinement_map(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_type_refinement_map: UnvalidatedTypeRefinementMap,
) -> ProcessTypeDefinitionResult<ValidatedTypeRefinementMap> {
    let supertype_to_subtype_map = unvalidated_type_refinement_map
        .into_iter()
        .map(|(key_type_name, values_type_names)| {
            let key_id = lookup_object_in_schema(schema, key_type_name)?;

            let value_type_ids = values_type_names
                .into_iter()
                .map(|value_type_name| lookup_object_in_schema(schema, value_type_name))
                .collect::<Result<Vec<_>, _>>()?;

            Ok((key_id, value_type_ids))
        })
        .collect::<Result<HashMap<_, _>, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>>(
        )?;
    Ok(supertype_to_subtype_map)
}

fn lookup_object_in_schema(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_type_name: UnvalidatedTypeName,
) -> ProcessTypeDefinitionResult<ServerObjectId> {
    let result = (*schema
        .server_field_data
        .defined_types
        .get(&unvalidated_type_name)
        .ok_or_else(|| {
            WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::IsographObjectTypeNameNotDefined {
                    type_name: unvalidated_type_name,
                },
                // TODO don't do this
                Location::Generated,
            )
        })?)
    .try_into()
    .map_err(|_| {
        WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::GenericObjectIsScalar {
                type_name: unvalidated_type_name,
            },
            // TODO don't do this
            Location::Generated,
        )
    })?;

    Ok(result)
}

pub fn graphql_input_value_definition_to_variable_definition(
    defined_types: &HashMap<UnvalidatedTypeName, ServerEntityId>,
    input_value_definition: WithLocation<GraphQLInputValueDefinition>,
    parent_type_name: IsographObjectTypeName,
    field_name: SelectableName,
) -> ProcessTypeDefinitionResult<WithLocation<VariableDefinition<ServerEntityId>>> {
    let default_value = input_value_definition
        .item
        .default_value
        .map(|graphql_constant_value| {
            Ok::<_, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>(WithLocation::new(
                convert_graphql_constant_value_to_isograph_constant_value(
                    graphql_constant_value.item,
                ),
                graphql_constant_value.location,
            ))
        })
        .transpose()?;

    let type_ = input_value_definition
        .item
        .type_
        .clone()
        .and_then(|input_type_name| {
            defined_types
                .get(&(*input_value_definition.item.type_.inner()).into())
                .ok_or_else(|| {
                    WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::FieldArgumentTypeDoesNotExist {
                            argument_type: input_type_name.into(),
                            argument_name: input_value_definition.item.name.item.into(),
                            parent_type_name,
                            field_name,
                        },
                        input_value_definition.location,
                    )
                })
                .copied()
        })?;

    Ok(WithLocation::new(
        VariableDefinition {
            name: input_value_definition.item.name.map(VariableName::from),
            type_,
            default_value,
        },
        input_value_definition.location,
    ))
}

fn convert_graphql_constant_value_to_isograph_constant_value(
    graphql_constant_value: graphql_lang_types::GraphQLConstantValue,
) -> isograph_lang_types::ConstantValue {
    match graphql_constant_value {
        graphql_lang_types::GraphQLConstantValue::Int(i) => {
            isograph_lang_types::ConstantValue::Integer(i)
        }
        graphql_lang_types::GraphQLConstantValue::Boolean(b) => {
            isograph_lang_types::ConstantValue::Boolean(b)
        }
        graphql_lang_types::GraphQLConstantValue::String(s) => {
            isograph_lang_types::ConstantValue::String(s)
        }
        graphql_lang_types::GraphQLConstantValue::Float(f) => {
            isograph_lang_types::ConstantValue::Float(f)
        }
        graphql_lang_types::GraphQLConstantValue::Null => isograph_lang_types::ConstantValue::Null,
        graphql_lang_types::GraphQLConstantValue::Enum(e) => {
            isograph_lang_types::ConstantValue::Enum(e)
        }
        graphql_lang_types::GraphQLConstantValue::List(l) => {
            let converted_list = l
                .into_iter()
                .map(|x| {
                    WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(x.item),
                        x.location,
                    )
                })
                .collect::<Vec<_>>();
            isograph_lang_types::ConstantValue::List(converted_list)
        }
        graphql_lang_types::GraphQLConstantValue::Object(o) => {
            let converted_object = o
                .into_iter()
                .map(|name_value_pair| NameValuePair {
                    name: name_value_pair.name,
                    value: WithLocation::new(
                        convert_graphql_constant_value_to_isograph_constant_value(
                            name_value_pair.value.item,
                        ),
                        name_value_pair.value.location,
                    ),
                })
                .collect::<Vec<_>>();
            isograph_lang_types::ConstantValue::Object(converted_object)
        }
    }
}

pub struct ProcessObjectTypeDefinitionOutcome {
    pub object_id: ServerObjectId,
    pub encountered_root_kind: Option<RootOperationKind>,
}

#[derive(Clone, Copy)]
enum GraphQLObjectDefinitionType {
    InputObject,
    Union,
    Object,
    Interface,
}

impl GraphQLObjectDefinitionType {
    pub fn as_str(&self) -> &'static str {
        match self {
            GraphQLObjectDefinitionType::InputObject => "input object",
            GraphQLObjectDefinitionType::Union => "union",
            GraphQLObjectDefinitionType::Object => "object",
            GraphQLObjectDefinitionType::Interface => "interface",
        }
    }

    pub fn has_typename_field(&self) -> bool {
        match self {
            GraphQLObjectDefinitionType::InputObject => false,
            GraphQLObjectDefinitionType::Union => false,
            GraphQLObjectDefinitionType::Object => true,
            GraphQLObjectDefinitionType::Interface => true,
        }
    }
}
