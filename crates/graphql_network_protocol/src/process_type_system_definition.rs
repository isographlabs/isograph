use std::collections::HashMap;

use common_lang_types::{
    GraphQLInterfaceTypeName, Location, SelectableName, ServerObjectEntityName,
    ServerScalarSelectableName, Span, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition, RootOperationKind,
};
use intern::string_key::Intern;
use isograph_schema::{
    CreateAdditionalFieldsError, ExposeAsFieldToInsert, ExposeFieldDirective, FieldMapItem,
    FieldToInsert, IsographObjectTypeDefinition, ProcessObjectTypeDefinitionOutcome,
    ProcessTypeSystemDocumentOutcome, RootTypes, ServerObjectEntity, ServerScalarEntity,
    STRING_JAVASCRIPT_TYPE, TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLNetworkProtocol, GraphQLSchemaObjectAssociatedData, GraphQLSchemaOriginalDefinitionType,
};

lazy_static! {
    pub static ref QUERY_TYPE: ServerObjectEntityName = "Query".intern().into();
    static ref MUTATION_TYPE: ServerObjectEntityName = "Mutation".intern().into();
    static ref SUBSCRIPTION_TYPE: ServerObjectEntityName = "Subscription".intern().into();
    static ref ID_FIELD_NAME: ServerScalarSelectableName = "id".intern().into();
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: UnvalidatedTypeName = "String".intern().into();
    static ref NODE_INTERFACE_NAME: GraphQLInterfaceTypeName = "Node".intern().into();
    pub static ref REFETCH_FIELD_NAME: SelectableName = "__refetch".intern().into();

}

