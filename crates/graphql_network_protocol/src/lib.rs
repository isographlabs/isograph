mod graphql_network_protocol;
mod process_type_system_definition;
mod query_text;
mod read_schema;

pub use graphql_network_protocol::*;
use isograph_schema::{ClientScalarSelectable, Schema, ServerObjectEntity};
pub use read_schema::*;

pub type ValidatedGraphqlSchema = Schema<GraphQLNetworkProtocol>;
pub type GraphqlSchema = Schema<GraphQLNetworkProtocol>;
pub type UnvalidatedGraphqlSchema = Schema<GraphQLNetworkProtocol>;

pub type ValidatedGraphqlClientField = ClientScalarSelectable<GraphQLNetworkProtocol>;

pub type GraphqlSchemaObject = ServerObjectEntity<GraphQLNetworkProtocol>;
