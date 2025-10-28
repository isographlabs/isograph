use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    GraphQLInterfaceTypeName, Location, SelectableName, ServerObjectEntityName,
    ServerScalarSelectableName, Span, UnvalidatedTypeName, WithLocation, WithSpan,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{Description, SelectionType};
use isograph_schema::{
    ExposeFieldDirective, ExposeFieldToInsert, FieldMapItem, FieldToInsert,
    IsographObjectTypeDefinition, ParseTypeSystemOutcome, ProcessObjectTypeDefinitionOutcome,
    STRING_JAVASCRIPT_TYPE, ServerObjectEntity, ServerScalarEntity, TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    GraphQLNetworkProtocol, GraphQLRootTypes, GraphQLSchemaObjectAssociatedData,
    GraphQLSchemaOriginalDefinitionType,
};

lazy_static! {
    static ref ID_FIELD_NAME: ServerScalarSelectableName = "id".intern().into();
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: UnvalidatedTypeName = "String".intern().into();
    static ref NODE_INTERFACE_NAME: GraphQLInterfaceTypeName = "Node".intern().into();
    pub static ref REFETCH_FIELD_NAME: SelectableName = "__refetch".intern().into();

}

#[allow(clippy::type_complexity)]
pub fn process_graphql_type_system_document(
    type_system_document: GraphQLTypeSystemDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
) -> ProcessGraphqlTypeDefinitionResult<(
    ParseTypeSystemOutcome<GraphQLNetworkProtocol>,
    // TODO why are we returning these?
    HashMap<ServerObjectEntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    Vec<ExposeFieldToInsert>,
)> {
    // TODO return a vec of errors, not just one

    // In the schema, interfaces, unions and objects are the same type of object (SchemaType),
    // with e.g. interfaces "simply" being objects that can be refined to other
    // concrete objects.

    let mut supertype_to_subtype_map = BTreeMap::new();

    let mut type_system_entities = vec![];

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
                        subtypes: vec![],
                    },
                    GraphQLObjectDefinitionType::Object,
                    &mut refetch_fields,
                )?;

                directives
                    .entry(object_name)
                    .or_default()
                    .extend(new_directives);

                type_system_entities.push(SelectionType::Object(object_definition_outcome));
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                let name_location = scalar_type_definition.name.location;
                type_system_entities.push(SelectionType::Scalar(WithLocation::new(
                    process_scalar_definition(scalar_type_definition),
                    name_location,
                )));
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
                            subtypes: vec![],
                        },
                        GraphQLObjectDefinitionType::Interface,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(SelectionType::Object(
                    process_object_type_definition_outcome,
                ));

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
                            subtypes: vec![],
                        },
                        GraphQLObjectDefinitionType::InputObject,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(SelectionType::Object(
                    process_object_type_definition_outcome,
                ));

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
                type_system_entities.push(SelectionType::Scalar(WithLocation::new(
                    process_scalar_definition(GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.unchecked_conversion()),
                        directives: enum_definition.directives,
                    }),
                    enum_definition.name.location,
                )))
            }
            GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                // TODO do something reasonable here, once we add support for type refinements.
                let (process_object_type_definition_outcome, new_directives) =
                    process_object_type_definition(
                        IsographObjectTypeDefinition {
                            description: union_definition
                                .description
                                .map(|with_span| with_span.map(|dv| dv.into())),
                            name: union_definition.name.map(|x| x.into()),
                            interfaces: vec![],
                            directives: union_definition.directives,
                            fields: vec![],
                        },
                        None,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type: GraphQLSchemaOriginalDefinitionType::Union,
                            subtypes: vec![],
                        },
                        GraphQLObjectDefinitionType::Union,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(SelectionType::Object(
                    process_object_type_definition_outcome,
                ));

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
                if graphql_root_types.is_some() {
                    return Err(WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::DuplicateSchemaDefinition,
                        location,
                    ));
                }
                *graphql_root_types = Some(GraphQLRootTypes {
                    query: schema_definition
                        .query
                        .map(|x| x.item.into())
                        .unwrap_or_else(|| "Query".intern().into()),
                    mutation: schema_definition
                        .mutation
                        .map(|x| x.item.into())
                        .unwrap_or_else(|| "Mutation".intern().into()),
                    subscription: schema_definition
                        .subscription
                        .map(|x| x.item.into())
                        .unwrap_or_else(|| "Subscription".intern().into()),
                })
            }
        }
    }

    // For each supertype (e.g. Node) and a subtype (e.g. Pet), we need to add an asConcreteType field.
    for (supertype_name, subtypes) in supertype_to_subtype_map.iter() {
        let object_outcome = type_system_entities
            .iter_mut()
            .find_map(|definition| {
                definition.as_ref_mut().as_object().and_then(|x| {
                    // Why do we have to do this?
                    let supertype_name: ServerObjectEntityName =
                        supertype_name.unchecked_conversion();
                    if x.server_object_entity.item.name.item == supertype_name {
                        Some(x)
                    } else {
                        None
                    }
                })
            })
            .expect("Expected supertype to exist. This is indicative of a bug in Isograph.");

        // add subtypes to associated_data
        object_outcome
            .server_object_entity
            .item
            .network_protocol_associated_data
            .subtypes
            .extend(subtypes);

        for subtype_name in subtypes.iter() {
            object_outcome.fields_to_insert.push(WithLocation::new(
                FieldToInsert {
                    description: Some(WithSpan::new(
                        Description(
                            format!("A client pointer for the {subtype_name} type.")
                                .intern()
                                .into(),
                        ),
                        Span::todo_generated(),
                    )),
                    name: WithLocation::new(
                        format!("as{subtype_name}").intern().into(),
                        Location::generated(),
                    ),
                    graphql_type: GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                        WithSpan::new(*subtype_name, Span::todo_generated()),
                    )),
                    javascript_type_override: None,
                    arguments: vec![],
                    is_inline_fragment: true,
                },
                Location::generated(),
            ));
        }
    }

    Ok((type_system_entities, directives, refetch_fields))
}

