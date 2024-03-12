use std::fmt;

use super::{write::write_arguments, NameValuePair, ValueType};
use crate::ConstantValue;
use common_lang_types::{DirectiveArgumentName, DirectiveName, WithEmbeddedLocation, WithLocation};
use serde::de;
use serde::de::value::SeqDeserializer;
use serde::de::IntoDeserializer;
use serde::de::MapAccess;
use serde::Deserialize;
use serde::Deserializer;
use thiserror::Error;

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

pub fn from_graph_ql_directive<'a, T: Deserialize<'a>>(
    directive: &'a GraphQLDirective<ConstantValue>,
) -> Result<T, DeserializationError> {
    T::deserialize(GraphQLDirectiveDeserializer { directive })
}

#[derive(Debug)]
struct GraphQLDirectiveDeserializer<'a> {
    directive: &'a GraphQLDirective<ConstantValue>,
}

#[derive(Debug, Error)]
pub enum DeserializationError {
    #[error("Error when deserializing {0} ")]
    Custom(String),
}

impl de::Error for DeserializationError {
    fn custom<T>(msg: T) -> Self
    where
        T: core::fmt::Display,
    {
        DeserializationError::Custom(msg.to_string())
    }
}

impl<'de> Deserializer<'de> for GraphQLDirectiveDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(NameValuePairVecDeserializer::new(&self.directive.arguments))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct NameValuePairVecDeserializer<'a, T> {
    arguments: &'a Vec<NameValuePair<T, ConstantValue>>,
    field_idx: usize,
}

impl<'a, T> NameValuePairVecDeserializer<'a, T> {
    fn new(args: &'a Vec<NameValuePair<T, ConstantValue>>) -> Self {
        NameValuePairVecDeserializer {
            arguments: args,
            field_idx: 0,
        }
    }
}

impl<'de, T: ToString> MapAccess<'de> for NameValuePairVecDeserializer<'de, T> {
    type Error = DeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let Some(name_value_pair) = self.arguments.get(self.field_idx) {
            return seed
                .deserialize(NameSerializer { name_value_pair })
                .map(Some);
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.arguments.get(self.field_idx) {
            Some(name_value_pair) => {
                self.field_idx += 1;
                seed.deserialize(ValueSerializer { name_value_pair })
            }
            _ => Err(DeserializationError::Custom(format!(
                "Called deserialization of field value for a field with idx {} that doesn't exist",
                self.field_idx
            ))),
        }
    }
}

struct NameSerializer<'a, TName, TValue: ValueType> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

struct ValueSerializer<'a, TName, TValue: ValueType> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

impl<'de, TName: ToString, TValue: ValueType> Deserializer<'de>
    for NameSerializer<'de, TName, TValue>
{
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.name_value_pair.name.item.to_string())
    }

    fn deserialize_identifier<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_string(self.name_value_pair.name.item.to_string())
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any
    }
}

pub struct ConstantValueDeserializer<'de> {
    value: &'de ConstantValue,
}

impl<'de> IntoDeserializer<'de, DeserializationError> for &'de ConstantValue {
    type Deserializer = ConstantValueDeserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        ConstantValueDeserializer { value: self }
    }
}

impl<'de> Deserializer<'de> for ConstantValueDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            ConstantValue::Boolean(bool) => visitor.visit_bool(*bool),
            ConstantValue::Enum(enum_literal) => visitor.visit_string(enum_literal.to_string()),
            ConstantValue::Float(float_value) => visitor.visit_f64(float_value.as_float()),
            ConstantValue::Int(i_64) => visitor.visit_i64(*i_64),
            ConstantValue::String(string) => visitor.visit_string(string.to_string()),
            ConstantValue::Null => visitor.visit_none(),
            ConstantValue::List(seq) => {
                let values: Vec<&ConstantValue> = seq.iter().map(|entry| &entry.item).collect();
                let seq_access = SeqDeserializer::new(values.into_iter());
                visitor.visit_seq(seq_access)
            }
            ConstantValue::Object(obj) => {
                let serializer = NameValuePairVecDeserializer::new(&obj);
                visitor.visit_map(serializer)
            }
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

impl<'de, TName> Deserializer<'de> for ValueSerializer<'de, TName, ConstantValue> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let deserializer = ConstantValueDeserializer {
            value: &self.name_value_pair.value.item,
        };
        deserializer.deserialize_any(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}
