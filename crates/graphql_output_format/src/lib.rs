use isograph_schema::{OutputFormat, Schema, UnvalidatedSchema, ValidatedSchema};

#[derive(Debug)]
pub struct GraphqlOutputFormat {}

impl OutputFormat for GraphqlOutputFormat {}

pub type ValidatedGraphqlSchema = ValidatedSchema<GraphqlOutputFormat>;
pub type GraphqlSchema<TSchemaValidationState> =
    Schema<TSchemaValidationState, GraphqlOutputFormat>;
pub type UnvalidatedGraphqlSchema = UnvalidatedSchema<GraphqlOutputFormat>;
