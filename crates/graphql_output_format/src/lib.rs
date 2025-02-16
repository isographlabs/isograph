mod artifact_generation;

use artifact_generation::generate_artifacts::get_artifact_path_and_content;
use common_lang_types::ArtifactPathAndContent;
use graphql_lang_types::{GraphQLTypeSystemDocument, GraphQLTypeSystemExtensionDocument};
use isograph_config::CompilerConfig;
use isograph_schema::{
    OutputFormat, Schema, SchemaObject, UnvalidatedSchema, ValidatedClientField, ValidatedSchema,
};

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Ord, Eq, std::hash::Hash, Default)]
pub struct GraphqlOutputFormat {}

impl OutputFormat for GraphqlOutputFormat {
    type TypeSystemDocument = GraphQLTypeSystemDocument;
    type TypeSystemExtensionDocument = GraphQLTypeSystemExtensionDocument;

    fn generate_artifact_path_and_content(
        schema: &ValidatedGraphqlSchema,
        config: &CompilerConfig,
    ) -> Vec<ArtifactPathAndContent> {
        get_artifact_path_and_content(schema, config)
    }
}

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphqlOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphqlOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphqlOutputFormat>;

pub type ValidatedGraphqlClientField = ValidatedClientField<GraphqlOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphqlOutputFormat>;
