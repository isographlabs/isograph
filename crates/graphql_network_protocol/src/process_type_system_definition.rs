use std::collections::{BTreeMap, HashMap};

use common_lang_types::{
    Diagnostic, DiagnosticResult, GraphQLInterfaceTypeName, Location, SelectableName,
    ServerObjectEntityName, Span, UnvalidatedTypeName, WithLocation, WithLocationPostfix, WithSpan,
    WithSpanPostfix,
};
use graphql_lang_types::{
    GraphQLConstantValue, GraphQLDirective, GraphQLNamedTypeAnnotation,
    GraphQLNonNullTypeAnnotation, GraphQLScalarTypeDefinition, GraphQLTypeAnnotation,
    GraphQLTypeSystemDefinition, GraphQLTypeSystemDocument, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionDocument, GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{Description, SelectionTypePostfix};
use isograph_schema::{
    ExposeFieldDirective, ExposeFieldToInsert, FieldMapItem, FieldToInsert, IsographDatabase,
    IsographObjectTypeDefinition, NetworkProtocol, ParseTypeSystemOutcome,
    ProcessObjectTypeDefinitionOutcome, STRING_JAVASCRIPT_TYPE, ServerObjectEntity,
    ServerScalarEntity, TYPENAME_FIELD_NAME,
};
use lazy_static::lazy_static;
use prelude::Postfix;

use crate::{
    GraphQLNetworkProtocol, GraphQLRootTypes, GraphQLSchemaObjectAssociatedData,
    GraphQLSchemaOriginalDefinitionType,
};

lazy_static! {
    // TODO use schema_data.string_type_id or something
    static ref STRING_TYPE_NAME: UnvalidatedTypeName = "String".intern().into();
    static ref NODE_INTERFACE_NAME: GraphQLInterfaceTypeName = "Node".intern().into();
    pub static ref REFETCH_FIELD_NAME: SelectableName = "__refetch".intern().into();

}

#[expect(clippy::type_complexity)]
pub fn process_graphql_type_system_document(
    db: &IsographDatabase<GraphQLNetworkProtocol>,
    type_system_document: GraphQLTypeSystemDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
) -> DiagnosticResult<(
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
                    db,
                    object_type_definition,
                    concrete_type,
                    GraphQLSchemaObjectAssociatedData {
                        original_definition_type: GraphQLSchemaOriginalDefinitionType::Object,
                        subtypes: vec![],
                        canonical_id_field_name: None,
                    },
                    GraphQLObjectDefinitionType::Object,
                    &mut refetch_fields,
                )?;

                directives
                    .entry(object_name)
                    .or_default()
                    .extend(new_directives);

                type_system_entities.push(object_definition_outcome.object_selected());
            }
            GraphQLTypeSystemDefinition::ScalarTypeDefinition(scalar_type_definition) => {
                let name_location = scalar_type_definition.name.location;
                type_system_entities.push(
                    process_scalar_definition(scalar_type_definition)
                        .with_location(name_location)
                        .scalar_selected(),
                );
                // N.B. we assume that Mutation will be an object, not a scalar
            }
            GraphQLTypeSystemDefinition::InterfaceTypeDefinition(interface_type_definition) => {
                let interface_name = interface_type_definition.name.item.unchecked_conversion();
                let (process_object_type_definition_outcome, new_directives) =
                    process_object_type_definition(
                        db,
                        interface_type_definition.into(),
                        None,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::Interface,
                            subtypes: vec![],
                            canonical_id_field_name: None,
                        },
                        GraphQLObjectDefinitionType::Interface,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(process_object_type_definition_outcome.object_selected());

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
                        db,
                        input_object_type_definition.into(),
                        // Shouldn't really matter what we pass here
                        concrete_type,
                        GraphQLSchemaObjectAssociatedData {
                            original_definition_type:
                                GraphQLSchemaOriginalDefinitionType::InputObject,
                            subtypes: vec![],
                            canonical_id_field_name: None,
                        },
                        GraphQLObjectDefinitionType::InputObject,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(process_object_type_definition_outcome.object_selected());

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
                type_system_entities.push(
                    process_scalar_definition(GraphQLScalarTypeDefinition {
                        description: enum_definition.description,
                        name: enum_definition.name.map(|x| x.unchecked_conversion()),
                        directives: enum_definition.directives,
                    })
                    .with_location(enum_definition.name.location)
                    .scalar_selected(),
                )
            }
            GraphQLTypeSystemDefinition::UnionTypeDefinition(union_definition) => {
                // TODO do something reasonable here, once we add support for type refinements.
                let (process_object_type_definition_outcome, new_directives) =
                    process_object_type_definition(
                        db,
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
                            canonical_id_field_name: None,
                        },
                        GraphQLObjectDefinitionType::Union,
                        &mut refetch_fields,
                    )?;

                type_system_entities.push(process_object_type_definition_outcome.object_selected());

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
                    return Diagnostic::new(
                        "Duplicate schema definition".to_string(),
                        location.wrap_some(),
                    )
                    .wrap_err();
                }
                *graphql_root_types = GraphQLRootTypes {
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
                }
                .wrap_some()
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
                    if x.server_object_entity.item.name == supertype_name {
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
            object_outcome.fields_to_insert.push(
                FieldToInsert {
                    description: Description(
                        format!("A client pointer for the {subtype_name} type.")
                            .intern()
                            .into(),
                    )
                    .with_generated_span()
                    .wrap_some(),
                    name: WithLocation::new_generated(format!("as{subtype_name}").intern().into()),

                    graphql_type: GraphQLTypeAnnotation::Named(GraphQLNamedTypeAnnotation(
                        (*subtype_name).with_generated_span(),
                    )),
                    javascript_type_override: None,
                    arguments: vec![],
                    is_inline_fragment: true,
                }
                .with_generated_location(),
            );
        }
    }

    (type_system_entities, directives, refetch_fields).wrap_ok()
}

