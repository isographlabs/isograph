use common_lang_types::{
    DirectiveName, QueryExtraInfo, QueryOperationName, QueryText, ServerObjectEntityName,
    WithLocation,
};
use graphql_lang_types::{from_graphql_directive, DeserializationError};
use graphql_schema_parser::SchemaParseError;
use intern::string_key::Intern;
use isograph_compiler::get_standard_sources;
use isograph_lang_types::IsographDatabase;
use isograph_schema::{
    CreateAdditionalFieldsError, ExposeAsFieldToInsert, Format, MergedSelectionMap,
    NetworkProtocol, ProcessTypeSystemDocumentOutcome, RootOperationName, Schema, StandardSources,
    ValidatedVariableDefinition,
};
use lazy_static::lazy_static;
use thiserror::Error;

use crate::{
    parse_graphql_schema,
    process_type_system_definition::{
        process_graphql_type_extension_document, process_graphql_type_system_document,
        ProcessGraphqlTypeSystemDefinitionError, QUERY_TYPE,
    },
    query_text::generate_query_text,
};

lazy_static! {
    static ref EXPOSE_FIELD_DIRECTIVE: DirectiveName = "exposeField".intern().into();
}

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type SchemaObjectAssociatedData = GraphQLSchemaObjectAssociatedData;
    type ParseAndProcessTypeSystemDocumentsError = ParseAndProcessGraphQLTypeSystemDocumentsError;

    fn parse_and_process_type_system_documents(
        db: &IsographDatabase,
    ) -> Result<
        ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>,
        ParseAndProcessGraphQLTypeSystemDocumentsError,
    > {
        let StandardSources {
            schema_source_id,
            schema_extension_sources,
        } = get_standard_sources(db);

        let (type_system_document, type_system_extension_documents) =
            parse_graphql_schema(db, *schema_source_id, schema_extension_sources).to_owned()?;

        let (mut result, mut directives, mut refetch_fields) =
            process_graphql_type_system_document(type_system_document.to_owned())?;

        for type_system_extension_document in type_system_extension_documents.values() {
            let (outcome, objects_and_directives, new_refetch_fields) =
                process_graphql_type_extension_document(type_system_extension_document.to_owned())?;

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

        let query = result
            .objects
            .iter_mut()
            .find(|(object, _)| object.server_object_entity.name == *QUERY_TYPE)
            .expect("Expected query type to be defined. Renaming the query is not yet supported.");
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
                .find(|(result, _)| result.server_object_entity.name == name)
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
                                    parent_object_name: object.server_object_entity.name,
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

        Ok(result)
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
