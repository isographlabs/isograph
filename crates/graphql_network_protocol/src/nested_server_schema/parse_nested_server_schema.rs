use std::collections::{BTreeMap, BTreeSet};

use common_lang_types::{
    DescriptionValue, EntityName, JavascriptName, SelectableName, VariableName,
    WithEmbeddedLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInterfaceTypeDefinition, GraphQLTypeSystemDefinition,
    GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocationPostfix, Description, EntityNameWrapper, SelectionTypePostfix,
    TypeAnnotationDeclaration, UnionTypeAnnotationDeclaration, UnionVariant,
    VariableDeclarationInner, VariableNameWrapper,
};
use isograph_schema::{
    BOOLEAN_ENTITY_NAME, DataModelEntity, DataModelSelectable, EntityAssociatedData,
    FLOAT_ENTITY_NAME, ID_ENTITY_NAME, INT_ENTITY_NAME, IsConcrete, IsographDatabase,
    NestedDataModelSchema, NestedDataModelSelectable, STRING_ENTITY_NAME, SelectableAssociatedData,
    ServerObjectSelectionInfo, TYPENAME_FIELD_NAME,
    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic,
    insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic,
    to_isograph_constant_value,
};
use prelude::Postfix;

use crate::{
    BOOLEAN_JAVASCRIPT_TYPE, GraphQLAndJavascriptProfile, GraphQLSchemaObjectAssociatedData,
    NEVER_JAVASCRIPT_TYPE, NUMBER_JAVASCRIPT_TYPE, STRING_JAVASCRIPT_TYPE, UNKNOWN_JAVASCRIPT_TYPE,
    parse_graphql_schema,
};

pub fn parse_nested_schema(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
) -> NestedDataModelSchema<GraphQLAndJavascriptProfile> {
    let mut schema = WithNonFatalDiagnostics {
        non_fatal_diagnostics: vec![],
        item: BTreeMap::new(),
    };

    define_default_graphql_data_model_entities(&mut schema);
    insert_parsed_items_into_schema(db, &mut schema);

    schema
}

fn define_default_graphql_data_model_entities(
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
) {
    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*STRING_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        }
        .with_missing_location(),
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*ID_ENTITY_NAME).with_missing_location(),
            description: "ID fields uniquely identify each item, within their type"
                .intern()
                .to::<DescriptionValue>()
                .wrap(Description)
                .with_missing_location()
                .wrap_some(),
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        }
        .with_missing_location(),
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*FLOAT_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        }
        .with_missing_location(),
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*INT_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        }
        .with_missing_location(),
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*BOOLEAN_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*BOOLEAN_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        }
        .with_missing_location(),
    );
}

