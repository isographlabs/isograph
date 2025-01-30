mod combined_schema;
mod graphql_output_format;
mod process_type_system_definition;
mod query_text;
mod read_schema;

pub use graphql_output_format::*;
use isograph_schema::{
    Schema, SchemaObject, SchemaScalar, UnvalidatedSchema, ValidatedClientField, ValidatedSchema,
};

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphQLOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphQLOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphQLOutputFormat>;

pub type ValidatedGraphqlClientField = ValidatedClientField<GraphQLOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphQLOutputFormat>;
pub type GraphqlSchemaScalar = SchemaScalar<GraphQLOutputFormat>;
