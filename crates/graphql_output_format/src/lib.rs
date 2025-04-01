mod graphql_output_format;
mod process_type_system_definition;
mod query_text;
mod read_schema;

pub use graphql_output_format::*;
use isograph_schema::{ClientField, Schema, SchemaObject, UnvalidatedSchema, ValidatedSchema};
pub use read_schema::*;

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphQLOutputFormat>;
pub type GraphqlSchema = Schema<GraphQLOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphQLOutputFormat>;

pub type ValidatedGraphqlClientField = ClientField<GraphQLOutputFormat>;

pub type GraphqlSchemaObject = SchemaObject<GraphQLOutputFormat>;
