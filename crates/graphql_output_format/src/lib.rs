mod graphql_output_format;
mod process_type_system_definition;
mod query_text;
mod read_schema;

pub use graphql_output_format::*;
use isograph_schema::{
    Schema, SchemaObject, UnvalidatedSchema, ValidatedClientField, ValidatedSchema,
};
pub use read_schema::*;

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphQLOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphQLOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphQLOutputFormat>;

pub type ValidatedGraphqlClientField = ValidatedClientField<GraphQLOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphQLOutputFormat>;