fn insert_parsed_items_into_schema(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
) {
    let (type_system_document, type_system_extension_documents) =
        match parse_graphql_schema(db).to_owned() {
            Ok(s) => s,
            Err(e) => {
                schema.non_fatal_diagnostics.push(e);
                return;
            }
        };

    // TODO clone less etc
    let documents = type_system_document
        .0
        .into_iter()
        .map(|with_location| with_location.map(GraphQLTypeSystemExtensionOrDefinition::Definition))
        .chain(
            type_system_extension_documents
                .iter()
                .flat_map(|(_, val)| val.lookup(db).clone().0.into_iter()),
        )
        .collect::<Vec<_>>();

    let mut supertype_to_subtype_map = BTreeMap::new();
    let mut interfaces_to_process = vec![];

    process_graphql_documents(
        schema,
        documents,
        &mut supertype_to_subtype_map,
        &mut interfaces_to_process,
    );

    for graphql_interface_type_definition in interfaces_to_process {
        let selectables = process_fields(
            graphql_interface_type_definition.item.name.item,
            graphql_interface_type_definition.item.fields,
        );
        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
            schema,
            DataModelEntity {
                name: graphql_interface_type_definition
                    .item
                    .name
                    .map_location(Some),
                description: graphql_interface_type_definition
                    .item
                    .description
                    .map(|x| x.map_location(Some).map(Description)),
                selectables,
                associated_data: EntityAssociatedData {
                    network_protocol: (),
                    target_platform: GraphQLSchemaObjectAssociatedData {
                        subtypes: supertype_to_subtype_map
                            .get(graphql_interface_type_definition.item.name.item.reference())
                            .expect("Expected interface to exist")
                            .clone(),
                    }
                    .object_selected(),
                }
                .server_defined(),
                selection_info: ServerObjectSelectionInfo {
                    is_concrete: IsConcrete(false),
                }
                .object_selected(),
            }
            .with_some_location(graphql_interface_type_definition.location),
        );
    }

    for (abstract_parent_entity_name, concrete_child_entity_names) in supertype_to_subtype_map {
        let typename_entity_name = format!("{}__discriminator", abstract_parent_entity_name)
            .intern()
            .to::<EntityName>()
            // And make it not selectable!
            .note_todo("Come up with a way to not have these be in the same namespace");

        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
            schema,
            DataModelEntity {
                description: format!("The typename of {}", abstract_parent_entity_name)
                    .intern()
                    .to::<DescriptionValue>()
                    .wrap(Description)
                    .with_missing_location()
                    .wrap_some(),
                name: typename_entity_name.with_missing_location(),
                selection_info: ().scalar_selected(),
                associated_data: EntityAssociatedData {
                    network_protocol: (),
                    target_platform: get_js_union_name(&concrete_child_entity_names)
                        .scalar_selected(),
                }
                .server_defined(),
                selectables: Default::default(),
            }
            .with_missing_location(),
        );

        let selectables = &mut schema
            .item
            .get_mut(abstract_parent_entity_name.reference())
            .expect("Expected entity to exist")
            .item
            .selectables;
        insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
            selectables,
            DataModelSelectable {
                name: (*TYPENAME_FIELD_NAME).with_missing_location(),
                parent_entity_name: abstract_parent_entity_name.with_missing_location(),
                description: format!(
                    "A discriminant for the {} type",
                    abstract_parent_entity_name
                )
                .intern()
                .to::<DescriptionValue>()
                .wrap(Description)
                .with_missing_location()
                .wrap_some(),
                arguments: vec![],
                target_entity: TypeAnnotationDeclaration::Scalar(
                    typename_entity_name.wrap(EntityNameWrapper),
                )
                .wrap_ok()
                .with_missing_location(),
                associated_data: SelectableAssociatedData {
                    network_protocol: (),
                    target_platform: (),
                },
                is_inline_fragment: false.into(),
            },
        );

        for concrete_child_entity_name in concrete_child_entity_names {
            insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
                selectables,
                DataModelSelectable {
                    description: format!(
                        "A client pointer for the {} type.",
                        concrete_child_entity_name
                    )
                    .intern()
                    .to::<DescriptionValue>()
                    .wrap(Description)
                    .with_missing_location()
                    .wrap_some(),
                    name: format!("as{}", concrete_child_entity_name)
                        .intern()
                        .to::<SelectableName>()
                        .with_missing_location(),
                    target_entity: TypeAnnotationDeclaration::Union(
                        UnionTypeAnnotationDeclaration {
                            variants: {
                                let mut variants = BTreeSet::new();
                                variants.insert(UnionVariant::Scalar(
                                    concrete_child_entity_name.into(),
                                ));
                                variants
                            },
                            nullable: true,
                        },
                    )
                    .wrap_ok()
                    .with_missing_location(),
                    is_inline_fragment: true.into(),
                    parent_entity_name: abstract_parent_entity_name
                        .unchecked_conversion::<EntityName>()
                        .with_missing_location(),
                    arguments: vec![],
                    associated_data: SelectableAssociatedData {
                        network_protocol: (),
                        target_platform: (),
                    },
                },
            );
        }
    }
}

