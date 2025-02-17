mod process_type_system_definition;
mod query_text;
mod read_schema;

use common_lang_types::{QueryOperationName, QueryText, RelativePathToSourceFile};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_config::{AbsolutePathAndRelativePath, CompilerConfig};
use isograph_schema::{
    MergedSelectionMap, OutputFormat, RootOperationName, Schema, SchemaObject, UnvalidatedSchema,
    ValidatedClientField, ValidatedSchema, ValidatedVariableDefinition,
};
use process_type_system_definition::{
    process_graphql_type_extension_document, process_graphql_type_system_document,
};
use query_text::generate_query_text;
use read_schema::{read_and_parse_graphql_schema, read_and_parse_schema_extensions};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphQLOutputFormat {}

impl OutputFormat for GraphQLOutputFormat {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

    fn read_and_parse_type_system_document(
        config: &CompilerConfig,
    ) -> Result<Self::TypeSystemDocument, Box<dyn std::error::Error>> {
        Ok(read_and_parse_graphql_schema(config)?)
    }
    fn read_and_parse_type_system_extension_document(
        schema_extension_path: &AbsolutePathAndRelativePath,
        config: &CompilerConfig,
    ) -> Result<
        (RelativePathToSourceFile, Self::TypeSystemExtensionDocument),
        Box<dyn std::error::Error>,
    > {
        Ok(read_and_parse_schema_extensions(
            schema_extension_path,
            config,
        )?)
    }

    fn process_type_system_document(
        schema: &mut UnvalidatedSchema<Self>,
        type_system_document: Self::TypeSystemDocument,
        options: &isograph_config::CompilerConfigOptions,
    ) -> Result<isograph_schema::ProcessTypeSystemDocumentOutcome, Box<dyn std::error::Error>> {
        Ok(process_graphql_type_system_document(
            schema,
            type_system_document,
            options,
        )?)
    }
    fn process_type_system_extension_document(
        schema: &mut UnvalidatedSchema<Self>,
        type_system_extension_document: Self::TypeSystemExtensionDocument,
        options: &isograph_config::CompilerConfigOptions,
    ) -> Result<isograph_schema::ProcessTypeSystemDocumentOutcome, Box<dyn std::error::Error>> {
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

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphQLOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphQLOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphQLOutputFormat>;

pub type ValidatedGraphqlClientField = ValidatedClientField<GraphQLOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphQLOutputFormat>;
