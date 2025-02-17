use std::error::Error;

use common_lang_types::{QueryOperationName, QueryText, RelativePathToSourceFile};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_config::{AbsolutePathAndRelativePath, CompilerConfig};
use isograph_schema::{
    MergedSelectionMap, OutputFormat, ProcessTypeSystemDocumentOutcome, RootOperationName,
    ValidatedSchema, ValidatedVariableDefinition,
};

use crate::{
    process_type_system_definition::{
        process_graphql_type_extension_document, process_graphql_type_system_document,
    },
    query_text::generate_query_text,
    read_schema::{read_and_parse_graphql_schema, read_and_parse_schema_extensions},
    UnvalidatedGraphqlSchema,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphQLOutputFormat {}

impl OutputFormat for GraphQLOutputFormat {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

    type SchemaObjectAssociatedData = ();

    fn read_and_parse_type_system_document(
        config: &CompilerConfig,
    ) -> Result<Self::TypeSystemDocument, Box<dyn Error>> {
        Ok(read_and_parse_graphql_schema(config)?)
    }
    fn read_and_parse_type_system_extension_document(
        schema_extension_path: &AbsolutePathAndRelativePath,
        config: &CompilerConfig,
    ) -> Result<(RelativePathToSourceFile, Self::TypeSystemExtensionDocument), Box<dyn Error>> {
        Ok(read_and_parse_schema_extensions(
            schema_extension_path,
            config,
        )?)
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
        schema: &ValidatedSchema<Self>,
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
