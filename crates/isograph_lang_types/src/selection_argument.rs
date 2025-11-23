use common_lang_types::{
    EnumLiteralValue, FieldArgumentName, StringLiteralValue, ValueKeyName, VariableName,
    WithLocation, WithSpan,
};
use graphql_lang_types::{FloatValue, NameValuePair};
use intern::string_key::Lookup;
use prelude::Postfix;

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct SelectionFieldArgument {
    pub name: WithSpan<FieldArgumentName>,
    pub value: WithLocation<NonConstantValue>,
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
pub enum NonConstantValue {
    Variable(VariableName),
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithLocation<NonConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, NonConstantValue>>),
}

impl NonConstantValue {
    pub fn to_alias_str_chunk(&self) -> String {
        match self {
            NonConstantValue::Variable(name) => format!("v_{name}"),
            // l for literal, i.e. this is shared with others
            NonConstantValue::Integer(int_value) => format!("l_{int_value}"),
            NonConstantValue::Boolean(bool) => format!("l_{bool}"),
            NonConstantValue::String(string) => format!(
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
            NonConstantValue::Float(f) => format!("l_{}", f.as_float()),
            NonConstantValue::Null => "l_null".to_string(),
            NonConstantValue::Enum(e) => format!("e_{e}"),
            NonConstantValue::List(_) => panic!("Lists are not supported here"),
            NonConstantValue::Object(object) => {
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

    pub fn variables(&self) -> Vec<VariableName> {
        // TODO return impl Iterator
        match self {
            NonConstantValue::Variable(variable_name) => vec![*variable_name],
            NonConstantValue::List(items) => {
                let mut variables = vec![];
                for item in items {
                    variables.extend(item.item.variables());
                }
                variables
            }
            NonConstantValue::Object(name_value_pairs) => {
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

impl From<ConstantValue> for NonConstantValue {
    fn from(value: ConstantValue) -> Self {
        match value {
            ConstantValue::Integer(i) => NonConstantValue::Integer(i),
            ConstantValue::Boolean(value) => NonConstantValue::Boolean(value),
            ConstantValue::String(value) => NonConstantValue::String(value),
            ConstantValue::Float(value) => NonConstantValue::Float(value),
            ConstantValue::Null => NonConstantValue::Null,
            ConstantValue::Enum(value) => NonConstantValue::Enum(value),
            ConstantValue::List(value) => NonConstantValue::List(
                value
                    .into_iter()
                    .map(|with_location| with_location.map(NonConstantValue::from))
                    .collect(),
            ),
            ConstantValue::Object(value) => NonConstantValue::Object(
                value
                    .into_iter()
                    .map(|name_value_pair| NameValuePair {
                        name: name_value_pair.name,
                        value: name_value_pair.value.map(NonConstantValue::from),
                    })
                    .collect(),
            ),
        }
    }
}

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub enum ConstantValue {
    Integer(i64),
    Boolean(bool),
    String(StringLiteralValue),
    Float(FloatValue),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithLocation<ConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, ConstantValue>>),
}

impl TryFrom<NonConstantValue> for ConstantValue {
    type Error = VariableName;

    fn try_from(value: NonConstantValue) -> Result<Self, Self::Error> {
        match value {
            NonConstantValue::Variable(variable_name) => variable_name.wrap_err(),
            NonConstantValue::Integer(i) => ConstantValue::Integer(i).wrap_ok(),
            NonConstantValue::Boolean(b) => ConstantValue::Boolean(b).wrap_ok(),
            NonConstantValue::String(s) => ConstantValue::String(s).wrap_ok(),
            NonConstantValue::Float(f) => ConstantValue::Float(f).wrap_ok(),
            NonConstantValue::Null => ConstantValue::Null.wrap_ok(),
            NonConstantValue::Enum(e) => ConstantValue::Enum(e).wrap_ok(),
            NonConstantValue::List(l) => {
                let converted_list = l
                    .into_iter()
                    .map(|x| {
                        WithLocation::new(x.item.try_into()?, x.location).wrap_ok::<Self::Error>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ConstantValue::List(converted_list).wrap_ok()
            }
            NonConstantValue::Object(o) => {
                let converted_object = o
                    .into_iter()
                    .map(|name_value_pair| {
                        NameValuePair {
                            name: name_value_pair.name,
                            value: WithLocation::new(
                                name_value_pair.value.item.try_into()?,
                                name_value_pair.value.location,
                            ),
                        }
                        .wrap_ok::<Self::Error>()
                    })
                    .collect::<Result<Vec<_>, _>>()?;
                ConstantValue::Object(converted_object).wrap_ok()
            }
        }
    }
}

impl ConstantValue {
    pub fn print_to_string(&self) -> String {
        match self {
            ConstantValue::Integer(i) => i.to_string(),
            ConstantValue::Boolean(b) => b.to_string(),
            ConstantValue::String(s) => format!("\"{s}\""),
            ConstantValue::Float(f) => f.as_float().to_string(),
            ConstantValue::Null => "null".to_string(),
            ConstantValue::Enum(e) => e.to_string(),
            ConstantValue::List(l) => {
                let inner = l
                    .iter()
                    .map(|value| value.item.print_to_string())
                    .collect::<Vec<_>>()
                    .join(", ");
                format!("[{inner}]")
            }
            ConstantValue::Object(o) => {
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
