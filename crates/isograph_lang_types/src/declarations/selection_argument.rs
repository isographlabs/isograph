use common_lang_types::{
    EmbeddedLocation, EnumLiteralValue, FieldArgumentName, StringLiteralValue, ValueKeyName,
    WithEmbeddedLocation, WithGenericLocation, WithLocationPostfix,
};
use graphql_lang_types::{FloatValue, NameValuePair};
use intern::string_key::Lookup;
use prelude::Postfix;

use crate::VariableNameWrapper;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct SelectionFieldArgument {
    pub name: WithEmbeddedLocation<FieldArgumentName>,
    pub value: WithEmbeddedLocation<NonConstantValue>,
}

impl SelectionFieldArgument {
    /// A function called on each SelectionFieldArgument when
    /// generating queries. This must be kept in sync with @isograph/react
    pub fn to_alias_str_chunk(&self) -> String {
        format!(
            "{}___{}",
            self.name.item,
            self.value.item.to_alias_str_chunk()
        )
    }

    pub fn into_key_and_value(&self) -> ArgumentKeyAndValue {
        ArgumentKeyAndValue {
            key: self.name.item,
            value: self.value.item.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ArgumentKeyAndValue {
    pub key: FieldArgumentName,
    pub value: NonConstantValue,
}

impl ArgumentKeyAndValue {
    pub fn to_alias_str_chunk(&self) -> String {
        format!("{}___{}", self.key, self.value.to_alias_str_chunk())
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum NonConstantValueInner<TLocation> {
    Variable(VariableNameWrapper),
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    List(Vec<WithGenericLocation<NonConstantValueInner<TLocation>, TLocation>>),
    Object(Vec<NameValuePair<ValueKeyName, NonConstantValueInner<TLocation>>>),
}

pub type NonConstantValue = NonConstantValueInner<EmbeddedLocation>;

impl<TLocation> NonConstantValueInner<TLocation> {
    pub fn to_alias_str_chunk(&self) -> String {
        match self {
            NonConstantValueInner::Variable(name) => format!("v_{name}"),
            // l for literal, i.e. this is shared with others
            NonConstantValueInner::Integer(int_value) => format!("l_{int_value}"),
            NonConstantValueInner::Boolean(bool) => format!("l_{bool}"),
            NonConstantValueInner::String(string) => format!(
                "s_{}",
                string
                    .lookup()
                    .chars()
                    .map(|c| match c {
                        'A'..='Z' | 'a'..='z' | '0'..='9' | '_' => c,
                        // N.B. This clearly isn't correct, the string can (for example) include
                        // spaces, which would break things.
                        // TODO get a solution or validate
                        _ => '_',
                    })
                    .collect::<String>(),
            ),
            // Also not correct
            NonConstantValueInner::Float(f) => format!("l_{}", f.as_float()),
            NonConstantValueInner::Null => "l_null".to_string(),
            NonConstantValueInner::Enum(e) => format!("e_{e}"),
            NonConstantValueInner::List(_) => panic!("Lists are not supported here"),
            NonConstantValueInner::Object(object) => {
                format!(
                    "o_{}_c",
                    object
                        .iter()
                        .map(|pair| format!(
                            "{}__{}",
                            pair.name.item,
                            pair.value.item.to_alias_str_chunk()
                        ))
                        .collect::<Vec<_>>()
                        .join("_")
                )
            }
        }
    }

    pub fn variables(&self) -> Vec<VariableNameWrapper> {
        // TODO return impl Iterator
        match self {
            NonConstantValueInner::Variable(variable_name) => vec![*variable_name],
            NonConstantValueInner::List(items) => {
                let mut variables = vec![];
                for item in items {
                    variables.extend(item.item.variables());
                }
                variables
            }
            NonConstantValueInner::Object(name_value_pairs) => {
                let mut variables = vec![];
                for item in name_value_pairs {
                    variables.extend(item.value.item.variables());
                }
                variables
            }
            _ => vec![],
        }
    }
}

impl<TLocation: Copy> From<ConstantValueInner<TLocation>> for NonConstantValueInner<TLocation> {
    fn from(value: ConstantValueInner<TLocation>) -> Self {
        match value {
            ConstantValueInner::Integer(i) => NonConstantValueInner::Integer(i),
            ConstantValueInner::Boolean(value) => NonConstantValueInner::Boolean(value),
            ConstantValueInner::String(value) => NonConstantValueInner::String(value),
            ConstantValueInner::Float(value) => NonConstantValueInner::Float(value),
            ConstantValueInner::Null => NonConstantValueInner::Null,
            ConstantValueInner::Enum(value) => NonConstantValueInner::Enum(value),
            ConstantValueInner::List(value) => NonConstantValueInner::List(
                value
                    .into_iter()
                    .map(|with_location| with_location.map(NonConstantValueInner::from))
                    .collect(),
            ),
            ConstantValueInner::Object(value) => NonConstantValueInner::Object(
                value
                    .into_iter()
                    .map(|name_value_pair| NameValuePair {
                        name: name_value_pair.name,
                        value: name_value_pair.value.map(NonConstantValueInner::from),
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum ConstantValueInner<TLocation> {
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    List(Vec<WithGenericLocation<ConstantValueInner<TLocation>, TLocation>>),
    Object(Vec<NameValuePair<ValueKeyName, ConstantValueInner<TLocation>>>),
}

pub type ConstantValue = ConstantValueInner<EmbeddedLocation>;

impl<TLocation> TryFrom<NonConstantValueInner<TLocation>> for ConstantValueInner<TLocation> {
    type Error = VariableNameWrapper;

    fn try_from(value: NonConstantValueInner<TLocation>) -> Result<Self, Self::Error> {
        match value {
            NonConstantValueInner::Variable(variable_name) => variable_name.wrap_err(),
            NonConstantValueInner::Integer(i) => ConstantValueInner::Integer(i).wrap_ok(),
            NonConstantValueInner::Boolean(b) => ConstantValueInner::Boolean(b).wrap_ok(),
            NonConstantValueInner::String(s) => ConstantValueInner::String(s).wrap_ok(),
            NonConstantValueInner::Float(f) => ConstantValueInner::Float(f).wrap_ok(),
            NonConstantValueInner::Null => ConstantValueInner::Null.wrap_ok(),
            NonConstantValueInner::Enum(e) => ConstantValueInner::Enum(e).wrap_ok(),
            NonConstantValueInner::List(l) => {
                let converted_list = l
                    .into_iter()
                    .map(|x| {
                        let constant: ConstantValueInner<TLocation> = x.item.try_into()?;
                        constant.with_location(x.location).wrap_ok::<Self::Error>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ConstantValueInner::List(converted_list).wrap_ok()
            }
            NonConstantValueInner::Object(o) => {
                let converted_object = o
                    .into_iter()
                    .map(|name_value_pair| {
                        NameValuePair {
                            name: name_value_pair.name,
                            value: {
                                let constant: ConstantValueInner<TLocation> =
                                    name_value_pair.value.item.try_into()?;
                                constant.with_location(name_value_pair.value.location)
                            },
                        }
                        .wrap_ok::<Self::Error>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ConstantValueInner::Object(converted_object).wrap_ok()
            }
        }
    }
}

impl<TLocation> ConstantValueInner<TLocation> {
    pub fn print_to_string(&self) -> String {
        match self {
            ConstantValueInner::Integer(i) => i.to_string(),
            ConstantValueInner::Boolean(b) => b.to_string(),
            ConstantValueInner::String(s) => format!("\"{s}\""),
            ConstantValueInner::Float(f) => f.as_float().to_string(),
            ConstantValueInner::Null => "null".to_string(),
            ConstantValueInner::Enum(e) => e.to_string(),
            ConstantValueInner::List(l) => {
                let inner = l
                    .iter()
                    .map(|value| value.item.print_to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            ConstantValueInner::Object(o) => {
                let inner = o
                    .iter()
                    .map(|key_value| {
                        format!(
                            "{}: {}",
                            key_value.name.item,
                            key_value.value.item.print_to_string()
                        )
                    })
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("{{{inner}}}")
            }
        }
    }
}
