use std::fmt::Debug;

use common_lang_types::{VariableName, WithLocation};
use graphql_lang_types::GraphQLTypeAnnotation;
use prelude::Postfix;

use crate::ConstantValue;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct VariableDefinition<TValue: Ord + Debug> {
    pub name: WithLocation<VariableName>,
    pub type_: GraphQLTypeAnnotation<TValue>,
    pub default_value: Option<WithLocation<ConstantValue>>,
}

impl<TValue: Ord + Debug> VariableDefinition<TValue> {
    pub fn map<TNewValue: Ord + Debug>(
        self,
        map: &mut impl FnMut(TValue) -> TNewValue,
    ) -> VariableDefinition<TNewValue> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.map(map),
            default_value: self.default_value,
        }
    }

    pub fn and_then<TNewValue: Ord + Debug, E>(
        self,
        map: &mut impl FnMut(TValue) -> Result<TNewValue, E>,
    ) -> Result<VariableDefinition<TNewValue>, E> {
        VariableDefinition {
            name: self.name,
            type_: self.type_.and_then(map)?,
            default_value: self.default_value,
        }
        .ok()
    }
}
