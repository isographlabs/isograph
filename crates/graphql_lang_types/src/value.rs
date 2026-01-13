use std::fmt;

use common_lang_types::{
    EmbeddedLocation, EnumLiteralValue, StringLiteralValue, ValueKeyName, VariableName,
    WithEmbeddedLocation, WithGenericLocation, WithSpan,
};

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLConstantValue {
    Int(i64),
    Float(FloatValue),
    String(StringLiteralValue),
    Boolean(bool),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithEmbeddedLocation<GraphQLConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, GraphQLConstantValue>>),
}

impl GraphQLConstantValue {
    pub fn as_string(&self) -> Option<StringLiteralValue> {
        match self {
            GraphQLConstantValue::String(s) => Some(*s),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub enum GraphQLNonConstantValue {
    Variable(VariableName),
    Int(i64),
    Float(FloatValue),
    String(StringLiteralValue),
    Boolean(bool),
    Null,
    Enum(EnumLiteralValue),
    // This is weird! We can be more consistent vis-a-vis where the WithSpan appears.
    List(Vec<WithSpan<GraphQLNonConstantValue>>),
    Object(Vec<NameValuePair<ValueKeyName, GraphQLNonConstantValue>>),
}

impl fmt::Display for GraphQLConstantValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            GraphQLConstantValue::Int(value) => f.write_fmt(format_args!("{value}")),
            GraphQLConstantValue::Float(value) => f.write_fmt(format_args!("{value}")),
            GraphQLConstantValue::String(value) => f.write_fmt(format_args!("\"{value}\"")),
            GraphQLConstantValue::Boolean(value) => f.write_fmt(format_args!("{value}")),
            GraphQLConstantValue::Null => f.write_str("null"),
            GraphQLConstantValue::Enum(value) => f.write_fmt(format_args!("{value}")),
            GraphQLConstantValue::List(value) => f.write_fmt(format_args!(
                "[{}]",
                value
                    .iter()
                    .map(|item| item.item.to_string())
                    .collect::<Vec<String>>()
                    .join(", ")
            )),
            GraphQLConstantValue::Object(value) => f.write_fmt(format_args!(
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

impl fmt::Display for GraphQLNonConstantValue {
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
        #[expect(clippy::cast_precision_loss)]
        FloatValue::new(value as f64)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct NameValuePairInner<TName, TValue, TLocation> {
    pub name: WithGenericLocation<TName, TLocation>,
    pub value: WithGenericLocation<TValue, TLocation>,
}

pub type NameValuePair<TName, TValue> = NameValuePairInner<TName, TValue, EmbeddedLocation>;

impl<TName, TValue, TLocation: Copy> NameValuePairInner<TName, TValue, TLocation> {
    pub fn map_name<U>(
        self,
        map: impl FnOnce(TName) -> U,
    ) -> NameValuePairInner<U, TValue, TLocation> {
        NameValuePairInner {
            name: self.name.map(map),
            value: self.value,
        }
    }
}

impl<TName: fmt::Display, TValue: fmt::Display, TLocation> fmt::Display
    for NameValuePairInner<TName, TValue, TLocation>
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("{}: {}", self.name.item, self.value.item))
    }
}
