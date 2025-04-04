mod graphql_network_format;
mod process_type_system_definition;
mod query_text;
mod read_schema;

pub use graphql_network_format::*;
use isograph_schema::{ClientField, Schema, ServerObjectEntity};
pub use read_schema::*;

pub type ValidatedGraphqlSchema = Schema<GraphQLOutputFormat>;
pub type GraphqlSchema = Schema<GraphQLOutputFormat>;
pub type UnvalidatedGraphqlSchema = Schema<GraphQLOutputFormat>;

pub type ValidatedGraphqlClientField = ClientField<GraphQLOutputFormat>;

pub type GraphqlSchemaObject = ServerObjectEntity<GraphQLOutputFormat>;
