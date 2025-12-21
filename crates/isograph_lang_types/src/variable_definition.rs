use std::fmt::Debug;

use common_lang_types::WithEmbeddedLocation;
use graphql_lang_types::GraphQLTypeAnnotation;

use crate::{ConstantValue, VariableNameWrapper};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct VariableDefinition {
    pub name: WithEmbeddedLocation<VariableNameWrapper>,
    pub type_: WithEmbeddedLocation<GraphQLTypeAnnotation>,
    pub default_value: Option<WithEmbeddedLocation<ConstantValue>>,
}
