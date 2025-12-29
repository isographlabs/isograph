use std::collections::BTreeMap;
use std::collections::btree_map::Entry;

use common_lang_types::{
    Diagnostic, DiagnosticResult, EntityName, QueryExtraInfo, QueryOperationName, QueryText,
    WithLocation, WithNonFatalDiagnostics,
};
use intern::string_key::Intern;
use isograph_lang_types::{SelectionType, VariableDeclaration};
use isograph_schema::IsographDatabase;
use isograph_schema::{
    Format, MergedSelectionMap, NetworkProtocol, ParseTypeSystemOutcome, RootOperationName,
    server_entity_named,
};
use pico_macros::memo;
use prelude::Postfix;

use crate::parse_type_system_document::parse_type_system_document;
use crate::process_type_system_definition::multiple_entity_definitions_found_diagnostic;
use crate::query_text::generate_query_text;

pub(crate) struct GraphQLRootTypes {
    pub query: EntityName,
    pub mutation: EntityName,
    pub subscription: EntityName,
}

impl Default for GraphQLRootTypes {
    fn default() -> Self {
        Self {
            query: "Query".intern().into(),
            mutation: "Mutation".intern().into(),
            subscription: "Subscription".intern().into(),
        }
    }
}

impl From<GraphQLRootTypes> for BTreeMap<EntityName, RootOperationName> {
    fn from(val: GraphQLRootTypes) -> Self {
        let mut map = BTreeMap::new();
        map.insert(val.query, RootOperationName("query"));
        map.insert(val.mutation, RootOperationName("mutation"));
        map.insert(val.subscription, RootOperationName("subscription"));
        map
    }
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type EntityAssociatedData = SelectionType<(), GraphQLSchemaObjectAssociatedData>;
    type SelectableAssociatedData = ();

    #[expect(clippy::type_complexity)]
    #[memo]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> DiagnosticResult<(
        WithNonFatalDiagnostics<ParseTypeSystemOutcome<Self>>,
        // fetchable types
        BTreeMap<EntityName, RootOperationName>,
    )> {
        parse_type_system_document(db)
    }

    fn generate_link_type<'a>(
        db: &IsographDatabase<Self>,
        server_object_entity_name: &EntityName,
    ) -> String {
        let server_object_entity = &server_entity_named(db, *server_object_entity_name)
            .as_ref()
            .expect(
                "Expected validation to have worked. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

        if server_object_entity
            .selection_info
            .as_object()
            .expect("Expected server object entity to be object")
            .0
        {
            let name = server_object_entity.name;
            return format!("Link<\"{name}\">");
        }

        let subtypes = server_object_entity
            .network_protocol_associated_data
            .as_ref()
            .as_object()
            .expect("Expected server object entity to have object associated data")
            .subtypes
            .iter()
            .map(|name| format!("\n  | Link<\"{name}\">"))
            .collect::<Vec<_>>();

        subtypes.join("")
    }

    fn generate_query_text<'a>(
        _db: &IsographDatabase<Self>,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a VariableDeclaration> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText {
        generate_query_text(
            query_name,
            selection_map,
            query_variables,
            root_operation_name,
            format,
        )
    }

    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: EntityName,
        indentation_level: u8,
    ) -> QueryExtraInfo {
        let indent = "  ".repeat((indentation_level + 1) as usize);
        QueryExtraInfo(format!(
            "{{\n\
            {indent}  kind: \"PersistedOperationExtraInfo\",\n\
            {indent}  operationName: \"{query_name}\",\n\
            {indent}  operationKind: \"{operation_name}\",\n\
            {indent}}}"
        ))
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub original_definition_type: GraphQLSchemaOriginalDefinitionType,
    // TODO expose this as a separate memoized method
    pub subtypes: Vec<EntityName>,
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub enum GraphQLSchemaOriginalDefinitionType {
    InputObject,
    Object,
    Interface,
    Union,
}

impl GraphQLSchemaOriginalDefinitionType {
    pub fn sdl_keyword(&self) -> &'static str {
        match self {
            GraphQLSchemaOriginalDefinitionType::InputObject => "input",
            GraphQLSchemaOriginalDefinitionType::Object => "object",
            GraphQLSchemaOriginalDefinitionType::Interface => "interface",
            GraphQLSchemaOriginalDefinitionType::Union => "union",
        }
    }
}

// TODO make this generic over value, too
pub(crate) fn insert_entity_or_multiple_definition_diagnostic<Value>(
    map: &mut BTreeMap<EntityName, WithLocation<Value>>,
    key: EntityName,
    item: WithLocation<Value>,
    non_fatal_diagnostics: &mut Vec<Diagnostic>,
) {
    match map.entry(key) {
        Entry::Vacant(vacant_entry) => {
            vacant_entry.insert(item);
        }
        Entry::Occupied(_) => non_fatal_diagnostics.push(
            multiple_entity_definitions_found_diagnostic(key, item.location.wrap_some()),
        ),
    }
}