#[allow(clippy::type_complexity)]
pub fn process_graphql_type_extension_document(
    extension_document: GraphQLTypeSystemExtensionDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
) -> ProcessGraphqlTypeDefinitionResult<(
    ParseTypeSystemOutcome<GraphQLNetworkProtocol>,
    HashMap<ServerObjectEntityName, Vec<GraphQLDirective<GraphQLConstantValue>>>,
    Vec<ExposeFieldToInsert>,
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

    let (outcome, mut directives, refetch_fields) = process_graphql_type_system_document(
        GraphQLTypeSystemDocument(definitions),
        graphql_root_types,
    )?;

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

    #[error("Attempted to extend {type_name}, but that type is not defined")]
    AttemptedToExtendUndefinedType { type_name: ServerObjectEntityName },

    #[error(
        "Type {subtype_name} claims to implement {supertype_name}, but {supertype_name} is not a type that has been defined."
    )]
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
    refetch_fields: &mut Vec<ExposeFieldToInsert>,
) -> ProcessGraphqlTypeDefinitionResult<(
    ProcessObjectTypeDefinitionOutcome<GraphQLNetworkProtocol>,
    Vec<GraphQLDirective<GraphQLConstantValue>>,
)> {
    let should_add_refetch_field = type_definition_type.is_concrete()
        && type_definition_type.is_output_type()
        && type_implements_node(&object_type_definition);
    let server_object_entity = WithLocation::new(
        ServerObjectEntity {
            description: object_type_definition.description.map(|d| d.item),
            name: object_type_definition.name,
            concrete_type,
            network_protocol_associated_data: associated_data,
        },
        object_type_definition.name.location.into(),
    );

    let mut fields_to_insert: Vec<_> = object_type_definition
        .fields
        .into_iter()
        .map(|field_definition| {
            WithLocation::new(
                FieldToInsert {
                    description: field_definition
                        .item
                        .description
                        .map(|with_span| with_span.map(|dv| dv.into())),
                    name: field_definition.item.name,
                    graphql_type: field_definition.item.type_,
                    javascript_type_override: None,

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
                graphql_type: GraphQLTypeAnnotation::NonNull(Box::new(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                        *STRING_TYPE_NAME,
                        Span::todo_generated(),
                    ))),
                )),
                // This is bad data modeling, and we should do better.
                javascript_type_override: concrete_type.map(|parent_concrete_type| {
                    format!("'{parent_concrete_type}'").intern().into()
                }),

                arguments: vec![],
                is_inline_fragment: false,
            },
            Location::generated(),
        ));
    }

    if should_add_refetch_field {
        refetch_fields.push(ExposeFieldToInsert {
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
            description: Some(Description(
                format!(
                    "A refetch field for the {} type.",
                    object_type_definition.name.item
                )
                .intern()
                .into(),
            )),
        });
    }

    Ok((
        ProcessObjectTypeDefinitionOutcome {
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
        description: scalar_type_definition
            .description
            .map(|with_span| with_span.map(|dv| dv.into())),
        name: scalar_type_definition.name,
        // TODO we should allow customization here
        javascript_name: *STRING_JAVASCRIPT_TYPE,
        network_protocol: std::marker::PhantomData,
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

    pub fn is_concrete(&self) -> bool {
        match self {
            GraphQLObjectDefinitionType::InputObject => true,
            GraphQLObjectDefinitionType::Union => false,
            GraphQLObjectDefinitionType::Object => true,
            GraphQLObjectDefinitionType::Interface => false,
        }
    }

    pub fn is_output_type(&self) -> bool {
        match self {
            GraphQLObjectDefinitionType::InputObject => false,
            GraphQLObjectDefinitionType::Union => true,
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

type UnvalidatedTypeRefinementMap = BTreeMap<UnvalidatedTypeName, Vec<UnvalidatedTypeName>>;

fn type_implements_node(object_type_definition: &IsographObjectTypeDefinition) -> bool {
    object_type_definition
        .interfaces
        .iter()
        .any(|x| x.item == *NODE_INTERFACE_NAME)
}
