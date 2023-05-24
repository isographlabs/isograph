use std::fmt;

use common_lang_types::{
    EnumLiteralValue, StringLiteralValue, ValueKeyName, VariableName, WithSpan,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum ConstantValue {
    Int(i64),
    Float(FloatValue),
    String(StringLiteralValue),
    Boolean(bool),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithSpan<ConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, ConstantValue>>),
}

#[allow(dead_code)]
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum Value {
    Variable(VariableName),
    Int(i64),
    Float(FloatValue),
    String(StringLiteralValue),
    Boolean(bool),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithSpan<Value>>),
    Object(Vec<NameValuePair<ValueKeyName, Value>>),
}

/// ValueType is a trait that is only fulfilled by values and constant
/// values, where the only difference is whether they can contain
/// variables.
pub trait ValueType: fmt::Display {}

impl ValueType for Value {}
impl ValueType for ConstantValue {}

impl fmt::Display for ConstantValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConstantValue::Int(value) => f.write_fmt(format_args!("{}", value)),
            ConstantValue::Float(value) => f.write_fmt(format_args!("{}", value)),
            ConstantValue::String(value) => f.write_fmt(format_args!("\"{}\"", value)),
            ConstantValue::Boolean(value) => f.write_fmt(format_args!("{}", value)),
            ConstantValue::Null => f.write_str("null"),
            ConstantValue::Enum(value) => f.write_fmt(format_args!("{}", value)),
            ConstantValue::List(value) => f.write_fmt(format_args!(
                "[{}]",
                value
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )),
            ConstantValue::Object(value) => f.write_fmt(format_args!(
                "{{{}}}",
                value
                    .iter()
                    .map(|item| item.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, _f: &mut fmt::Formatter<'_>) -> fmt::Result {
        todo!()
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct FloatValue(u64);

impl FloatValue {
    pub fn new(v: f64) -> Self {
        Self(v.to_bits())
    }

    pub fn as_float(self) -> f64 {
        f64::from_bits(self.0)
    }
}

impl From<f64> for FloatValue {
    fn from(value: f64) -> Self {
        FloatValue::new(value)
    }
}

impl fmt::Debug for FloatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.as_float()))
    }
}

impl fmt::Display for FloatValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}", self.as_float()))
    }
}

impl std::convert::From<i64> for FloatValue {
    fn from(value: i64) -> Self {
        FloatValue::new(value as f64)
    }
}

// TODO get rid of this WithSpan and move it to the generic
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NameValuePair<TName, TValue: ValueType> {
    pub name: WithSpan<TName>,
    pub value: WithSpan<TValue>,
}

impl<TName, TValue: ValueType> NameValuePair<TName, TValue> {
    pub fn map_name<U>(self, map: impl FnOnce(TName) -> U) -> NameValuePair<U, TValue> {
        NameValuePair {
            name: self.name.map(|tname| map(tname)),
            value: self.value,
        }
    }
}

impl<TName: fmt::Display, TValue: ValueType> fmt::Display for NameValuePair<TName, TValue> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}: {}", self.name, self.value))
    }
}