#[allow(clippy::type_complexity)]
pub fn process_graphql_type_system_document(
    type_system_document: GraphQLTypeSystemDocument,
) -> ProcessGraphqlTypeDefinitionResult<(
    ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>,
    HashMap<ServerObjectEntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    Vec<ExposeAsFieldToInsert>,
)> {
    // TODO return a vec of errors, not just one

    // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
    // with e.g. interfaces "simply" being objects that can be refined to other
    // concrete objects.

    let mut supertype_to_subtype_map = HashMap::new();

    let mut processed_root_types = None;

    let mut scalars = vec![];
    let mut objects = vec![];
    let mut directives = HashMap::<_, Vec<_>>::new();

    let mut refetch_fields = vec![];

    for with_location in type_system_document.0 {
        let WithLocation {
            location,
            item: type_system_definition,
        } = with_location;
        match type_system_definition {
            GraphQLTypeSystemDefinition::ObjectTypeDefinition(object_type_definition) => {
                let concrete_type = Some(object_type_definition.name.item.into());

                for interface_name in object_type_definition.interfaces.iter() {
                    insert_into_type_refinement_map(
                        interface_name.item.into(),
                        object_type_definition.name.item.into(),
                        &mut supertype_to_subtype_map,
                    );
                }

                let object_name = object_type_definition.name.item.unchecked_conversion();
                let object_type_definition = object_type_definition.into();

                let (object_definition_outcome, new_directives) = process_object_type_definition(
                    object_type_definition,
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Object,
                    },
                    GraphQLObjectDefinitionType::Object,
                    &mut refetch_fields,
                )?;

                directives
                    .entry(object_name)
                    .or_default()
                    .extend(new_directives);

                objects.push((object_definition_outcome, location));
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                scalars.push((process_scalar_definition(scalar_type_definition), location));
                // N.B. we assume that Mutation will be an object, not a scalar
            }
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                let interface_name = interface_type_definition.name.item.unchecked_conversion();
                let (process_object_type_definition_outcome, new_directives) =
                    process_object_type_definition(
                        interface_type_definition.into(),
                        None,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::Interface,
                        },
                        GraphQLObjectDefinitionType::Interface,
                        &mut refetch_fields,
                    )?;
                objects.push((process_object_type_definition_outcome, location));

                directives
                    .entry(interface_name)
                    .or_default()
                    .extend(new_directives);
                // N.B. we assume that Mutation will be an object, not an interface
            }
            GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                input_object_type_definition,
            ) => {
                let concrete_type = Some(input_object_type_definition.name.item.into());
                let input_object_name = input_object_type_definition
                    .name
                    .item
                    .unchecked_conversion();
                let (process_object_type_definition_outcome, new_directives) =
                    process_object_type_definition(
                        input_object_type_definition.into(),
                        // Shouldn't really matter what we pass here
                        concrete_type,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::InputObject,
                        },
                        GraphQLObjectDefinitionType::InputObject,
                        &mut refetch_fields,
                    )?;
                objects.push((process_object_type_definition_outcome, location));
                directives
                    .entry(input_object_name)
                    .or_default()
                    .extend(new_directives);
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
                let (process_object_type_definition_outcome, new_directives) =
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
                        &mut refetch_fields,
                    )?;
                objects.push((process_object_type_definition_outcome, location));
                directives
                    .entry(union_definition.name.item.unchecked_conversion())
                    .or_default()
                    .extend(new_directives);

                for union_member_type in union_definition.union_member_types {
                    insert_into_type_refinement_map(
                        union_definition.name.item.into(),
                        union_member_type.item.into(),
                        &mut supertype_to_subtype_map,
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

    // For each supertype (e.g. Node) and a subtype (e.g. Pet), we need to add an asConcreteType field.
    for (supertype_name, subtypes) in supertype_to_subtype_map.iter() {
        if let Some((object_outcome, _)) = objects.iter_mut().find(|obj| {
            let supertype_name: ServerObjectEntityName = supertype_name.unchecked_conversion();

            obj.0.server_object_entity.name == supertype_name
        }) {
            for subtype_name in subtypes.iter() {
                object_outcome.fields_to_insert.push(WithLocation::new(
                    FieldToInsert {
                        description: Some(WithSpan::new(
                            format!("A client pointer for the {subtype_name} type.")
                                .intern()
                                .into(),
                            Span::todo_generated(),
                        )),
                        name: WithLocation::new(
                            format!("as{subtype_name}").intern().into(),
                            Location::generated(),
                        ),
                        type_: GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                            WithSpan::new(*subtype_name, Span::todo_generated()),
                        )),
                        arguments: vec![],
                        is_inline_fragment: true,
                    },
                    Location::generated(),
                ));
            }
        } else {
            return Err(WithLocation::new(
                ProcessGraphqlTypeSystemDefinitionError::AttemptedToImplementNonExistentType {
                subtype_name: *subtypes.first().expect("Expected subtypes not to be empty. This is indicative of a bug in Isograph."),
                    supertype_name: *supertype_name,
                },
                Location::generated(),
            ));
        };
    }

    Ok((
        ProcessTypeSystemDocumentOutcome { scalars, objects },
        directives,
        refetch_fields,
    ))
}

#[allow(clippy::type_complexity)]
pub fn process_graphql_type_extension_document(
    extension_document: GraphQLTypeSystemExtensionDocument,
) -> ProcessGraphqlTypeDefinitionResult<(
    ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>,
    HashMap<ServerObjectEntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    Vec<ExposeAsFieldToInsert>,
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

    let (outcome, mut directives, refetch_fields) =
        process_graphql_type_system_document(GraphQLTypeSystemDocument(definitions))?;

    for extension in extensions.into_iter() {
        // TODO collect errors into vec
        // TODO we can encounter new interface implementations; we should account for that

        for (name, new_directives) in process_graphql_type_system_extension(extension) {
            directives.entry(name).or_default().extend(new_directives);
        }
    }

    Ok((outcome, directives, refetch_fields))
}

pub(crate) type ProcessGraphqlTypeDefinitionResult<T> =
    Result<T, WithLocation<ProcessGraphqlTypeSystemDefinitionError>>;

#[derive(Error, Eq, PartialEq, Debug, Clone)]
pub enum ProcessGraphqlTypeSystemDefinitionError {
    #[error("Duplicate schema definition")]
    DuplicateSchemaDefinition,