#[expect(clippy::type_complexity)]
pub fn process_graphql_type_extension_document(
    db: &IsographDatabase<GraphQLNetworkProtocol>,
    extension_document: GraphQLTypeSystemExtensionDocument,
    graphql_root_types: &mut Option<GraphQLRootTypes>,
) -> DiagnosticResult<(
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
                definitions.push(definition.with_location(location));
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(extension) => {
                extensions.push(extension.with_location(location))
            }
        }
    }

    let (outcome, mut directives, refetch_fields) = process_graphql_type_system_document(
        db,
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

    (outcome, directives, refetch_fields).wrap_ok()
}

fn process_object_type_definition(
    db: &IsographDatabase<GraphQLNetworkProtocol>,
    object_type_definition: IsographObjectTypeDefinition,
    concrete_type: Option<ServerObjectEntityName>,
    associated_data: GraphQLSchemaObjectAssociatedData,
    type_definition_type: GraphQLObjectDefinitionType,
    refetch_fields: &mut Vec<ExposeFieldToInsert>,
) -> DiagnosticResult<(
    ProcessObjectTypeDefinitionOutcome<GraphQLNetworkProtocol>,
    Vec<GraphQLDirective<GraphQLConstantValue>>,
)> {
    let should_add_refetch_field = type_definition_type.is_concrete()
        && type_definition_type.is_output_type()
        && type_implements_node(&object_type_definition);
    let server_object_entity = ServerObjectEntity {
        description: object_type_definition.description.map(|d| d.item),
        name: object_type_definition.name.item,
        concrete_type,
        network_protocol_associated_data: associated_data,
    }
    .with_location(object_type_definition.name.location.into());

    let mut fields_to_insert: Vec<_> = object_type_definition
        .fields
        .into_iter()
        .map(|field_definition| {
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
            }
            .with_location(field_definition.location)
        })
        .collect();

    // We need to define a typename field for objects and interfaces, but not unions or input objects
    if type_definition_type.has_typename_field() {
        fields_to_insert.push(
            FieldToInsert {
                description: None,
                name: WithLocation::new((*TYPENAME_FIELD_NAME).into(), Location::Generated),
                graphql_type: GraphQLTypeAnnotation::NonNull(
                    GraphQLNonNullTypeAnnotation::Named(GraphQLNamedTypeAnnotation(WithSpan::new(
                        *STRING_TYPE_NAME,
                        Span::todo_generated(),
                    )))
                    .boxed(),
                ),
                // This is bad data modeling, and we should do better.
                javascript_type_override: concrete_type.map(|parent_concrete_type| {
                    format!("\"{parent_concrete_type}\"").intern().into()
                }),

                arguments: vec![],
                is_inline_fragment: false,
            }
            .with_generated_location(),
        );
    }

    if should_add_refetch_field {
        let id_field_name =
            GraphQLNetworkProtocol::get_id_field_name(db, &object_type_definition.name.item);

        refetch_fields.push(ExposeFieldToInsert {
            expose_field_directive: ExposeFieldDirective {
                expose_as: (*REFETCH_FIELD_NAME).wrap_some(),
                field_map: vec![FieldMapItem {
                    from: id_field_name.unchecked_conversion(),
                    to: id_field_name.unchecked_conversion(),
                }],
                field: format!("node.as{}", object_type_definition.name.item)
                    .intern()
                    .into(),
            },
            parent_object_name: object_type_definition.name.item,
            description: Description(
                format!(
                    "A refetch field for the {} type.",
                    object_type_definition.name.item
                )
                .intern()
                .into(),
            )
            .wrap_some(),
        });
    }

    (
        ProcessObjectTypeDefinitionOutcome {
            server_object_entity,
            fields_to_insert,
            expose_fields_to_insert: vec![],
        },
        object_type_definition.directives,
    )
        .wrap_ok()
}

// TODO this should accept an IsographScalarTypeDefinition
fn process_scalar_definition(
    scalar_type_definition: GraphQLScalarTypeDefinition,
) -> ServerScalarEntity<GraphQLNetworkProtocol> {
    ServerScalarEntity {
        description: scalar_type_definition
            .description
            .map(|with_span| with_span.item.into()),
        name: scalar_type_definition.name.item,
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
            GraphQLObjectDefinitionType::Union => true,
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