fn process_graphql_documents(
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
    documents: Vec<WithEmbeddedLocation<GraphQLTypeSystemExtensionOrDefinition>>,
    supertype_to_subtype_map: &mut UnvalidatedTypeRefinementMap,
    interfaces_to_process: &mut Vec<WithEmbeddedLocation<GraphQLInterfaceTypeDefinition>>,
) {
    for document in documents {
        match document.item {
            GraphQLTypeSystemExtensionOrDefinition::Definition(graphql_type_system_definition) => {
                match graphql_type_system_definition {
                    GraphQLTypeSystemDefinition::ObjectTypeDefinition(
                        graphql_object_type_definition,
                    ) => {
                        let mut selectables = process_fields(
                            graphql_object_type_definition.name.item,
                            graphql_object_type_definition.fields,
                        );
                        let entity_name = graphql_object_type_definition.name.item;

                        let typename_entity_name = format!("{}__discriminator", entity_name)
                            .intern()
                            .to::<EntityName>()
                            // And make it not selectable!
                            .note_todo(
                                "Come up with a way to not have \
                                these be in the same namespace",
                            );

                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: typename_entity_name.with_missing_location(),
                                description: format!("The typename of {}", entity_name)
                                    .intern()
                                    .to::<DescriptionValue>()
                                    .wrap(Description)
                                    .with_missing_location()
                                    .wrap_some(),
                                selectables: Default::default(),
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: format!("\"{entity_name}\"")
                                        .intern()
                                        .to::<JavascriptName>()
                                        .scalar_selected(),
                                }
                                .server_defined(),
                                selection_info: ().scalar_selected(),
                            }
                            .with_some_location(document.location),
                        );

                        selectables.item.insert(
                            *TYPENAME_FIELD_NAME,
                            DataModelSelectable {
                                name: (*TYPENAME_FIELD_NAME).with_missing_location(),
                                // Missing location because we didn't parse the parent type name as part of the
                                // selectable
                                parent_entity_name: graphql_object_type_definition
                                    .name
                                    .item
                                    .with_missing_location(),
                                description: format!("A discriminant for the {} type", entity_name)
                                    .intern()
                                    .to::<DescriptionValue>()
                                    .wrap(Description)
                                    .with_missing_location()
                                    .wrap_some(),
                                arguments: vec![],
                                target_entity: TypeAnnotationDeclaration::Scalar(
                                    typename_entity_name.into(),
                                )
                                .wrap_ok()
                                .with_missing_location(),
                                associated_data: SelectableAssociatedData {
                                    network_protocol: (),
                                    target_platform: (),
                                },
                                is_inline_fragment: false.into(),
                            },
                        );

                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_object_type_definition.name.map_location(Some),
                                description: graphql_object_type_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables,
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: GraphQLSchemaObjectAssociatedData {
                                        subtypes: vec![],
                                    }
                                    .object_selected(),
                                }
                                .server_defined(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(true),
                                }
                                .object_selected(),
                            }
                            .with_some_location(document.location),
                        );

                        for interface in graphql_object_type_definition.interfaces {
                            supertype_to_subtype_map
                                .entry(interface.item)
                                .or_default()
                                .push(graphql_object_type_definition.name.item);
                        }

                        // TODO refetch field and refetch field selection set
                    }
                    GraphQLTypeSystemDefinition::ScalarTypeDefinition(
                        graphql_scalar_type_definition,
                    ) => {
                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_scalar_type_definition.name.map_location(Some),
                                description: graphql_scalar_type_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables: Default::default(),
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: (*UNKNOWN_JAVASCRIPT_TYPE).scalar_selected(),
                                }
                                .server_defined(),
                                selection_info: ().scalar_selected(),
                            }
                            .with_some_location(document.location),
                        );
                    }
                    GraphQLTypeSystemDefinition::InterfaceTypeDefinition(
                        graphql_interface_type_definition,
                    ) => {
                        supertype_to_subtype_map
                            .entry(graphql_interface_type_definition.name.item)
                            .or_default();
                        interfaces_to_process.push(
                            graphql_interface_type_definition.with_location(document.location),
                        );
                    }
                    GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                        graphql_input_object_type_definition,
                    ) => {
                        let selectables = process_fields(
                            graphql_input_object_type_definition.name.item,
                            graphql_input_object_type_definition
                                .fields
                                .into_iter()
                                .map(|x| x.map(From::from))
                                .collect(),
                        );
                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_input_object_type_definition.name.map_location(Some),
                                description: graphql_input_object_type_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables,
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: GraphQLSchemaObjectAssociatedData {
                                        subtypes: vec![],
                                    }
                                    .object_selected(),
                                }
                                .server_defined(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(true),
                                }
                                .object_selected(),
                            }
                            .with_some_location(document.location),
                        );
                    }
                    GraphQLTypeSystemDefinition::DirectiveDefinition(
                        _graphql_directive_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::EnumDefinition(graphql_enum_definition) => {
                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_enum_definition.name.map_location(Some),
                                description: graphql_enum_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables: Default::default(),
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
                                }
                                .server_defined(),
                                selection_info: ().scalar_selected(),
                            }
                            .with_some_location(document.location),
                        );
                    }
                    GraphQLTypeSystemDefinition::UnionTypeDefinition(
                        graphql_union_type_definition,
                    ) => {
                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_union_type_definition.name.map_location(Some),
                                description: graphql_union_type_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables: Default::default(),
                                associated_data: EntityAssociatedData {
                                    network_protocol: (),
                                    target_platform: GraphQLSchemaObjectAssociatedData {
                                        subtypes: graphql_union_type_definition
                                            .union_member_types
                                            .iter()
                                            .map(|x| x.item)
                                            .collect(),
                                    }
                                    .object_selected(),
                                }
                                .server_defined(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(false),
                                }
                                .object_selected(),
                            }
                            .with_some_location(document.location),
                        );

                        *supertype_to_subtype_map
                            .entry(graphql_union_type_definition.name.item)
                            .or_default() = graphql_union_type_definition
                            .union_member_types
                            .into_iter()
                            .map(|x| x.item)
                            .collect();
                    }
                    GraphQLTypeSystemDefinition::SchemaDefinition(_graphql_schema_definition) => {
                        // TODO schema
                    }
                }
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(graphql_type_system_extension) => {
                match graphql_type_system_extension {
                    GraphQLTypeSystemExtension::ObjectTypeExtension(
                        _graphql_object_type_extension,
                    ) => {
                        // TODO extensions
                    }
                }
            }
        }
    }
}

