mod query_text;

use common_lang_types::{QueryOperationName, QueryText};
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_schema::{
    MergedSelectionMap, OutputFormat, RootOperationName, Schema, SchemaObject, UnvalidatedSchema,
    ValidatedClientField, ValidatedSchema, ValidatedVariableDefinition,
};
use query_text::generate_query_text;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphqlOutputFormat {}

impl OutputFormat for GraphqlOutputFormat {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

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

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphqlOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphqlOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphqlOutputFormat>;

pub type ValidatedGraphqlClientField = ValidatedClientField<GraphqlOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphqlOutputFormat>;
