use std::collections::BTreeMap;

use common_lang_types::{
    Diagnostic, DiagnosticResult, Location, QueryExtraInfo, QueryOperationName, QueryText,
    ServerObjectEntityName, ServerScalarSelectableName, UnvalidatedTypeName, WithLocationPostfix,
};
use graphql_lang_types::from_graphql_directives;
use intern::string_key::Intern;
use isograph_lang_types::SelectionTypePostfix;
use isograph_schema::{
    ExposeFieldToInsert, Format, MergedSelectionMap, NetworkProtocol, ParseTypeSystemOutcome,
    RootOperationName, ServerObjectEntityDirectives, ValidatedVariableDefinition,
    server_object_entity_named,
};
use isograph_schema::{IsographDatabase, ServerScalarEntity};
use pico_macros::memo;
use prelude::Postfix;

use crate::{
    parse_graphql_schema,
    process_type_system_definition::{
        process_graphql_type_extension_document, process_graphql_type_system_document,
    },
    query_text::generate_query_text,
};

pub(crate) struct GraphQLRootTypes {
    pub query: ServerObjectEntityName,
    pub mutation: ServerObjectEntityName,
    pub subscription: ServerObjectEntityName,
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

impl From<GraphQLRootTypes> for BTreeMap<ServerObjectEntityName, RootOperationName> {
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
    type SchemaObjectAssociatedData = GraphQLSchemaObjectAssociatedData;

    #[memo]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> DiagnosticResult<(
        ParseTypeSystemOutcome<Self>,
        BTreeMap<ServerObjectEntityName, RootOperationName>,
    )> {
        let mut graphql_root_types = None;

        let (type_system_document, type_system_extension_documents) = parse_graphql_schema(db)
            .to_owned()
            .note_todo("Do not clone. Use a MemoRef.")?;

        let (mut result, mut directives, mut refetch_fields) =
            process_graphql_type_system_document(
                type_system_document
                    .to_owned(db)
                    .note_todo("Do not clone. Use a MemoRef."),
                &mut graphql_root_types,
            )?;

        for type_system_extension_document in type_system_extension_documents.values() {
            let (outcome, objects_and_directives, new_refetch_fields) =
                process_graphql_type_extension_document(
                    type_system_extension_document
                        .to_owned(db)
                        .note_todo("Do not clone. Use a MemoRef."),
                    &mut graphql_root_types,
                )?;

            for (name, new_directives) in objects_and_directives {
                directives.entry(name).or_default().extend(new_directives);
            }

            // Note: we process all newly-defined types in schema extensions.
            // However, we ignore a bunch of things, like newly-defined fields on existing types, etc.
            // We should probably fix that!
            result.extend(outcome);
            refetch_fields.extend(new_refetch_fields);
        }

        let graphql_root_types = graphql_root_types.unwrap_or_default();

        let query = result
            .iter_mut()
            .find_map(|item| match item.as_ref_mut().as_object() {
                Some(outcome) => {
                    if outcome.server_object_entity.item.name == graphql_root_types.query {
                        Some(outcome)
                    } else {
                        None
                    }
                }
                None => None,
            })
            .expect("Expected query type to be found.");
        query.expose_fields_to_insert.extend(refetch_fields);

        // - in the extension document, you may have added directives to objects, e.g. @expose
        // - we need to transfer those to the original objects.
        //
        // The way we are doing this is in dire need of cleanup.
        for (server_object_entity_name, directives) in directives {
            // TODO don't do O(n^2) here
            match result
                .iter_mut()
                .find_map(|item| match item.as_ref_mut().as_object() {
                    Some(x) => {
                        if x.server_object_entity.item.name == server_object_entity_name {
                            Some(x)
                        } else {
                            None
                        }
                    }
                    None => None,
                }) {
                Some(outcome) => {
                    let server_object_entity_directives: ServerObjectEntityDirectives =
                        from_graphql_directives(&directives).map_err(|err| {
                            Diagnostic::new(
                                format!("Failed to deserialize: {}", err),
                                Location::Generated.wrap_some(),
                            )
                        })?;

                    // comment out this block for development
                    if server_object_entity_directives.canonical_id.is_some() {
                        panic!(
                            "Canonical ID directive is not supported yet. It's under development."
                        );
                    }

                    for expose_field_directive in
                        server_object_entity_directives.expose_field.clone()
                    {
                        outcome.expose_fields_to_insert.push(ExposeFieldToInsert {
                            expose_field_directive,
                            parent_object_name: outcome.server_object_entity.item.name,
                            description: None,
                        });
                    }

                    outcome
                        .server_object_entity
                        .item
                        .server_object_entity_directives = server_object_entity_directives;
                }
                None => {
                    return Diagnostic::new(
                        format!("Attempted to extend {server_object_entity_name}, but that type is not defined"),
                        // TODO we should have a location here
                        None,
                    )
                    .wrap_err()?;
                }
            }
        }

        extend_result_with_default_types(&mut result);

        (result, graphql_root_types.into()).wrap_ok()
    }

    fn generate_link_type<'a>(
        db: &IsographDatabase<Self>,
        server_object_entity_name: &ServerObjectEntityName,
    ) -> String {
        let server_object_entity = &server_object_entity_named(db, *server_object_entity_name)
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

        if let Some(concrete_type) = server_object_entity.concrete_type {
            return format!("Link<\"{concrete_type}\">");
        }

        let subtypes = server_object_entity
            .network_protocol_associated_data
            .subtypes
            .iter()
            .map(|name| format!("\n  | Link<\"{name}\">"))
            .collect::<Vec<_>>();

        subtypes.join("")
    }

    fn generate_query_text<'a>(
        db: &IsographDatabase<Self>,
        query_name: QueryOperationName,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText {
        generate_query_text(
            db,
            query_name,
            selection_map,
            query_variables,
            root_operation_name,
            format,
        )
    }

    fn generate_query_extra_info(
        query_name: QueryOperationName,
        operation_name: ServerObjectEntityName,
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

    fn get_id_field_name(
        db: &IsographDatabase<GraphQLNetworkProtocol>,
        server_object_entity_name: &ServerObjectEntityName,
    ) -> ServerScalarSelectableName {
        let entity = server_object_entity_named(db, *server_object_entity_name)
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .as_ref()
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            )
            .lookup(db);

        entity.canonical_id_field_name()
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub original_definition_type: GraphQLSchemaOriginalDefinitionType,
    pub subtypes: Vec<UnvalidatedTypeName>,
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

fn extend_result_with_default_types(result: &mut ParseTypeSystemOutcome<GraphQLNetworkProtocol>) {
    result.push(
        ServerScalarEntity {
            description: None,
            name: "ID".intern().into(),
            javascript_name: "string".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .with_generated_location()
        .scalar_selected(),
    );
    result.push(
        ServerScalarEntity {
            description: None,
            name: "String".intern().into(),
            javascript_name: "string".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .with_generated_location()
        .scalar_selected(),
    );
    result.push(
        ServerScalarEntity {
            description: None,
            name: "Boolean".intern().into(),
            javascript_name: "boolean".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .with_generated_location()
        .scalar_selected(),
    );
    result.push(
        ServerScalarEntity {
            description: None,
            name: "Float".intern().into(),
            javascript_name: "number".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .with_generated_location()
        .scalar_selected(),
    );
    result.push(
        ServerScalarEntity {
            description: None,
            name: "Int".intern().into(),
            javascript_name: "number".intern().into(),
            network_protocol: std::marker::PhantomData,
        }
        .with_generated_location()
        .scalar_selected(),
    );
}
