use std::collections::HashMap;

use common_lang_types::{
    IsographObjectTypeName, Location, ServerScalarSelectableName, Span, UnvalidatedTypeName,
    WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLFieldDefinition, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition, RootOperationKind,
};
use intern::string_key::Intern;
use isograph_schema::{
    insert_fields_error::InsertFieldsError, IsographObjectTypeDefinition,
    ProcessObjectTypeDefinitionOutcome, ProcessTypeSystemDocumentOutcome, RootTypes,
    ServerObjectEntity, ServerScalarEntity, STRING_JAVASCRIPT_TYPE, TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLNetworkProtocol, GraphQLSchemaObjectAssociatedData, GraphQLSchemaOriginalDefinitionType,
};

lazy_static! {
    static ref QUERY_TYPE: IsographObjectTypeName = "Query".intern().into();
    static ref MUTATION_TYPE: IsographObjectTypeName = "Mutation".intern().into();
    static ref ID_FIELD_NAME: ServerScalarSelectableName = "id".intern().into();
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: UnvalidatedTypeName = "String".intern().into();
}

pub fn process_graphql_type_system_document(
    type_system_document: GraphQLTypeSystemDocument,
) -> ProcessGraphqlTypeDefinitionResult<ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>> {
    // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
    // with e.g. interfaces "simply" being objects that can be refined to other
    // concrete objects.

    let mut supertype_to_subtype_map = HashMap::new();
    let mut subtype_to_supertype_map = HashMap::new();

    let mut processed_root_types = None;

    let mut scalars = vec![];
    let mut objects = vec![];

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

                objects.push((
                    process_object_type_definition(
                        object_type_definition,
                        concrete_type,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type: GraphQLSchemaOriginalDefinitionType::Object,
                        },
                        GraphQLObjectDefinitionType::Object,
                    )?,
                    location,
                ));
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                scalars.push((process_scalar_definition(scalar_type_definition), location));
                // N.B. we assume that Mutation will be an object, not a scalar
            }
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                objects.push((
                    process_object_type_definition(
                        interface_type_definition.into(),
                        None,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::Interface,
                        },
                        GraphQLObjectDefinitionType::Interface,
                    )?,
                    location,
                ));
                // N.B. we assume that Mutation will be an object, not an interface
            }
            GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                input_object_type_definition,
            ) => {
                let concrete_type = Some(input_object_type_definition.name.item.into());
                objects.push((
                    process_object_type_definition(
                        input_object_type_definition.into(),
                        // Shouldn't really matter what we pass here
                        concrete_type,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::InputObject,
                        },
                        GraphQLObjectDefinitionType::InputObject,
                    )?,
                    location,
                ));
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
                objects.push((
                    process_object_type_definition(
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
                    )?,
                    location,
                ));

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

    Ok(ProcessTypeSystemDocumentOutcome {
        scalars,
        objects,
        unvalidated_subtype_to_supertype_map: subtype_to_supertype_map,
        unvalidated_supertype_to_subtype_map: supertype_to_subtype_map,
    })
}

#[allow(clippy::type_complexity)]
pub fn process_graphql_type_extension_document(
    extension_document: GraphQLTypeSystemExtensionDocument,
) -> ProcessGraphqlTypeDefinitionResult<(
    ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>,
    HashMap<IsographObjectTypeName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
)> {
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
    let outcome = process_graphql_type_system_document(GraphQLTypeSystemDocument(definitions))?;

    let mut directives = HashMap::new();
    for extension in extensions.into_iter() {
        // TODO collect errors into vec
        // TODO we can encounter new interface implementations; we should account for that
        directives.extend(process_graphql_type_system_extension(extension));
    }

    Ok((outcome, directives))
}

pub(crate) type ProcessGraphqlTypeDefinitionResult<T> =
    Result<T, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>;

#[derive(Error, Eq, PartialEq, Debug)]
pub enum ProcessGraphqlTypeSystemDefinitionError {
    #[error("Duplicate schema definition")]
    DuplicateSchemaDefinition,

    #[error("{0}")]
    InsertFieldsError(#[from] InsertFieldsError),

    #[error("Attempted to extend {type_name}, but that type is not defined")]
    AttemptedToExtendUndefinedType { type_name: IsographObjectTypeName },
}

fn process_object_type_definition(
    object_type_definition: IsographObjectTypeDefinition,
    concrete_type: Option<IsographObjectTypeName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
    type_definition_type: GraphQLObjectDefinitionType,
) -> ProcessGraphqlTypeDefinitionResult<ProcessObjectTypeDefinitionOutcome<GraphQLNetworkProtocol>>
{
    let server_object_entity = ServerObjectEntity {
        description: object_type_definition.description.map(|d| d.item),
        name: object_type_definition.name.item,
        concrete_type,
        output_associated_data: associated_data,
    };

    let mut fields_to_insert = object_type_definition.fields;

    // We need to define a typename field for objects and interfaces, but not unions or input objects
    if type_definition_type.has_typename_field() {
        fields_to_insert.push(WithLocation::new(
            GraphQLFieldDefinition {
                description: None,
                name: WithLocation::new((*TYPENAME_FIELD_NAME).into(), Location::generated()),
                type_: GraphQLTypeAnnotation::NonNull(Box::new(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                        *STRING_TYPE_NAME,
                        Span::todo_generated(),
                    ))),
                )),
                arguments: vec![],
                directives: vec![],
            },
            Location::generated(),
        ));
    }

    let encountered_root_kind = if object_type_definition.name.item == *QUERY_TYPE {
        Some(RootOperationKind::Query)
    } else if object_type_definition.name.item == *MUTATION_TYPE {
        Some(RootOperationKind::Mutation)
    } else {
        // TODO subscription
        None
    };

    Ok(ProcessObjectTypeDefinitionOutcome {
        encountered_root_kind,
        directives: object_type_definition.directives,
        server_object_entity,
        fields_to_insert,
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

fn process_graphql_type_system_extension(
    extension: WithLocation<GraphQLTypeSystemExtension>,
) -> HashMap<IsographObjectTypeName, Vec<GraphQLDirective<GraphQLConstantValue>>> {
    let mut types_and_directives = HashMap::new();
    match extension.item {
        GraphQLTypeSystemExtension::ObjectTypeExtension(object_extension) => {
            types_and_directives.insert(
                object_extension.name.item.into(),
                object_extension.directives,
            );
        }
    }

    types_and_directives
}

#[derive(Clone, Copy)]
enum GraphQLObjectDefinitionType {
    InputObject,
    Union,
    Object,
    Interface,
}

impl GraphQLObjectDefinitionType {
    pub fn has_typename_field(&self) -> bool {
        match self {
            GraphQLObjectDefinitionType::InputObject => false,
            GraphQLObjectDefinitionType::Union => false,
            GraphQLObjectDefinitionType::Object => true,
            GraphQLObjectDefinitionType::Interface => true,
        }
    }
}

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

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;
