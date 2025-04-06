use std::{collections::BTreeMap, error::Error};

use common_lang_types::{QueryOperationName, QueryText, RelativePathToSourceFile};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_lang_types::SchemaSource;
use isograph_schema::{
    MergedSelectionMap, NetworkProtocol, ProcessTypeSystemDocumentOutcome, RootOperationName,
    Schema, ValidatedVariableDefinition,
};
use pico::{Database, MemoRef, SourceId};

use crate::{
    parse_graphql_schema_and_extensions,
    process_type_system_definition::{
        process_graphql_type_extension_document, process_graphql_type_system_document,
    },
    query_text::generate_query_text,
    UnvalidatedGraphqlSchema,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphQLNetworkProtocol {}

impl NetworkProtocol for GraphQLNetworkProtocol {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

    type SchemaObjectAssociatedData = GraphQLSchemaObjectAssociatedData;

    fn parse_type_system_documents(
        db: &Database,
        schema_source_id: SourceId<SchemaSource>,
        schema_extension_sources: &BTreeMap<RelativePathToSourceFile, SourceId<SchemaSource>>,
    ) -> Result<
        (
            MemoRef<Self::TypeSystemDocument>,
            BTreeMap<RelativePathToSourceFile, MemoRef<Self::TypeSystemExtensionDocument>>,
        ),
        Box<dyn Error>,
    > {
        Ok(
            parse_graphql_schema_and_extensions(db, schema_source_id, schema_extension_sources)
                .to_owned()?,
        )
    }

    fn process_type_system_document(
        schema: &mut UnvalidatedGraphqlSchema,
        type_system_document: Self::TypeSystemDocument,
        options: &isograph_config::CompilerConfigOptions,
    ) -> Result<ProcessTypeSystemDocumentOutcome, Box<dyn Error>> {
        Ok(process_graphql_type_system_document(
            schema,
            type_system_document,
            options,
        )?)
    }
    fn process_type_system_extension_document(
        schema: &mut UnvalidatedGraphqlSchema,
        type_system_extension_document: Self::TypeSystemExtensionDocument,
        options: &isograph_config::CompilerConfigOptions,
    ) -> Result<ProcessTypeSystemDocumentOutcome, Box<dyn Error>> {
        Ok(process_graphql_type_extension_document(
            schema,
            type_system_extension_document,
            options,
        )?)
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
