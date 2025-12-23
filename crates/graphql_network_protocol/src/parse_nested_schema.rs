use std::collections::{BTreeMap, btree_map::Entry};

use common_lang_types::{
    DescriptionValue, Location, SelectableName, VariableName, WithEmbeddedLocation,
    WithGenericLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use graphql_lang_types::{
    GraphQLFieldDefinition, GraphQLTypeSystemDefinition, GraphQLTypeSystemExtension,
    GraphQLTypeSystemExtensionOrDefinition,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    Description, SelectionTypePostfix, TypeAnnotationDeclaration, VariableDeclarationInner,
    VariableNameWrapper,
};
use isograph_schema::{
    BOOLEAN_ENTITY_NAME, DataModelEntity, DataModelSelectable, FLOAT_ENTITY_NAME, ID_ENTITY_NAME,
    INT_ENTITY_NAME, IsConcrete, IsographDatabase, NestedDataModelEntity, NestedDataModelSchema,
    NestedDataModelSelectable, STRING_ENTITY_NAME, to_isograph_constant_value,
};
use pico::MemoRef;
use prelude::Postfix;

use crate::{
    BOOLEAN_JAVASCRIPT_TYPE, GraphQLAndJavascriptProfile, GraphQLSchemaObjectAssociatedData,
    NUMBER_JAVASCRIPT_TYPE, STRING_JAVASCRIPT_TYPE, parse_graphql_schema,
    process_type_system_definition::multiple_entity_definitions_found_diagnostic,
};

#[expect(unused)]
pub fn parse_nested_schema(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
) -> MemoRef<NestedDataModelSchema<GraphQLAndJavascriptProfile>> {
    let mut schema = WithNonFatalDiagnostics {
        non_fatal_diagnostics: vec![],
        item: BTreeMap::new(),
    };

    define_default_graphql_data_model_entities(db, &mut schema);

    schema.interned_value(db)
}

fn define_default_graphql_data_model_entities(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
) {
    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*STRING_ENTITY_NAME).with_location(None),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*ID_ENTITY_NAME).with_location(None),
            description: "ID fields uniquely identify each item, within their type"
                .intern()
                .to::<DescriptionValue>()
                .wrap(Description)
                .with_location(None)
                .wrap_some(),
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*FLOAT_ENTITY_NAME).with_location(None),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*INT_ENTITY_NAME).with_location(None),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*BOOLEAN_ENTITY_NAME).with_location(None),
            description: None,
            selectables: Default::default(),
            network_protocol_associated_data: (),
            target_platform_associated_data: (*BOOLEAN_JAVASCRIPT_TYPE).scalar_selected(),
            selection_info: ().scalar_selected(),
        },
    );

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

    process_graphql_documents(db, schema, documents);
}

fn process_graphql_documents(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
    documents: Vec<GraphQLTypeSystemExtensionOrDefinition>,
) {
    for document in documents {
        match document {
            GraphQLTypeSystemExtensionOrDefinition::Definition(graphql_type_system_definition) => {
                match graphql_type_system_definition {
                    GraphQLTypeSystemDefinition::ObjectTypeDefinition(
                        graphql_object_type_definition,
                    ) => {
                        let selectables = process_fields(graphql_object_type_definition.fields);
                        insert_into_schema_or_emit_multiple_definitions_diagnostic(
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
                                selection_info: IsConcrete(true).object_selected(),
                            },
                        );
                    }
                    GraphQLTypeSystemDefinition::ScalarTypeDefinition(
                        graphql_scalar_type_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::InterfaceTypeDefinition(
                        graphql_interface_type_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::InputObjectTypeDefinition(
                        graphql_input_object_type_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::DirectiveDefinition(
                        graphql_directive_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::EnumDefinition(graphql_enum_definition) => {}
                    GraphQLTypeSystemDefinition::UnionTypeDefinition(
                        graphql_union_type_definition,
                    ) => {}
                    GraphQLTypeSystemDefinition::SchemaDefinition(graphql_schema_definition) => {}
                }
            }
            GraphQLTypeSystemExtensionOrDefinition::Extension(graphql_type_system_extension) => {
                match graphql_type_system_extension {
                    GraphQLTypeSystemExtension::ObjectTypeExtension(
                        graphql_object_type_extension,
                    ) => {}
                }
            }
        }
    }
}

fn insert_into_schema_or_emit_multiple_definitions_diagnostic(
    schema: &mut NestedDataModelSchema<GraphQLAndJavascriptProfile>,
    item: NestedDataModelEntity<GraphQLAndJavascriptProfile>,
) {
    let key = item.name.item;
    match schema.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(WithGenericLocation::new(item, None));
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

fn process_fields(
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
        selectables.item.insert(
            field.name.item,
            DataModelSelectable {
                name: field.name.map_location(Some),
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
                // TODO we don't know whether it's an object or scalar yet! This needs to go on the entity
                target_platform_associated_data: (),
                is_inline_fragment: false.into(),
            },
        );
    }

    selectables
}
