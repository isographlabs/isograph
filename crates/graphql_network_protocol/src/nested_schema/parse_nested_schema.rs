use std::collections::{BTreeMap, btree_map::Entry};

use common_lang_types::{
    DescriptionValue, EntityName, JavascriptName, Location, SelectableName, VariableName,
    WithEmbeddedLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLInterfaceTypeDefinition, GraphQLTypeSystemDefinition,
    GraphQLTypeSystemExtension, GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    Description, SelectionTypePostfix, TypeAnnotationDeclaration, VariableDeclarationInner,
    VariableNameWrapper,
};
use isograph_schema::{
    BOOLEAN_ENTITY_NAME, DataModelEntity, DataModelSelectable, FLOAT_ENTITY_NAME, ID_ENTITY_NAME,
    INT_ENTITY_NAME, IsConcrete, IsographDatabase, NestedDataModelEntity, NestedDataModelSchema,
    NestedDataModelSelectable, STRING_ENTITY_NAME, ServerObjectSelectionInfo, TYPENAME_FIELD_NAME,
    multiple_selectable_definitions_found_diagnostic, to_isograph_constant_value,
};
use prelude::Postfix;

use crate::{
    BOOLEAN_JAVASCRIPT_TYPE, GraphQLAndJavascriptProfile, GraphQLSchemaObjectAssociatedData,
    NUMBER_JAVASCRIPT_TYPE, STRING_JAVASCRIPT_TYPE, UNKNOWN_JAVASCRIPT_TYPE, get_js_union_name,
    parse_graphql_schema,
    process_type_system_definition::{
        UnvalidatedTypeRefinementMap, multiple_entity_definitions_found_diagnostic,
    },
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
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
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
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*FLOAT_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*INT_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*BOOLEAN_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*BOOLEAN_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
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
        .iter()
        .map(|x| GraphQLTypeSystemExtensionOrDefinition::Definition(x.item.clone()))
        .chain(
            type_system_extension_documents
                .iter()
                .flat_map(|(_, val)| val.lookup(db).clone().0.into_iter().map(|x| x.item)),
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
            graphql_interface_type_definition.name.item,
            graphql_interface_type_definition.fields,
        );
        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
            schema,
            DataModelEntity {
                name: graphql_interface_type_definition.name.map_location(Some),
                description: graphql_interface_type_definition
                    .description
                    .map(|x| x.map_location(Some).map(Description)),
                selectables,
                network_protocol_associated_data: (),
                target_platform_associated_data: GraphQLSchemaObjectAssociatedData {
                    subtypes: supertype_to_subtype_map
                        .get(graphql_interface_type_definition.name.item.reference())
                        .expect("Expected interface to exist")
                        .clone(),
                }
                .object_selected(),
                selection_info: ServerObjectSelectionInfo {
                    is_concrete: IsConcrete(false),
                }
                .object_selected(),
            },
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
                network_protocol_associated_data: (),
                target_platform_associated_data: get_js_union_name(&concrete_child_entity_names)
                    .scalar_selected(),
                selectables: Default::default(),
            },
        );

        // TODO insert __typename field
    }
}