fn process_fields(
    parent_entity_name: EntityName,
    // TODO accept iterator and don't materialize when processing input items
    fields: Vec<WithEmbeddedLocation<GraphQLFieldDefinition>>,
) -> WithNonFatalDiagnostics<
    // TODO this should be aliased
    BTreeMap<SelectableName, NestedDataModelSelectable<GraphQLAndJavascriptProfile>>,
> {
    let mut selectables: WithNonFatalDiagnostics<
        BTreeMap<SelectableName, NestedDataModelSelectable<GraphQLAndJavascriptProfile>>,
    > = WithNonFatalDiagnostics::default();

    for field in fields {
        let field = field.item;
        insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
            &mut selectables,
            DataModelSelectable {
                name: field.name.map_location(Some),
                parent_entity_name: parent_entity_name.with_missing_location(),
                description: field
                    .description
                    .map(|x| x.map_location(Some).map(Description)),
                arguments: field
                    .arguments
                    .into_iter()
                    .map(|argument| VariableDeclarationInner {
                        name: argument
                            .item
                            .name
                            .map(|x| x.to::<VariableName>())
                            .map(VariableNameWrapper),
                        type_: argument
                            .item
                            .type_
                            .map(TypeAnnotationDeclaration::from_graphql_type_annotation),
                        default_value: argument
                            .item
                            .default_value
                            .map(|x| x.map(to_isograph_constant_value)),
                    })
                    .collect(),
                // TODO support errors here
                target_entity: field
                    .type_
                    .map_location(Some)
                    .map(TypeAnnotationDeclaration::from_graphql_type_annotation)
                    .map(Ok),
                associated_data: SelectableAssociatedData {
                    network_protocol: (),
                    target_platform: (),
                },
                is_inline_fragment: false.into(),
            },
        );
    }

    selectables
}

fn get_js_union_name(members: &[EntityName]) -> JavascriptName {
    if members.is_empty() {
        *NEVER_JAVASCRIPT_TYPE
    } else {
        members
            .iter()
            .map(|name| format!("\"{name}\""))
            .collect::<Vec<String>>()
            .join(" | ")
            .intern()
            .into()
    }
}

type UnvalidatedTypeRefinementMap = BTreeMap<EntityName, Vec<EntityName>>;