    #[error("{0}")]
    CreateAdditionalFieldsError(#[from] CreateAdditionalFieldsError),

    #[error("Attempted to extend {type_name}, but that type is not defined")]
    AttemptedToExtendUndefinedType { type_name: ServerObjectEntityName },

    #[error("Type {subtype_name} claims to implement {supertype_name}, but {supertype_name} is not a type that has been defined.")]
    AttemptedToImplementNonExistentType {
        subtype_name: UnvalidatedTypeName,
        supertype_name: UnvalidatedTypeName,
    },
}

fn process_object_type_definition(
    object_type_definition: IsographObjectTypeDefinition,
    concrete_type: Option<ServerObjectEntityName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
    type_definition_type: GraphQLObjectDefinitionType,
    refetch_fields: &mut Vec<ExposeAsFieldToInsert>,
) -> ProcessGraphqlTypeDefinitionResult<(
    ProcessObjectTypeDefinitionOutcome<GraphQLNetworkProtocol>,
    Vec<GraphQLDirective<GraphQLConstantValue>>,
)> {
    let object_implements_node = implements_node(&object_type_definition);
    let server_object_entity = ServerObjectEntity {
        description: object_type_definition.description.map(|d| d.item),
        name: object_type_definition.name.item,
        concrete_type,
        output_associated_data: associated_data,
    };

    let mut fields_to_insert: Vec<_> = object_type_definition
        .fields
        .into_iter()
        .map(|field_definition| {
            WithLocation::new(
                FieldToInsert {
                    description: field_definition.item.description,
                    name: field_definition.item.name,
                    type_: field_definition.item.type_,
                    arguments: field_definition.item.arguments,
                    is_inline_fragment: field_definition.item.is_inline_fragment,
                },
                field_definition.location,
            )
        })
        .collect();

    // We need to define a typename field for objects and interfaces, but not unions or input objects
    if type_definition_type.has_typename_field() {
        fields_to_insert.push(WithLocation::new(
            FieldToInsert {
                description: None,
                name: WithLocation::new((*TYPENAME_FIELD_NAME).into(), Location::generated()),
                type_: GraphQLTypeAnnotation::NonNull(Box::new(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                        *STRING_TYPE_NAME,
                        Span::todo_generated(),
                    ))),
                )),
                arguments: vec![],
                is_inline_fragment: false,
            },
            Location::generated(),
        ));
    }

    if object_implements_node {
        refetch_fields.push(ExposeAsFieldToInsert {
            expose_field_directive: ExposeFieldDirective {
                expose_as: Some(*REFETCH_FIELD_NAME),
                field_map: vec![FieldMapItem {
                    from: (*ID_FIELD_NAME).unchecked_conversion(),
                    to: (*ID_FIELD_NAME).unchecked_conversion(),
                }],
                field: format!("node.as{}", object_type_definition.name.item)
                    .intern()
                    .into(),
            },
            parent_object_name: object_type_definition.name.item,
            description: Some(
                format!(
                    "A refetch field for the {} type.",
                    object_type_definition.name.item
                )
                .intern()
                .into(),
            ),
        });
    }

    let encountered_root_kind = if object_type_definition.name.item == *QUERY_TYPE {
        Some(RootOperationKind::Query)
    } else if object_type_definition.name.item == *MUTATION_TYPE {
        Some(RootOperationKind::Mutation)
    } else if object_type_definition.name.item == *SUBSCRIPTION_TYPE {
        Some(RootOperationKind::Subscription)
    } else {
        None
    };

    Ok((
        ProcessObjectTypeDefinitionOutcome {
            encountered_root_kind,
            server_object_entity,
            fields_to_insert,
            expose_as_fields_to_insert: vec![],
        },
        object_type_definition.directives,
    ))
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
) -> HashMap<ServerObjectEntityName, Vec<GraphQLDirective<GraphQLConstantValue>>> {
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

fn insert_into_type_refinement_map(
    supertype_name: UnvalidatedTypeName,
    subtype_name: UnvalidatedTypeName, // aka the concrete type or union member
    supertype_to_subtype_map: &mut UnvalidatedTypeRefinementMap,
) {
    supertype_to_subtype_map
        .entry(supertype_name)
        .or_default()
        .push(subtype_name);
}

type UnvalidatedTypeRefinementMap = HashMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;

fn implements_node(object_type_definition: &IsographObjectTypeDefinition) -> bool {
    object_type_definition
        .interfaces
        .iter()
        .any(|x| x.item == *NODE_INTERFACE_NAME)
}
