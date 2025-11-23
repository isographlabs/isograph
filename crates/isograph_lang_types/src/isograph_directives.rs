use common_lang_types::{IsographDirectiveName, WithLocation, WithSpan};
use intern::Lookup;
use prelude::Postfix;
use serde::{
    Deserialize, Deserializer,
    de::{self, IntoDeserializer, MapAccess},
};
use thiserror::Error;

use crate::{NonConstantValue, SelectionFieldArgument};

#[derive(Clone, Eq, PartialEq, Ord, PartialOrd, Debug, Hash)]
pub struct IsographFieldDirective {
    pub name: WithSpan<IsographDirectiveName>,
    pub arguments: Vec<WithLocation<SelectionFieldArgument>>,
}

pub fn from_isograph_field_directives<'a, T: Deserialize<'a>>(
    directives: &'a [WithSpan<IsographFieldDirective>],
) -> Result<T, DeserializationError> {
    T::deserialize(IsographFieldDirectivesDeserializer { directives })
}

#[derive(Debug)]
struct IsographFieldDirectiveDeserializer<'a> {
    directive: &'a IsographFieldDirective,
}

#[derive(Debug, Error, Eq, PartialEq, Clone, PartialOrd, Ord)]
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

impl<'de> Deserializer<'de> for IsographFieldDirectiveDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(NameValuePairVecDeserializer::new(&self.directive.arguments))
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_some(self)
    }
}

struct NameValuePairVecDeserializer<'a> {
    arguments: &'a Vec<WithLocation<SelectionFieldArgument>>,
    field_idx: usize,
}

impl<'a> NameValuePairVecDeserializer<'a> {
    fn new(args: &'a Vec<WithLocation<SelectionFieldArgument>>) -> Self {
        NameValuePairVecDeserializer {
            arguments: args,
            field_idx: 0,
        }
    }
}

impl<'de> MapAccess<'de> for NameValuePairVecDeserializer<'de> {
    type Error = DeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let Some(name_value_pair) = self.arguments.get(self.field_idx) {
            return seed
                .deserialize(NameDeserializer {
                    name: name_value_pair.item.name.item.lookup(),
                })
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
                seed.deserialize(NonConstantValueDeserializer {
                    value: &name_value_pair.item.value.item,
                })
            }
            _ => Err(DeserializationError::Custom(format!(
                "Called deserialization of field value for a field with idx {} \
                that doesn't exist. This is indicative of a bug in Isograph.",
                self.field_idx
            ))),
        }
    }
}

struct NameDeserializer {
    name: &'static str,
}

impl<'de> Deserializer<'de> for NameDeserializer {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_borrowed_str(self.name)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

pub struct NonConstantValueDeserializer<'de> {
    value: &'de NonConstantValue,
}

impl<'de> IntoDeserializer<'de, DeserializationError> for &'de NonConstantValue {
    type Deserializer = NonConstantValueDeserializer<'de>;

    fn into_deserializer(self) -> Self::Deserializer {
        NonConstantValueDeserializer { value: self }
    }
}

impl<'de> Deserializer<'de> for NonConstantValueDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.value {
            NonConstantValue::Variable(_variable) => todo!("Variable?"),
            NonConstantValue::Integer(i_64) => visitor.visit_i64(*i_64),
            NonConstantValue::Boolean(bool) => visitor.visit_bool(*bool),
            NonConstantValue::String(s) => visitor.visit_str(s.lookup()),
            NonConstantValue::Float(f) => visitor.visit_f64(f.as_float()),
            NonConstantValue::Null => visitor.visit_none(),
            NonConstantValue::Enum(_) => panic!("Enums not supported when deserializing"),
            NonConstantValue::List(_) => {
                panic!("Deserializing from lists is not yet supported here.")
            }
            NonConstantValue::Object(_) => panic!("Deserializing objects not yet supported here."),
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

#[derive(Debug)]
struct IsographFieldDirectivesDeserializer<'a> {
    directives: &'a [WithSpan<IsographFieldDirective>],
}

impl<'de> Deserializer<'de> for IsographFieldDirectivesDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(IsographFieldDirectiveVecDeserializer::new(self.directives))
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        if self.directives.is_empty() {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

struct IsographFieldDirectiveVecDeserializer<'a> {
    directives: &'a [WithSpan<IsographFieldDirective>],
    field_idx: usize,
}

impl<'a> IsographFieldDirectiveVecDeserializer<'a> {
    pub fn new(directives: &'a [WithSpan<IsographFieldDirective>]) -> Self {
        Self {
            field_idx: 0,
            directives,
        }
    }
}

impl<'de> MapAccess<'de> for IsographFieldDirectiveVecDeserializer<'de> {
    type Error = DeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let Some(directive) = self.directives.get(self.field_idx) {
            return seed
                .deserialize(NameDeserializer {
                    name: directive.item.name.item.lookup(),
                })
                .map(Some);
        }
        Ok(None)
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        match self.directives.get(self.field_idx) {
            Some(directive) => {
                self.field_idx += 1;
                seed.deserialize(IsographFieldDirectiveDeserializer {
                    directive: &directive.item,
                })
            }
            _ => DeserializationError::Custom(format!(
                "Called deserialization of field value for a field with idx {} \
                that doesn't exist. This is indicative of a bug in Isograph.",
                self.field_idx
            ))
            .err(),
        }
    }
}
