use std::fmt::Debug;

use common_lang_types::WithEmbeddedLocation;

use crate::{ConstantValue, TypeAnnotation, VariableNameWrapper};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct VariableDefinition {
    pub name: WithEmbeddedLocation<VariableNameWrapper>,
    pub type_: WithEmbeddedLocation<TypeAnnotation>,
    pub default_value: Option<WithEmbeddedLocation<ConstantValue>>,
}