fn process_graphql_documents(
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
    documents: Vec<GraphQLTypeSystemExtensionOrDefinition>,
    supertype_to_subtype_map: &mut UnvalidatedTypeRefinementMap,
    interfaces_to_process: &mut Vec<GraphQLInterfaceTypeDefinition>,
) {
    for document in documents {
        match document {
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data: format!("\"{entity_name}\"")
                                    .intern()
                                    .to::<JavascriptName>()
                                    .scalar_selected(),
                                selection_info: ().scalar_selected(),
                            },
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
                                description: format!(
                                    "A typename field identifying the type {}",
                                    entity_name
                                )
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data: (),
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data:
                                    GraphQLSchemaObjectAssociatedData { subtypes: vec![] }
                                        .object_selected(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(true),
                                }
                                .object_selected(),
                            },
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data: (*UNKNOWN_JAVASCRIPT_TYPE)
                                    .scalar_selected(),
                                selection_info: ().scalar_selected(),
                            },
                        );
                    }
                    GraphQLTypeSystemDefinition::InterfaceTypeDefinition(
                        graphql_interface_type_definition,
                    ) => {
                        supertype_to_subtype_map
                            .entry(graphql_interface_type_definition.name.item)
                            .or_default();
                        interfaces_to_process.push(graphql_interface_type_definition);
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data:
                                    GraphQLSchemaObjectAssociatedData { subtypes: vec![] }
                                        .object_selected(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(true),
                                }
                                .object_selected(),
                            },
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
                                network_protocol_associated_data: (),
                                target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE)
                                    .scalar_selected(),
                                selection_info: ().scalar_selected(),
                            },
                        );
                    }
                    GraphQLTypeSystemDefinition::UnionTypeDefinition(
                        graphql_union_type_definition,
                    ) => {
                        let entity_name = graphql_union_type_definition.name.item;

                        let typename_entity_name = format!("{}__discriminator", entity_name)
                            .intern()
                            .to::<EntityName>()
                            // And make it not selectable!
                            .note_todo(
                                "Come up with a way to not have \
                                these be in the same namespace",
                            );

                        let mut selectables = BTreeMap::new();
                        selectables.insert(
                            *TYPENAME_FIELD_NAME,
                            DataModelSelectable {
                                name: (*TYPENAME_FIELD_NAME).with_missing_location(),
                                parent_entity_name: typename_entity_name.with_missing_location(),
                                description: format!(
                                    "A typename field identifying the type {}",
                                    entity_name
                                )
                                .intern()
                                .to::<DescriptionValue>()
                                .wrap(Description)
                                .with_location(None)
                                .wrap_some(),
                                arguments: vec![],
                                target_entity: TypeAnnotationDeclaration::Scalar(
                                    typename_entity_name.into(),
                                )
                                .wrap_ok()
                                .with_location(None),
                                network_protocol_associated_data: (),
                                target_platform_associated_data: (),
                                is_inline_fragment: false.into(),
                            },
                        );
                        let selectables = WithNonFatalDiagnostics::new(selectables, vec![]);

                        insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
                            schema,
                            DataModelEntity {
                                name: graphql_union_type_definition.name.map_location(Some),
                                description: graphql_union_type_definition
                                    .description
                                    .map(|x| x.map_location(Some).map(Description)),
                                selectables,
                                network_protocol_associated_data: (),
                                target_platform_associated_data:
                                    GraphQLSchemaObjectAssociatedData {
                                        subtypes: graphql_union_type_definition
                                            .union_member_types
                                            .iter()
                                            .map(|x| x.item)
                                            .collect(),
                                    }
                                    .object_selected(),
                                selection_info: ServerObjectSelectionInfo {
                                    is_concrete: IsConcrete(false),
                                }
                                .object_selected(),
                            },
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

// TODO these should be one method
fn insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
    item: NestedDataModelEntity<GraphQLAndJavascriptProfile>,
) {
    let key = item.name.item;
    match schema.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            // TODO parse graphql schema should wrap the items with locations
            vacant_entry.insert(item.with_missing_location());
        }
        Entry::Occupied(occupied_entry) => {
            schema
                .non_fatal_diagnostics
                .push(multiple_entity_definitions_found_diagnostic(
                    key,
                    occupied_entry.get().location.map(|x| x.to::<Location>()),
                ));
        }
    }
}

fn insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
    selectable: &mut WithNonFatalDiagnostics<
        BTreeMap<SelectableName, NestedDataModelSelectable<GraphQLAndJavascriptProfile>>,
    >,
    item: NestedDataModelSelectable<GraphQLAndJavascriptProfile>,
) {
    let key = item.name.item;
    match selectable.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            // TODO parse graphql schema should wrap the items with locations
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => {
            selectable.non_fatal_diagnostics.push(
                multiple_selectable_definitions_found_diagnostic(
                    item.parent_entity_name.item,
                    key,
                    // TODO proper location
                    None,
                ),
            );
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
                network_protocol_associated_data: (),
                target_platform_associated_data: (),
                is_inline_fragment: false.into(),
            },
        );
    }

    selectables
}
