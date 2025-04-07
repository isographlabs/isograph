use std::collections::{hash_map::Entry as HashMapEntry, HashMap};

use common_lang_types::{
    GraphQLObjectTypeName, IsographObjectTypeName, Location, ServerScalarSelectableName, Span,
    UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLNamedTypeAnnotation, GraphQLNonNullTypeAnnotation,
    GraphQLScalarTypeDefinition, GraphQLTypeAnnotation, GraphQLTypeSystemDefinition,
    GraphQLTypeSystemDocument, GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionDocument,
    GraphQLTypeSystemExtensionOrDefinition, RootOperationKind,
};
use intern::string_key::Intern;
use isograph_lang_types::{ServerEntityId, ServerObjectEntityId};
use isograph_schema::{
    EncounteredRootTypes, InsertFieldsError, IsographObjectTypeDefinition,
    ProcessTypeSystemDocumentOutcome, ProcessedRootTypes, RootOperationName, RootTypes, Schema,
    ServerObjectEntity, ServerScalarEntity, TypeRefinementMaps, STRING_JAVASCRIPT_TYPE,
    TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLNetworkProtocol, GraphQLSchemaObjectAssociatedData, GraphQLSchemaOriginalDefinitionType,
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
) -> ProcessGraphqlTypeDefinitionResult<ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>> {
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

    let mut scalars = vec![];

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
                    encountered_root_types
                        .set_root_type(encountered_root_kind, outcome.object_entity_id);
                }
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                scalars.push((process_scalar_definition(scalar_type_definition), location));
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
                scalars.push((
                    process_scalar_definition(GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.unchecked_conversion()),
                        directives: enum_definition.directives,
                    }),
                    location,
                ));
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
        type_refinement_maps: type_refinement_map,
        scalars,
        field_queue,
    })
}

pub fn process_graphql_type_extension_document(
    schema: &mut UnvalidatedGraphqlSchema,
    extension_document: GraphQLTypeSystemExtensionDocument,
) -> ProcessGraphqlTypeDefinitionResult<ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>> {
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
    let outcome =
        process_graphql_type_system_document(schema, GraphQLTypeSystemDocument(definitions))?;

    for extension in extensions.into_iter() {
        // TODO collect errors into vec
        // TODO we can encounter new interface implementations; we should account for that
        process_graphql_type_system_extension(schema, extension)?;
    }

    Ok(outcome)
}

pub(crate) type ProcessGraphqlTypeDefinitionResult<T> =
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
pub enum ProcessGraphqlTypeSystemDefinitionError {
    // TODO include info about where the type was previously defined
    // TODO the type_definition_name refers to the second object being defined, which isn't
    // all that helpful
    #[error("Duplicate type definition ({type_definition_type}) named \"{type_name}\"")]
    DuplicateTypeDefinition {
        type_definition_type: &'static str,
        type_name: UnvalidatedTypeName,
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

    #[error("{0}")]
    InsertFieldsError(#[from] InsertFieldsError),
}

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;
// When constructing the final map, we can replace object type names with ids.
pub type ValidatedTypeRefinementMap = HashMap<ServerObjectEntityId, Vec<ServerObjectEntityId>>;

fn process_object_type_definition(
    schema: &mut UnvalidatedGraphqlSchema,
    object_type_definition: IsographObjectTypeDefinition,
    concrete_type: Option<IsographObjectTypeName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
    type_definition_type: GraphQLObjectDefinitionType,
    field_queue: &mut HashMap<ServerObjectEntityId, Vec<WithLocation<GraphQLFieldDefinition>>>,
) -> ProcessGraphqlTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome> {
    let &mut Schema {
        ref mut server_entity_data,
        ..
    } = schema;
    let next_object_entity_id = server_entity_data.server_objects.len().into();
    let defined_types = &mut server_entity_data.defined_entities;
    let server_objects = &mut server_entity_data.server_objects;

    server_entity_data
        .server_object_entity_available_selectables
        .entry(next_object_entity_id)
        .or_default()
        .2
        .extend(object_type_definition.directives);

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
            server_objects.push(ServerObjectEntity {
                description: object_type_definition.description.map(|d| d.item),
                name: object_type_definition.name.item,
                concrete_type,
                output_associated_data: associated_data,
            });

            vacant.insert(ServerEntityId::Object(next_object_entity_id));

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
            field_queue.insert(next_object_entity_id, fields_to_insert);

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
        object_entity_id: next_object_entity_id,
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
) -> ProcessGraphqlTypeDefinitionResult<TypeRefinementMaps> {
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
    scalar_type_definition: GraphQLScalarTypeDefinition,
) -> ServerScalarEntity<GraphQLNetworkProtocol> {
    ServerScalarEntity {
        description: scalar_type_definition.description,
        name: scalar_type_definition.name,
        javascript_name: *STRING_JAVASCRIPT_TYPE,
        output_format: std::marker::PhantomData,
    }
}

fn process_root_types(
    schema: &UnvalidatedGraphqlSchema,
    processed_root_types: Option<ProcessedRootTypes>,
    encountered_root_types: EncounteredRootTypes,
) -> ProcessGraphqlTypeDefinitionResult<EncounteredRootTypes> {
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
) -> ProcessGraphqlTypeDefinitionResult<ServerObjectEntityId> {
    match schema
        .server_entity_data
        .defined_entities
        .get(&type_name.item.into())
    {
        Some(ServerEntityId::Object(object_entity_id)) => Ok(*object_entity_id),
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
) -> ProcessGraphqlTypeDefinitionResult<()> {
    match extension.item {
        GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
            let name = object_extension.name.item;

            let id = schema
                .server_entity_data
                .defined_entities
                .get(&name.into())
                .expect(
                    "TODO why does this id not exist. This probably indicates a bug in Isograph.",
                );

            match *id {
                ServerEntityId::Object(object_entity_id) => {
                    if !object_extension.fields.is_empty() {
                        panic!("Adding fields in schema extensions is not allowed, yet.");
                    }
                    if !object_extension.interfaces.is_empty() {
                        panic!("Adding interfaces in schema extensions is not allowed, yet.");
                    }

                    schema
                        .server_entity_data
                        .server_object_entity_available_selectables
                        .entry(object_entity_id)
                        .or_default()
                        .2
                        .extend(object_extension.directives);

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

fn validate_type_refinement_map(
    schema: &mut UnvalidatedGraphqlSchema,
    unvalidated_type_refinement_map: UnvalidatedTypeRefinementMap,
) -> ProcessGraphqlTypeDefinitionResult<ValidatedTypeRefinementMap> {
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
) -> ProcessGraphqlTypeDefinitionResult<ServerObjectEntityId> {
    let result = (*schema
        .server_entity_data
        .defined_entities
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
    .as_object_result()
    .map_err(|_| {
        WithLocation::new(
            ProcessGraphqlTypeSystemDefinitionError::GenericObjectIsScalar {
                type_name: unvalidated_type_name,
            },
            // TODO don't do this
            Location::Generated,
        )
    })?;

    Ok(*result)
}

pub struct ProcessObjectTypeDefinitionOutcome {
    pub object_entity_id: ServerObjectEntityId,
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
