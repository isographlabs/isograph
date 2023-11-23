use std::fmt;

use common_lang_types::{DirectiveArgumentName, DirectiveName, WithSpan};

use super::{write::write_arguments, NameValuePair, ValueType};

// TODO maybe this should be NameAndArguments and a field should be the same thing...?
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct Directive<T: ValueType> {
    pub name: WithSpan<DirectiveName>,
    pub arguments: Vec<NameValuePair<DirectiveArgumentName, T>>,
}

impl<T: ValueType> fmt::Display for Directive<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name)?;
        write_arguments(f, &self.arguments)?;
        Ok(())
    }
}
