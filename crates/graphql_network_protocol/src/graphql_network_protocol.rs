use std::{collections::BTreeMap, error::Error};

use common_lang_types::{
    Location, QueryOperationName, QueryText, RelativePathToSourceFile, WithLocation,
};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_lang_types::SchemaSource;
use isograph_schema::{
    MergedSelectionMap, NetworkProtocol, ProcessTypeSystemDocumentOutcome, RootOperationName,
    Schema, ValidatedVariableDefinition,
};
use pico::{Database, SourceId};

use crate::{
    parse_graphql_schema,
    process_type_system_definition::{
        process_graphql_type_extension_document, process_graphql_type_system_document,
        ProcessGraphqlTypeSystemDefinitionError,
    },
    query_text::generate_query_text,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

    type SchemaObjectAssociatedData = GraphQLSchemaObjectAssociatedData;

    fn parse_and_process_type_system_documents(
        db: &Database,
        schema_source_id: SourceId<SchemaSource>,
        schema_extension_sources: &BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
    ) -> Result<ProcessTypeSystemDocumentOutcome<GraphQLNetworkProtocol>, Box<dyn Error>> {
        let (type_system_document, type_system_extension_documents) =
            parse_graphql_schema(db, schema_source_id, schema_extension_sources).to_owned()?;

        let mut result = process_graphql_type_system_document(type_system_document.to_owned())?;

        for (_, type_system_extension_document) in type_system_extension_documents {
            let (outcome, objects_and_directives) =
                process_graphql_type_extension_document(type_system_extension_document.to_owned())?;

            let ProcessTypeSystemDocumentOutcome {
                scalars,
                objects,
                unvalidated_supertype_to_subtype_map: unvalidated_subtype_to_supertype_map,
            } = outcome;

            // Note: we process all newly-defined types in schema extensions.
            // However, we ignore a bunch of things, like newly-defined fields on existing types, etc.
            // We should probably fix that!
            result.objects.extend(objects);
            result.scalars.extend(scalars);
            result
                .unvalidated_supertype_to_subtype_map
                .extend(unvalidated_subtype_to_supertype_map);

            // - in the extension document, you may have added directives to objects, e.g. @exposeAs
            // - we need to transfer those to the original objects.
            //
            // The way we are doing this is in dire need of cleanup.
            for (name, directives) in objects_and_directives {
                let object = result
                    .objects
                    .iter_mut()
                    .find(|(result, _)| result.server_object_entity.name == name)
                    .ok_or_else(|| {
                        WithLocation::new(
                        ProcessGraphqlTypeSystemDefinitionError::AttemptedToExtendUndefinedType {
                            type_name: name
                        },
                        Location::generated()
                    )
                    })?;

                object.0.directives.extend(directives);
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
    ) -> QueryText {
        generate_query_text(
            query_name,
            schema,
            selection_map,
            query_variables,
            root_operation_name,
        )
    }
}

#[derive(Debug)]
pub struct GraphQLSchemaObjectAssociatedData {
    pub original_definition_type: GraphQLSchemaOriginalDefinitionType,
}

#[derive(Debug)]
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
