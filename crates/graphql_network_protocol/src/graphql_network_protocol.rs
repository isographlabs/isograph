use std::collections::BTreeMap;

use common_lang_types::{
    DirectiveName, QueryExtraInfo, QueryOperationName, QueryText, ServerObjectEntityName,
    UnvalidatedTypeName, WithLocation,
};
use graphql_lang_types::{DeserializationError, from_graphql_directive};
use graphql_schema_parser::SchemaParseError;
use intern::string_key::Intern;
use isograph_lang_types::SelectionType;
use isograph_schema::{
    ExposeFieldToInsert, Format, MergedSelectionMap, NetworkProtocol, ParseTypeSystemOutcome,
    RootOperationName, SchemaSource, ValidatedVariableDefinition, server_object_entity_named,
};
use isograph_schema::{IsographDatabase, ServerScalarEntity};
use lazy_static::lazy_static;
use pico_macros::memo;
use thiserror::Error;

use crate::{
    parse_graphql_schema,
    process_type_system_definition::{
        ProcessGraphqlTypeSystemDefinitionError, process_graphql_type_extension_document,
        process_graphql_type_system_document,
    },
    query_text::generate_query_text,
};

lazy_static! {
    static ref EXPOSE_FIELD_DIRECTIVE: DirectiveName = "exposeField".intern().into();
}

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
    type ParseTypeSystemDocumentsError = ParseGraphQLTypeSystemDocumentsError;

    #[memo]
    fn parse_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> Result<
        (
            ParseTypeSystemOutcome<Self>,
            BTreeMap<ServerObjectEntityName, RootOperationName>,
        ),
        ParseGraphQLTypeSystemDocumentsError,
    > {
        eprintln!("parse type system documents called...");
        // let mut graphql_root_types = None;

        // let (type_system_document, type_system_extension_documents) =
        parse_graphql_schema(db);

        let SchemaSource {
            content,
            text_source,
            ..
        } = db.get_schema_source();

        let v = if content.contains("FOOBARBAZ") {
            vec![]
        } else {
            vec![SelectionType::Scalar(WithLocation::new_generated(
                ServerScalarEntity {
                    description: None,
                    name: WithLocation::new_generated("foo".intern().into()),
                    javascript_name: "jsname".intern().into(),
                    network_protocol: std::marker::PhantomData,
                },
            ))]
        };

        Ok((v, {
            let mut map = BTreeMap::new();
            map.insert("foo".intern().into(), RootOperationName("query"));
            map
        }))
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
            .item;

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
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub original_definition_type: GraphQLSchemaOriginalDefinitionType,
    pub subtypes: Vec<UnvalidatedTypeName>,
}

#[derive(Debug, PartialEq, Eq, Clone)]
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

#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum ParseGraphQLTypeSystemDocumentsError {
    #[error("{}", message.for_display())]
    SchemaParse {
        message: WithLocation<SchemaParseError>,
    },

    #[error("{}", message.for_display())]
    ProcessGraphQLTypeSystemDefinitionWithLocation {
        message: WithLocation<ProcessGraphqlTypeSystemDefinitionError>,
    },

    #[error("{message}")]
    ProcessGraphQLTypeSystemDefinition {
        #[from]
        message: ProcessGraphqlTypeSystemDefinitionError,
    },

    #[error("Failed to deserialize {0}")]
    FailedToDeserialize(String),
}

impl From<WithLocation<SchemaParseError>> for ParseGraphQLTypeSystemDocumentsError {
    fn from(value: WithLocation<SchemaParseError>) -> Self {
        Self::SchemaParse { message: value }
    }
}

impl From<WithLocation<ProcessGraphqlTypeSystemDefinitionError>>
    for ParseGraphQLTypeSystemDocumentsError
{
    fn from(value: WithLocation<ProcessGraphqlTypeSystemDefinitionError>) -> Self {
        Self::ProcessGraphQLTypeSystemDefinitionWithLocation { message: value }
    }
}

fn extend_result_with_default_types(result: &mut ParseTypeSystemOutcome<GraphQLNetworkProtocol>) {
    result.push(SelectionType::Scalar(WithLocation::new_generated(
        ServerScalarEntity {
            description: None,
            name: WithLocation::new_generated("ID".intern().into()),
            javascript_name: "string".intern().into(),
            network_protocol: std::marker::PhantomData,
        },
    )));
    result.push(SelectionType::Scalar(WithLocation::new_generated(
        ServerScalarEntity {
            description: None,
            name: WithLocation::new_generated("String".intern().into()),
            javascript_name: "string".intern().into(),
            network_protocol: std::marker::PhantomData,
        },
    )));
    result.push(SelectionType::Scalar(WithLocation::new_generated(
        ServerScalarEntity {
            description: None,
            name: WithLocation::new_generated("Boolean".intern().into()),
            javascript_name: "boolean".intern().into(),
            network_protocol: std::marker::PhantomData,
        },
    )));
    result.push(SelectionType::Scalar(WithLocation::new_generated(
        ServerScalarEntity {
            description: None,
            name: WithLocation::new_generated("Float".intern().into()),
            javascript_name: "number".intern().into(),
            network_protocol: std::marker::PhantomData,
        },
    )));
    result.push(SelectionType::Scalar(WithLocation::new_generated(
        ServerScalarEntity {
            description: None,
            name: WithLocation::new_generated("Int".intern().into()),
            javascript_name: "number".intern().into(),
            network_protocol: std::marker::PhantomData,
        },
    )));
}
