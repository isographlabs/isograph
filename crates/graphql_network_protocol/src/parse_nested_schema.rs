use std::collections::{BTreeMap, btree_map::Entry};

use common_lang_types::{
    EntityName, Location, WithGenericLocation, WithLocationPostfix, WithNonFatalDiagnostics,
};
use isograph_lang_types::{SelectionType, SelectionTypePostfix};
use isograph_schema::{
    DataModelEntity, INT_ENTITY_NAME, IsographDatabase, NestedDataModelEntity,
    NestedDataModelSchema, STRING_ENTITY_NAME,
};
use pico::MemoRef;
use prelude::Postfix;

use crate::{
    GraphQLAndJavascriptProfile, GraphQLNetworkProtocol,
    process_type_system_definition::multiple_entity_definitions_found_diagnostic,
};

pub fn parse_nested_schema(
    db: &IsographDatabase<GraphQLAndJavascriptProfile>,
) -> MemoRef<NestedDataModelSchema<GraphQLNetworkProtocol>> {
    let mut schema = WithNonFatalDiagnostics {
        non_fatal_diagnostics: vec![],
        item: BTreeMap::new(),
    };

    define_default_graphql_data_model_entities(&mut schema);

    schema.interned_value(db)
}

fn define_default_graphql_data_model_entities(
    schema: &mut NestedDataModelSchema<GraphQLNetworkProtocol>,
) {
    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*STRING_ENTITY_NAME).with_location(None),
            selectables: Default::default(),
            network_protocol_associated_data: ().scalar_selected(),
        },
    );

    insert_into_schema_or_emit_multiple_definitions_diagnostic(
        schema,
        DataModelEntity {
            name: (*INT_ENTITY_NAME).with_location(None),
            selectables: Default::default(),
            network_protocol_associated_data: ().scalar_selected(),
        },
    );
}

fn insert_into_schema_or_emit_multiple_definitions_diagnostic(
    schema: &mut NestedDataModelSchema<GraphQLNetworkProtocol>,
    item: NestedDataModelEntity<GraphQLNetworkProtocol>,
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
