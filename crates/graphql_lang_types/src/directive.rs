use std::fmt;

use common_lang_types::{DirectiveArgumentName, DirectiveName, WithEmbeddedLocation};

use super::{write::write_arguments, NameValuePair, ValueType};

// TODO maybe this should be NameAndArguments and a field should be the same thing...?
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLDirective<T: ValueType> {
    pub name: WithEmbeddedLocation<DirectiveName>,
    pub arguments: Vec<NameValuePair<DirectiveArgumentName, T>>,
}

impl<T: ValueType> fmt::Display for GraphQLDirective<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name)?;
        write_arguments(f, &self.arguments)?;
        Ok(())
    }
}
