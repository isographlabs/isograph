use std::collections::{BTreeMap, btree_map::Entry};

use common_lang_types::{
    Diagnostic, EntityName, Location, SelectableName, WithLocationPostfix, WithNonFatalDiagnostics,
};
use intern::string_key::Intern;
use isograph_lang_types::{
    DefinitionLocationPostfix, SelectionTypePostfix, TypeAnnotationDeclaration,
};
use isograph_schema::{
    DataModelEntity, DataModelSelectable, EntityAssociatedData, INTEGER_ENTITY_NAME, IsConcrete,
    IsographDatabase, NestedDataModelEntity, NestedDataModelSchema, NestedDataModelSelectable,
    SelectableAssociatedData, ServerObjectSelectionInfo, TEXT_ENTITY_NAME,
};
use prelude::*;
use sqlparser::ast::{ColumnDef, CreateTable, Statement};
use tracing::debug;

use crate::{
    NUMBER_JAVASCRIPT_TYPE, SQLAndJavascriptProfile, SQLSchemaObjectAssociatedData,
    STRING_JAVASCRIPT_TYPE, parse_sql_schema,
};

pub fn parse_nested_schema(
    db: &IsographDatabase<SQLAndJavascriptProfile>,
) -> NestedDataModelSchema<SQLAndJavascriptProfile> {
    let mut schema = WithNonFatalDiagnostics {
        non_fatal_diagnostics: vec![],
        item: BTreeMap::new(),
    };

    define_default_sql_data_model_entities(&mut schema);
    insert_parsed_items_into_schema(db, &mut schema);

    schema
}

fn define_default_sql_data_model_entities(
    schema: &mut NestedDataModelSchema<SQLAndJavascriptProfile>,
) {
    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*TEXT_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*STRING_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        },
    );

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*INTEGER_ENTITY_NAME).with_missing_location(),
            description: None,
            selectables: Default::default(),
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: (*NUMBER_JAVASCRIPT_TYPE).scalar_selected(),
            }
            .server_defined(),
            selection_info: ().scalar_selected(),
        },
    );
}

fn insert_parsed_items_into_schema(
    db: &IsographDatabase<SQLAndJavascriptProfile>,
    schema: &mut NestedDataModelSchema<SQLAndJavascriptProfile>,
) {
    let type_system_document = match parse_sql_schema(db).to_owned() {
        Ok(s) => s,
        Err(e) => {
            debug!("Error parsing SQL schema: {:?}", e);
            schema.non_fatal_diagnostics.push(e);
            return;
        }
    };

    debug!("Parsed SQL schema: {:?}", type_system_document);

    process_statements(schema, type_system_document.0);
}

fn process_statements(
    schema: &mut NestedDataModelSchema<SQLAndJavascriptProfile>,
    statements: Vec<Statement>,
) {
    for statement in statements {
        match statement {
            Statement::CreateTable(create_table) => process_crate_table(schema, create_table),
            _ => { /* Ignore other statements for now */ }
        }
    }
}

fn process_crate_table(
    schema: &mut NestedDataModelSchema<SQLAndJavascriptProfile>,
    create_table: CreateTable,
) {
    let entity_name = create_table.name.to_string().intern().into();
    let selectables = process_columns(entity_name, create_table.columns);

    insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: entity_name.with_missing_location(),
            description: None,
            selectables,
            associated_data: EntityAssociatedData {
                network_protocol: (),
                target_platform: SQLSchemaObjectAssociatedData {}.object_selected(),
            }
            .server_defined(),
            selection_info: ServerObjectSelectionInfo {
                is_concrete: IsConcrete(true),
            }
            .object_selected(),
        },
    );
}

fn process_columns(
    name: EntityName,
    columns: Vec<ColumnDef>,
) -> WithNonFatalDiagnostics<
    // TODO this should be aliased
    BTreeMap<SelectableName, NestedDataModelSelectable<SQLAndJavascriptProfile>>,
> {
    let mut selectables: WithNonFatalDiagnostics<
        BTreeMap<SelectableName, NestedDataModelSelectable<SQLAndJavascriptProfile>>,
    > = WithNonFatalDiagnostics::default();

    for column in columns {
        let column_name: SelectableName = column.name.value.intern().into();
        let type_ = column.data_type.to_string().intern().to::<EntityName>();
        insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
            &mut selectables,
            DataModelSelectable {
                name: column_name.with_missing_location(),
                parent_entity_name: name.with_missing_location(),
                description: None,
                arguments: vec![],
                // TODO support errors here
                target_entity: TypeAnnotationDeclaration::Scalar(type_.into())
                    .wrap_ok()
                    .with_missing_location(),
                associated_data: SelectableAssociatedData {
                    network_protocol: (),
                    target_platform: (),
                }
                .server_defined(),
                is_inline_fragment: false.into(),
            },
        );
    }

    selectables
}

// TODO these should be one method
fn insert_entity_into_schema_or_emit_multiple_definitions_diagnostic(
    schema: &mut NestedDataModelSchema<SQLAndJavascriptProfile>,
    item: NestedDataModelEntity<SQLAndJavascriptProfile>,
) {
    let key = item.name.item;
    match schema.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
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

fn multiple_entity_definitions_found_diagnostic(
    server_object_entity_name: EntityName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!("Multiple definitions of `{server_object_entity_name}` were found."),
        location,
    )
}

fn insert_selectable_into_schema_or_emit_multiple_definitions_diagnostic(
    selectables: &mut WithNonFatalDiagnostics<
        BTreeMap<SelectableName, NestedDataModelSelectable<SQLAndJavascriptProfile>>,
    >,
    item: NestedDataModelSelectable<SQLAndJavascriptProfile>,
) {
    let key = item.name.item;
    match selectables.item.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => {
            selectables.non_fatal_diagnostics.push(
                multiple_selectable_definitions_found_diagnostic(
                    item.parent_entity_name.item,
                    key,
                    None,
                ),
            );
        }
    }
}

pub fn multiple_selectable_definitions_found_diagnostic(
    parent_object_entity_name: EntityName,
    selectable_name: SelectableName,
    location: Option<Location>,
) -> Diagnostic {
    Diagnostic::new(
        format!(
            "Multiple definitions of `{parent_object_entity_name}.{selectable_name}` were found"
        ),
        location,
    )
}
