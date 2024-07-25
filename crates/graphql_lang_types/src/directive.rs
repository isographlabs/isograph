use std::fmt;

use super::{write::write_arguments, NameValuePair};
use crate::GraphQLConstantValue;
use common_lang_types::{DirectiveArgumentName, DirectiveName, WithEmbeddedLocation};
use intern::Lookup;
use serde::{
    de::{self, value::SeqDeserializer, IntoDeserializer, MapAccess},
    Deserialize, Deserializer,
};
use thiserror::Error;

// TODO maybe this should be NameAndArguments and a field should be the same thing...?
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLDirective<T> {
    pub name: WithEmbeddedLocation<DirectiveName>,
    pub arguments: Vec<NameValuePair<DirectiveArgumentName, T>>,
}

impl<T: fmt::Display> fmt::Display for GraphQLDirective<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "@{}", self.name)?;
        write_arguments(f, &self.arguments)?;
        Ok(())
    }
}

pub fn from_graph_ql_directive<'a, T: Deserialize<'a>>(
    directive: &'a GraphQLDirective<GraphQLConstantValue>,
) -> Result<T, DeserializationError> {
    T::deserialize(GraphQLDirectiveDeserializer { directive })
}

#[derive(Debug)]
struct GraphQLDirectiveDeserializer<'a> {
    directive: &'a GraphQLDirective<GraphQLConstantValue>,
}

#[derive(Debug, Error)]
pub enum DeserializationError {
    #[error("Error when deserializing.\n\n{0}")]
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
    arguments: &'a Vec<NameValuePair<T, GraphQLConstantValue>>,
    field_idx: usize,
}

impl<'a, T> NameValuePairVecDeserializer<'a, T> {
    fn new(args: &'a Vec<NameValuePair<T, GraphQLConstantValue>>) -> Self {
        NameValuePairVecDeserializer {
            arguments: args,
            field_idx: 0,
        }
    }
}

impl<'de, T: Lookup + Copy> MapAccess<'de> for NameValuePairVecDeserializer<'de, T> {
    type Error = DeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let Some(name_value_pair) = self.arguments.get(self.field_idx) {
            return seed
                .deserialize(NameDeserializer { name_value_pair })
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
                seed.deserialize(ValueDeserializer { name_value_pair })
            }
            _ => Err(DeserializationError::Custom(format!(
                "Called deserialization of field value for a field with idx {} that doesn't exist. This is indicative of a bug in Isograph.",
                self.field_idx
            ))),
        }
    }
}

struct NameDeserializer<'a, TName, TValue> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

struct ValueDeserializer<'a, TName, TValue> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

impl<'de, TName: Lookup + Copy, TValue> Deserializer<'de> for NameDeserializer<'de, TName, TValue> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.name_value_pair.name.item.lookup())
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

pub struct ConstantValueDeserializer<'de> {
    value: &'de GraphQLConstantValue,
}

impl<'de> IntoDeserializer<'de, DeserializationError> for &'de GraphQLConstantValue {
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
            GraphQLConstantValue::Boolean(bool) => visitor.visit_bool(*bool),
            GraphQLConstantValue::Enum(enum_literal) => {
                visitor.visit_borrowed_str(enum_literal.lookup())
            }
            GraphQLConstantValue::Float(float_value) => visitor.visit_f64(float_value.as_float()),
            GraphQLConstantValue::Int(i_64) => visitor.visit_i64(*i_64),
            GraphQLConstantValue::String(string) => visitor.visit_borrowed_str(string.lookup()),
            GraphQLConstantValue::Null => visitor.visit_none(),
            GraphQLConstantValue::List(seq) => {
                let seq_access = SeqDeserializer::new(seq.iter().map(|entry| &entry.item));
                visitor.visit_seq(seq_access)
            }
            GraphQLConstantValue::Object(obj) => {
                let serializer = NameValuePairVecDeserializer::new(obj);
                visitor.visit_map(serializer)
            }
        }
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

impl<'de, TName> Deserializer<'de> for ValueDeserializer<'de, TName, GraphQLConstantValue> {
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

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        let deserializer = ConstantValueDeserializer {
            value: &self.name_value_pair.value.item,
        };
        deserializer.deserialize_option(visitor)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}
