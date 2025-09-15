use std::collections::BTreeMap;

use common_lang_types::{
    DirectiveName, QueryExtraInfo, QueryOperationName, QueryText, ServerObjectEntityName,
    UnvalidatedTypeName, WithLocation,
};
use graphql_lang_types::{DeserializationError, from_graphql_directive};
use graphql_schema_parser::SchemaParseError;
use intern::string_key::Intern;
use isograph_schema::IsographDatabase;
use isograph_schema::{
    CreateAdditionalFieldsError, ExposeAsFieldToInsert, Format, MergedSelectionMap,
    NetworkProtocol, ProcessTypeSystemDocumentOutcome, RootOperationName, Schema,
    ValidatedVariableDefinition,
};
use lazy_static::lazy_static;
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
    type ParseAndProcessTypeSystemDocumentsError = ParseAndProcessGraphQLTypeSystemDocumentsError;

    fn parse_and_process_type_system_documents(
        db: &IsographDatabase<Self>,
    ) -> Result<
        (
            ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>,
            BTreeMap<ServerObjectEntityName, RootOperationName>,
        ),
        ParseAndProcessGraphQLTypeSystemDocumentsError,
    > {
        let mut graphql_root_types = None;

        let (type_system_document, type_system_extension_documents) =
            parse_graphql_schema(db).to_owned()?;

        let (mut result, mut directives, mut refetch_fields) =
            process_graphql_type_system_document(
                type_system_document.to_owned(),
                &mut graphql_root_types,
            )?;

        for type_system_extension_document in type_system_extension_documents.values() {
            let (outcome, objects_and_directives, new_refetch_fields) =
                process_graphql_type_extension_document(
                    type_system_extension_document.to_owned(),
                    &mut graphql_root_types,
                )?;

            for (name, new_directives) in objects_and_directives {
                directives.entry(name).or_default().extend(new_directives);
            }

            let ProcessTypeSystemDocumentOutcome { scalars, objects } = outcome;

            // Note: we process all newly-defined types in schema extensions.
            // However, we ignore a bunch of things, like newly-defined fields on existing types, etc.
            // We should probably fix that!
            result.objects.extend(objects);
            result.scalars.extend(scalars);
            refetch_fields.extend(new_refetch_fields);
        }

        let graphql_root_types = graphql_root_types.unwrap_or_default();

        let query = result
            .objects
            .iter_mut()
            .find(|(object, _)| object.server_object_entity.name.item == graphql_root_types.query)
            .expect("Expected query type to be found.");
        query.0.expose_as_fields_to_insert.extend(refetch_fields);

        // - in the extension document, you may have added directives to objects, e.g. @exposeAs
        // - we need to transfer those to the original objects.
        //
        // The way we are doing this is in dire need of cleanup.
        for (name, directives) in directives {
            // TODO don't do O(n^2) here
            match result
                .objects
                .iter_mut()
                .find(|(result, _)| result.server_object_entity.name.item == name)
            {
                Some((object, _)) => {
                    for directive in directives {
                        if directive.name.item == *EXPOSE_FIELD_DIRECTIVE {
                            let expose_field_directive = from_graphql_directive(&directive)
                                .map_err(|err| match err {
                                    DeserializationError::Custom(err) => WithLocation::new(
                                        CreateAdditionalFieldsError::FailedToDeserialize(err),
                                        directive.name.location.into(), // TODO: use location of the entire directive
                                    ),
                                })?;

                            object
                                .expose_as_fields_to_insert
                                .push(ExposeAsFieldToInsert {
                                    expose_field_directive,
                                    parent_object_name: object.server_object_entity.name.item,
                                    description: None,
                                });
                        }
                    }
                }
                None => {
                    return Err(
                        ProcessGraphqlTypeSystemDefinitionError::AttemptedToExtendUndefinedType {
                            type_name: name,
                        },
                    )?;
                }
            }
        }

        Ok((result, graphql_root_types.into()))
    }

    fn generate_link_type<'a>(
        schema: &Schema<Self>,
        server_object_entity: &ServerObjectEntityName,
    ) -> String {
        let server_object_entity = schema
            .server_entity_data
            .server_object_entity(*server_object_entity)
            .expect(
                "Expected entity to exist. \
                This is indicative of a bug in Isograph.",
            );

        if let Some(concrete_type) = server_object_entity.concrete_type {
            return format!("Link<\"{concrete_type}\">");
        }

        let subtypes = server_object_entity
            .network_protocol_associated_data
            .subtypes
            .iter()
            .map(|name| format!("Link<\"{name}\">"))
            .collect::<Vec<_>>();

        subtypes.join(" | ")
    }

    fn generate_query_text<'a>(
        query_name: QueryOperationName,
        schema: &Schema<Self>,
        selection_map: &MergedSelectionMap,
        query_variables: impl Iterator<Item = &'a ValidatedVariableDefinition> + 'a,
        root_operation_name: &RootOperationName,
        format: Format,
    ) -> QueryText {
        generate_query_text(
            query_name,
            schema,
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
pub enum ParseAndProcessGraphQLTypeSystemDocumentsError {
    #[error("{message}")]
    SchemaParse {
        #[from]
        message: WithLocation<SchemaParseError>,
    },

    #[error("{message}")]
    ProcessGraphQLTypeSystemDefinitionWithLocation {
        #[from]
        message: WithLocation<ProcessGraphqlTypeSystemDefinitionError>,
    },

    #[error("{message}")]
    ProcessGraphQLTypeSystemDefinition {
        #[from]
        message: ProcessGraphqlTypeSystemDefinitionError,
    },

    #[error("{message}")]
    CreateAdditionalFields {
        #[from]
        message: WithLocation<CreateAdditionalFieldsError>,
    },
}
