use std::collections::HashSet;

use super::NameValuePair;
use crate::GraphQLConstantValue;
use common_lang_types::{
    DeserializationError, Diagnostic, DirectiveArgumentName, DirectiveName, Location,
    WithEmbeddedLocation,
};
use intern::Lookup;
use prelude::Postfix;
use serde::{
    Deserialize, Deserializer,
    de::{self, IntoDeserializer, MapAccess, SeqAccess, value::SeqDeserializer},
};

// TODO maybe this should be NameAndArguments and a field should be the same thing...?
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd, Hash)]
pub struct GraphQLDirective<T> {
    pub name: WithEmbeddedLocation<DirectiveName>,
    pub arguments: Vec<NameValuePair<DirectiveArgumentName, T>>,
}

pub fn from_graphql_directives<'a, T: Deserialize<'a>>(
    directives: &'a [GraphQLDirective<GraphQLConstantValue>],
) -> Result<T, Diagnostic> {
    T::deserialize(GraphQLDirectivesDeserializer::new(directives)).map_err(|e| {
        Diagnostic::new(e.to_string(), Location::Generated.wrap_some())
            .note_todo("Use the directives location, then the specific location")
    })
}

#[derive(Debug)]
struct GraphQLDirectivesDeserializer<'a> {
    directives: &'a [GraphQLDirective<GraphQLConstantValue>],
    directive_names: Vec<DirectiveName>,
    current_index: usize,
}

impl<'a> GraphQLDirectivesDeserializer<'a> {
    fn new(directives: &'a [GraphQLDirective<GraphQLConstantValue>]) -> Self {
        let mut seen = HashSet::new();

        let directive_names = directives
            .iter()
            .map(|d| d.name.item)
            .filter(|name| seen.insert(*name))
            .collect();

        Self {
            directives,
            directive_names,
            current_index: 0,
        }
    }
}

impl<'de> Deserializer<'de> for GraphQLDirectivesDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_map(self)
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf option unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> MapAccess<'de> for GraphQLDirectivesDeserializer<'de> {
    type Error = DeserializationError;

    fn next_key_seed<K>(&mut self, seed: K) -> Result<Option<K::Value>, Self::Error>
    where
        K: de::DeserializeSeed<'de>,
    {
        if let Some(name) = self.directive_names.get(self.current_index) {
            let name_str = name.lookup();
            seed.deserialize(name_str.into_deserializer()).map(Some)
        } else {
            Ok(None)
        }
    }

    fn next_value_seed<V>(&mut self, seed: V) -> Result<V::Value, Self::Error>
    where
        V: de::DeserializeSeed<'de>,
    {
        let name = self.directive_names[self.current_index];
        self.current_index += 1;

        let matching_directives = self
            .directives
            .iter()
            .filter(|d| d.name.item == name)
            .collect();

        seed.deserialize(DirectiveValueDeserializer {
            directives: matching_directives,
            current_index: 0,
        })
    }
}

struct DirectiveValueDeserializer<'a> {
    directives: Vec<&'a GraphQLDirective<GraphQLConstantValue>>,
    current_index: usize,
}

impl<'de> Deserializer<'de> for DirectiveValueDeserializer<'de> {
    type Error = DeserializationError;

    fn deserialize_any<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        visitor.visit_seq(self)
    }

    fn deserialize_option<V>(self, visitor: V) -> Result<V::Value, Self::Error>
    where
        V: de::Visitor<'de>,
    {
        match self.directives.len() {
            0 => visitor.visit_none(),
            1 => visitor.visit_some(GraphQLDirectiveDeserializer {
                directive: self.directives[0],
            }),
            _ => Err(DeserializationError::Custom(format!(
                "Expected at most one @{} directive, but found {}",
                self.directives[0].name.item.lookup(),
                self.directives.len()
            ))),
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum identifier ignored_any
    }
}

impl<'de> SeqAccess<'de> for DirectiveValueDeserializer<'de> {
    type Error = DeserializationError;

    fn next_element_seed<T>(&mut self, seed: T) -> Result<Option<T::Value>, Self::Error>
    where
        T: de::DeserializeSeed<'de>,
    {
        if let Some(&directive) = self.directives.get(self.current_index) {
            self.current_index += 1;
            seed.deserialize(GraphQLDirectiveDeserializer { directive })
                .map(Some)
        } else {
            Ok(None)
        }
    }
}

#[derive(Debug)]
struct GraphQLDirectiveDeserializer<'a> {
    directive: &'a GraphQLDirective<GraphQLConstantValue>,
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
                seed.deserialize(NameValuePairDeserializer { name_value_pair })
            }
            _ => Err(DeserializationError::Custom(format!(
                "Called deserialization of field value for a field with idx \
                {} that doesn't exist. This is indicative of a bug in Isograph.",
                self.field_idx
            ))),
        }
    }
}

struct NameDeserializer<'a, TName, TValue> {
    name_value_pair: &'a NameValuePair<TName, TValue>,
}

struct NameValuePairDeserializer<'a, TName, TValue> {
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
        if let GraphQLConstantValue::Null = self.value {
            visitor.visit_none()
        } else {
            visitor.visit_some(self)
        }
    }

    serde::forward_to_deserialize_any! {
        bool i8 i16 i32 i64 i128 u8 u16 u32 u64 u128 f32 f64 char str string
        bytes byte_buf unit unit_struct newtype_struct seq tuple
        tuple_struct map struct enum ignored_any identifier
    }
}

impl<'de, TName> Deserializer<'de> for NameValuePairDeserializer<'de, TName, GraphQLConstantValue> {
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
